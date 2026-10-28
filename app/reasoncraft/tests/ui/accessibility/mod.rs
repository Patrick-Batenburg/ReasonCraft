//! Accessibility: every control has a name in the AccessKit tree, unique
//! among its siblings (and in the window), the keyboard reaches every
//! field (also rows of virtual lists), the UI scale setting and Reset to
//! defaults.

use crate::common::{app_with_doc, harness};
use std::collections::HashMap;

use eframe::egui::{self, Key, Modifiers, accesskit};
use egui_kittest::kittest::{AccessKitNode, NodeT, Queryable};
use reasoncraft::ReasonCraftApp;
use reasoncraft::actions::{Action, apply};
use reasoncraft::app::RibbonTab;
use reasoncraft::doc_tab::DocTab;
use reasoncraft::export::ExportFormat;
use reasoncraft::file_ops::Modal;
use reasoncraft::help::HelpPage;
use reasoncraft::i18n::{FluentArgs, t, t_args};
use reasoncraft::settings::{MinimapMode, SETTINGS_KEY, Settings, TextTab};
use reasoncraft::theme::ThemeChoice;
use reasoncraft_core::argdown::{ImportNote, ImportNoteKind};
use reasoncraft_core::{CardKind, Document, Item, NodeId, Parent};

mod announce;
mod dialogs;
mod keyboard;
mod map;
mod order;
mod settings;

use crate::common::H;

/// Whether `node` is a control a screen reader announces: one that can
/// be clicked or focused, or one greyed out for now. egui's own scroll
/// bars are known by their role.
fn control(node: &AccessKitNode) -> bool {
    let data = node.data();
    let acts = data.supports_action(accesskit::Action::Click)
        || data.supports_action(accesskit::Action::Focus);
    !node.is_hidden() && (acts || node.is_disabled()) && node.role() != accesskit::Role::ScrollBar
}

/// The accessible name of `node`: a label's is its text.
fn name(node: &AccessKitNode) -> String {
    node.label()
        .or_else(|| {
            node.label_comes_from_value()
                .then(|| node.value())
                .flatten()
        })
        .unwrap_or_default()
}

/// Every control without a name, and every set of siblings
/// sharing a role and a name, under `node`.
fn sibling_problems(node: &AccessKitNode, out: &mut Vec<String>) {
    let mut seen: HashMap<(accesskit::Role, String), usize> = HashMap::new();
    for child in node.children() {
        if control(&child) {
            let name = name(&child);
            if name.trim().is_empty() {
                let at = child.bounding_box().map(|r| (r.x0 as i32, r.y0 as i32));
                out.push(format!("unnamed {:?} at {at:?}", child.role()));
            } else {
                *seen.entry((child.role(), name)).or_default() += 1;
            }
        }
        sibling_problems(&child, out);
    }
    for ((role, name), n) in seen {
        if n > 1 {
            out.push(format!("{n} siblings {role:?} {name:?}"));
        }
    }
}

/// Controls shown twice on purpose: the title bar's quick Save, Undo and
/// Redo run the same commands as the backstage's Save and the Home tab's
/// Undo and Redo.
const SAME_COMMAND_TWICE: [&str; 3] = ["file.save", "home.undo", "home.redo"];

/// The problems of the tree shown now: unnamed controls, controls
/// sharing a name with a sibling, and controls (other than text, which
/// can be selected) sharing a role and a name anywhere in the window.
fn problems(h: &H, state: &str) -> Vec<String> {
    let mut out = Vec::new();
    let root = h.root();
    sibling_problems(&root.accesskit_node(), &mut out);
    let allowed: Vec<String> = SAME_COMMAND_TWICE
        .iter()
        .map(|id| t(reasoncraft::commands::get(id).label))
        .collect();
    let mut seen: HashMap<(accesskit::Role, String), usize> = HashMap::new();
    for node in root.children_recursive() {
        let node = node.accesskit_node();
        let name = name(&node);
        if control(&node) && node.role() != accesskit::Role::Label && !name.is_empty() {
            *seen.entry((node.role(), name)).or_default() += 1;
        }
    }
    for ((role, name), n) in seen {
        if n > 1 && !allowed.contains(&name) {
            out.push(format!("{n} in the window {role:?} {name:?}"));
        }
    }
    out.sort();
    out.dedup();
    out.into_iter().map(|p| format!("{state}: {p}")).collect()
}

/// A claim with a reason (with a misspelling), an objection, an unplaced
/// essay note without a label and a snippet; returns the reason's box.
fn sample() -> (Document, NodeId) {
    let mut d = Document::new("Cats are fine pets");
    let root = Parent::Box(d.root);
    let (_, reason) = d
        .add_card(root, CardKind::Reason, "They recieve love")
        .unwrap();
    d.add_card(root, CardKind::Objection, "They scratch")
        .unwrap();
    d.essay
        .unplaced
        .push((String::new(), "An old paragraph".into()));
    d.essay.unplaced.push(("Gone".into(), "Another one".into()));
    d.essay.unplaced.push(("Gone".into(), "A third one".into()));
    d.add_snippet("A quote from a book");
    (d, reason)
}

