//! Step 6 end to end, driven by actions (and F1): a Dutch user opens the
//! Dutch example, turns the map English while the interface stays Dutch,
//! saves an edited template into the recent files, resets the settings
//! (keeping them), opens the help with F1 and exports an English SVG;
//! every control on screen has a name.
//!
//! It switches the UI language, so it lives in this test binary and holds
//! the language lock (see `main.rs`).

use eframe::egui::{Key, accesskit};
use egui_kittest::kittest::{AccessKitNode, NodeT, Queryable};
use reasoncraft::ReasonCraftApp;
use reasoncraft::actions::Action;
use reasoncraft::export::{ExportFormat, ExportOptions, ExportRequest, PageSetup};
use reasoncraft::file_ops::FakeDialogs;
use reasoncraft::help::{HelpPage, title};
use reasoncraft::i18n::{t, t_lang, ui_language};
use reasoncraft_core::examples::{Example, Template, example, template};
use reasoncraft_core::{BoxKind, Document, Item};

use super::{lock, painted_texts};
use crate::common::{build, settle};

use crate::common::H;

/// Apply `action` through the frame loop, as a widget would, and let the
/// work it starts (an export, a dictionary) end.
fn act(h: &mut H, action: Action) {
    h.state_mut().push(action);
    h.step();
    settle(h);
}

/// The names of the source types in `doc`, in `lang`, that read
/// differently in English and Dutch.
fn source_names(doc: &Document, lang: &str) -> Vec<String> {
    let mut names: Vec<String> = doc
        .boxes
        .values()
        .filter_map(|b| match &b.kind {
            BoxKind::Source { source_type } => Some(format!("source.{}", source_type.key())),
            _ => None,
        })
        .filter(|key| t_lang("en", key) != t_lang("nl", key))
        .map(|key| t_lang(lang, &key))
        .collect();
    names.sort();
    names.dedup();
    names
}

/// Every one of `shown` is in `texts`, and none of `hidden`.
fn speaks(texts: &[String], shown: &[String], hidden: &[String]) {
    for word in shown {
        assert!(texts.contains(word), "{word:?} in {texts:?}");
    }
    for word in hidden {
        assert!(!texts.contains(word), "{word:?} in {texts:?}");
    }
}

/// The texts the map shows now, after fitting it to the window.
fn map_texts(h: &mut H) -> Vec<String> {
    act(h, Action::ZoomFit);
    painted_texts(h, &t("canvas.label"))
}

/// Every control on screen without an accessible name.
fn unnamed_controls(node: &AccessKitNode, out: &mut Vec<String>) {
    for child in node.children() {
        let data = child.data();
        let control = data.supports_action(accesskit::Action::Click)
            || data.supports_action(accesskit::Action::Focus);
        let named = child
            .label()
            .or_else(|| {
                child
                    .label_comes_from_value()
                    .then(|| child.value())
                    .flatten()
            })
            .is_some_and(|n| !n.trim().is_empty());
        if control && !child.is_hidden() && child.role() != accesskit::Role::ScrollBar && !named {
            out.push(format!("{:?}", child.role()));
        }
        unnamed_controls(&child, out);
    }
}

