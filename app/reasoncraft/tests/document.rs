//! UI tests for the backstage Document page and the document language.

use eframe::egui;
use egui_kittest::{Harness, kittest::Queryable};
use reasoncraft::app::RibbonTab;
use reasoncraft::doc_tab::DocTab;
use reasoncraft::i18n::{FluentArgs, t, t_args};
use reasoncraft::settings::TextTab;
use reasoncraft::{ReasonCraftApp, actions::Action};
use reasoncraft_core::{BoxKind, CardKind, Document, Parent, SourceType};

type H = Harness<'static, ReasonCraftApp>;

/// "Robots of mensen" as the claim, with a sourced reason, so the only
/// possible issue is the compound statement.
fn harness() -> H {
    let mut d = Document::new("Robots of mensen");
    let (_, p) = d
        .add_card(
            Parent::Box(d.root),
            CardKind::Reason,
            "Robots mensen helpen",
        )
        .unwrap();
    let source = BoxKind::Source {
        source_type: SourceType::Web,
    };
    d.add_box(Parent::Box(p), source, "x.example").unwrap();
    let mut app = ReasonCraftApp::new_for_test();
    app.tabs[0] = DocTab::from_document(d);
    app.settings.text_tab = TextTab::Check;
    Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .with_max_steps(32)
        .build_ui_state(|ui, app: &mut ReasonCraftApp| app.show(ui), app)
}

fn chip(count: usize) -> String {
    let mut args = FluentArgs::new();
    args.set("count", count);
    t_args("status.issues", &args)
}

fn has(h: &H, label: &str) -> bool {
    h.query_by_label(label).is_some()
}

fn open_document_page(h: &mut H) {
    h.state_mut().ribbon_tab = RibbonTab::File;
    h.run();
    h.get_by_label(&t("backstage.document")).click();
    h.run();
}

#[test]
fn switching_language_changes_checks() {
    let mut h = harness();
    h.run();
    // In English "of" is an ordinary word.
    assert!(has(&h, &chip(0)));

    open_document_page(&mut h);
    let dutch = reasoncraft::a11y::in_group(&t("document.language"), &t("lang.nl"));
    h.get_by_label(&dutch).click();
    h.run();
    assert_eq!(h.state().tabs[0].doc.meta.language, "nl");
    h.state_mut().ribbon_tab = RibbonTab::Home;
    h.run();
    assert!(has(&h, &chip(1)), "in Dutch, \"of\" joins two claims");

    // Undo returns to English and the check goes quiet again.
    h.state_mut().push(Action::Undo);
    h.run();
    assert!(has(&h, &chip(0)));
}

#[test]
fn title_and_author_typing_is_one_history_step() {
    let mut h = harness();
    open_document_page(&mut h);
    h.get_by_label(&t("document.title_field")).click();
    h.get_by_label(&t("document.title_field"))
        .type_text("Robots");
    h.run();
    h.get_by_label(&t("document.author_field")).click();
    h.get_by_label(&t("document.author_field")).type_text("Ann");
    h.run();
    let tab = &h.state().tabs[0];
    assert_eq!(tab.doc.meta.title, "Robots");
    assert_eq!(tab.doc.meta.author, "Ann");
    // Moving between the fields stays in one session until another
    // command ends it.
    h.state_mut().push(Action::ZoomFit);
    h.run();
    assert_eq!(h.state().tabs[0].history.undo.len(), 1);
}

#[test]
fn settings_page_sets_default_language() {
    let mut h = harness();
    h.state_mut().ribbon_tab = RibbonTab::File;
    h.run();
    h.get_by_label(&t("backstage.settings")).click();
    h.run();
    let dutch = reasoncraft::a11y::in_group(&t("settings.default_language"), &t("lang.nl"));
    h.get_by_label(&dutch).click();
    h.run();
    assert_eq!(h.state().settings.default_doc_language, "nl");
}