fn sample_harness() -> (H, NodeId) {
    let (d, reason) = sample();
    let mut app = app_with_doc(d);
    app.settings.minimap = MinimapMode::On;
    (harness(app), reason)
}

/// Run frames until `lang`'s dictionary is loaded, then settle.
fn wait_dictionary(h: &mut H, lang: &str) {
    crate::common::poll_until("the dictionary", || {
        h.step();
        h.state_mut().spell.get(lang).is_some()
    });
    crate::common::settle(h);
}

fn act(h: &mut H, action: Action) {
    apply(h.state_mut(), action);
    h.run();
}

#[test]
fn every_interactive_widget_has_a_name() {
    let (mut h, reason) = sample_harness();
    let mut all = Vec::new();
    // The map with the find bar, the spelling panel and the command list.
    act(&mut h, Action::Select(Some(Item::Box(reason))));
    act(&mut h, Action::OpenSpelling);
    wait_dictionary(&mut h, "en");
    act(&mut h, Action::OpenFind { replace: true });
    act(&mut h, Action::OpenCommandSearch);
    h.event(egui::Event::Text("zoom".into()));
    h.run();
    all.extend(problems(&h, "map"));
    h.key_press(Key::Escape);
    h.run();
    // Two commands named "Nil" (a card's and a source's rating).
    h.key_press_modifiers(Modifiers::ALT, Key::Q);
    h.run();
    h.event(egui::Event::Text("nil".into()));
    h.run();
    all.extend(problems(&h, "command search"));
    h.key_press(Key::Escape);
    h.run();
    for tab in [TextTab::Essay, TextTab::Check, TextTab::Snippets] {
        act(&mut h, Action::SetTextTab(tab));
        all.extend(problems(&h, &format!("{tab:?} tab")));
    }
    // Every help page, with its links.
    act(&mut h, Action::OpenHelp(HelpPage::Contents));
    all.extend(problems(&h, "help contents"));
    for topic in reasoncraft::help::TOPICS {
        act(&mut h, Action::OpenHelp(HelpPage::Topic(topic)));
        all.extend(problems(&h, &format!("help {topic}")));
    }
    for (tab, key) in [
        (RibbonTab::Insert, "insert"),
        (RibbonTab::Evaluate, "evaluate"),
        (RibbonTab::View, "view"),
        (RibbonTab::Export, "export"),
        (RibbonTab::Help, "help"),
    ] {
        h.state_mut().ribbon_tab = tab;
        h.run();
        all.extend(problems(&h, &format!("{key} ribbon")));
    }
    let card = h.state().tabs[0].doc.cards.keys().next().copied().unwrap();
    act(&mut h, Action::Select(Some(Item::Card(card))));
    h.state_mut().ribbon_tab = RibbonTab::Evaluate;
    h.run();
    all.extend(problems(&h, "evaluate ribbon, card"));
    h.state_mut().ribbon_tab = RibbonTab::Home;
    act(&mut h, Action::Select(Some(Item::Box(reason))));
    act(&mut h, Action::EditLink);
    all.extend(problems(&h, "link editor"));
    act(&mut h, Action::DismissModal);
    act(&mut h, Action::OpenComments(reason));
    all.extend(problems(&h, "comments"));
    act(&mut h, Action::CloseComments);
    act(&mut h, Action::OpenExport(ExportFormat::Png));
    all.extend(problems(&h, "export dialog"));
    act(&mut h, Action::CloseExportDialog);
    h.state_mut().modal = Some(Modal::ImportReport(vec![ImportNote {
        line: 3,
        kind: ImportNoteKind::Contradiction,
    }]));
    h.run();
    all.extend(problems(&h, "import report"));
    act(&mut h, Action::DismissModal);
    act(&mut h, Action::ResetSettings);
    all.extend(problems(&h, "reset prompt"));
    act(&mut h, Action::DismissModal);
    // Two recent maps of the same name, in different folders.
    h.state_mut().settings.recent_files = vec!["one/map.rcraft".into(), "two/map.rcraft".into()];
    h.state_mut().ribbon_tab = RibbonTab::File;
    h.run();
    for page in [
        "action.new",
        "backstage.recent",
        "backstage.document",
        "backstage.settings",
        "backstage.about",
    ] {
        h.get_by_label(&t(page)).click();
        h.run();
        all.extend(problems(&h, page));
    }
    assert!(all.is_empty(), "{all:#?}");
}

/// A note whose label is blank (a file may hold one) is called by its
/// number, on screen and for screen readers.
#[test]
fn unplaced_note_without_label_is_named() {
    let mut doc = Document::new("Claim");
    doc.essay
        .unplaced
        .push(("  ".to_owned(), "Old text".to_owned()));
    let mut app = app_with_doc(doc);
    app.settings.text_tab = TextTab::Essay;
    let mut h = harness(app);
    let mut args = FluentArgs::new();
    args.set("n", 1);
    let label = t_args("essay.unplaced_note", &args);
    h.get_by_label(&label).scroll_to_me();
    h.run();
    h.get_by_label(&label);
    let mut args = FluentArgs::new();
    args.set("name", label.as_str());
    h.get_by_label(&t_args("essay.remove_name", &args));
}