#[test]
fn release_flow() {
    let _lang = lock();
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDialogs::default();
    let mut app = ReasonCraftApp::new_for_test_with(Box::new(fake.clone()));
    // A Dutch computer: after Reset to defaults the UI follows it.
    app.os_locale = || Some("nl-NL".to_owned());
    let mut h = build(app);
    h.run();

    // 1. The UI in Dutch.
    act(&mut h, Action::SetUiLanguage(Some("nl".to_owned())));
    assert_eq!(ui_language(), "nl");
    h.get_by_label(&t_lang("nl", "ribbon.tab.home"));

    // 2. The reasoning example opens in Dutch, and its map speaks Dutch.
    act(&mut h, Action::OpenExample(Example::Reasoning));
    let doc = h.state().active_tab().doc.clone();
    assert_eq!(doc.meta.language, "nl");
    let want = example(Example::Reasoning, "nl");
    assert_eq!(doc.boxes[&doc.root].text, want.boxes[&want.root].text);
    let (dutch, english) = (source_names(&doc, "nl"), source_names(&doc, "en"));
    assert!(!dutch.is_empty(), "the example has sources");
    speaks(&map_texts(&mut h), &dutch, &english);

    // 3. The document in English: the map turns English, the UI stays Dutch.
    act(&mut h, Action::SetDocLanguage("en".to_owned()));
    assert_eq!(h.state().active_tab().doc.meta.language, "en");
    speaks(&map_texts(&mut h), &english, &dutch);
    assert_eq!(ui_language(), "nl");
    h.get_by_label(&t_lang("nl", "ribbon.tab.home"));
    assert!(h.query_by_label(&t_lang("en", "ribbon.tab.home")).is_none());
    let example_tab = h.state().active;

    // 4. A template (Dutch, like the UI), edited and saved: it heads the
    // recent files.
    act(&mut h, Action::OpenTemplate(Template::BlankReasoning));
    let doc = &h.state().active_tab().doc;
    let want = template(Template::BlankReasoning, "nl");
    assert_eq!(doc.boxes[&doc.root].text, want.boxes[&want.root].text);
    let root = doc.root;
    act(&mut h, Action::Select(Some(Item::Box(root))));
    act(&mut h, Action::AddReason);
    act(&mut h, Action::TypeText("Minder files".to_owned()));
    act(&mut h, Action::StopEditing);
    assert!(h.state().active_tab().is_dirty());
    let saved = dir.path().join("sjabloon.rcraft");
    fake.set_save(saved.clone());
    act(&mut h, Action::Save);
    crate::common::settle(&mut h);
    assert!(h.state().modal.is_none(), "{:?}", h.state().modal);
    assert!(saved.is_file());
    assert!(!h.state().active_tab().is_dirty());
    assert_eq!(h.state().settings.recent_files.first(), Some(&saved));

    // 5. Reset to defaults keeps the recent files; the UI follows the
    // (Dutch) OS again.
    h.state_mut().settings.ui_scale = 1.5;
    act(&mut h, Action::ResetSettings);
    act(&mut h, Action::ConfirmResetSettings);
    let settings = &h.state().settings;
    assert_eq!(settings.ui_scale, 1.0);
    assert_eq!(settings.ui_language, None);
    assert_eq!(settings.recent_files, vec![saved.clone()]);
    assert_eq!(ui_language(), "nl");

    // 6. F1 opens the help, in Dutch.
    act(&mut h, Action::ActivateTab(example_tab));
    h.key_press(Key::F1);
    h.run();
    assert!(h.state().right_panel);
    let HelpPage::Topic(topic) = h.state().help.page else {
        panic!("F1 shows a topic: {:?}", h.state().help.page)
    };
    let shown = |h: &H, lang: &str| {
        h.query_all_by_label(&title(lang, topic))
            .any(|n| n.accesskit_node().role() == accesskit::Role::Label)
    };
    assert!(shown(&h, "nl") && !shown(&h, "en"), "{topic} in Dutch");

    // 7. The SVG speaks the document's language: English.
    let svg = dir.path().join("kaart.svg");
    fake.set_export(svg.clone());
    act(
        &mut h,
        Action::RunExport(ExportRequest {
            format: ExportFormat::Svg,
            options: ExportOptions {
                svg_outlines: false,
                ..ExportOptions::default()
            },
            page: PageSetup::default(),
        }),
    );
    crate::common::step_until(&mut h, |app| !app.export.busy(), "the export");
    assert!(h.state().modal.is_none(), "{:?}", h.state().modal);
    let svg = std::fs::read_to_string(&svg).unwrap();
    for en in &english {
        assert!(svg.contains(&format!(">{en}<")), "{en:?} in the SVG");
    }
    for nl in &dutch {
        assert!(!svg.contains(nl.as_str()), "{nl:?} in the SVG");
    }

    // 8. Every control on screen, the help included, has a name.
    let mut unnamed = Vec::new();
    unnamed_controls(&h.root().accesskit_node(), &mut unnamed);
    assert!(unnamed.is_empty(), "unnamed controls: {unnamed:?}");
}
