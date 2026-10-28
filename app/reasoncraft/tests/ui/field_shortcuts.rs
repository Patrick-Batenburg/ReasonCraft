//! The file keys (Ctrl+S, Ctrl+Shift+S, Ctrl+O, Ctrl+N) while a text
//! field outside the map has focus: they end the field's editing session,
//! then run, like they do while a box is being edited.

use crate::common::build;
use eframe::egui::{self, Key, Modifiers};
use egui_kittest::kittest::Queryable;
use reasoncraft::ReasonCraftApp;
use reasoncraft::app::RibbonTab;
use reasoncraft::doc_tab::{DocTab, EditSession};
use reasoncraft::file_ops::FakeDialogs;
use reasoncraft::i18n::t;
use reasoncraft::settings::TextTab;
use reasoncraft_core::Document;

use crate::common::H;

fn harness(doc: Document, fake: &FakeDialogs) -> H {
    let mut app = ReasonCraftApp::new_for_test_with(Box::new(fake.clone()));
    app.tabs[0] = DocTab::from_document(doc);
    app.settings.text_tab = TextTab::Snippets;
    build(app)
}

fn doc_with_snippet() -> Document {
    let mut d = Document::new("Robots help people");
    d.add_snippet("Robots");
    d
}

#[test]
fn ctrl_s_in_snippet_field_records_the_session_and_saves() {
    let fake = FakeDialogs::default();
    let mut h = harness(doc_with_snippet(), &fake);
    h.run();
    let field = t("snippet.text");
    h.get_by_label(&field).click();
    h.run();
    h.get_by_label(&field).type_text(" help");
    h.run();
    assert_eq!(h.state().tabs[0].history.undo.len(), 0, "typing is live");

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("snippets.rcraft");
    fake.set_save(path.clone());
    h.key_press_modifiers(Modifiers::COMMAND, Key::S);
    h.run();

    let tab = &h.state().tabs[0];
    assert_eq!(tab.path.as_deref(), Some(path.as_path()), "saved");
    assert!(!tab.is_dirty());
    assert_eq!(tab.history.undo.len(), 1, "the typing is one step");
    assert!(
        !matches!(tab.session(), Some(EditSession::Snippet { .. })),
        "the session ended"
    );
    assert_eq!(tab.doc.snippets[0].text, "Robots help", "no 's' typed");
    let saved = reasoncraft_core::file::open(&path).unwrap();
    assert_eq!(saved.snippets[0].text, "Robots help");
    // The field keeps its focus; other keys still type into it.
    assert!(h.ctx.text_edit_focused());
}

/// Alt+Q from a field opens Search commands; the "œ" macOS types with it
/// (in the same frame) does not land in the field.
#[test]
fn alt_q_in_snippet_field_types_nothing() {
    let fake = FakeDialogs::default();
    let mut h = harness(doc_with_snippet(), &fake);
    h.run();
    let field = t("snippet.text");
    h.get_by_label(&field).click();
    h.run();
    h.input_mut().events.extend([
        egui::Event::Key {
            key: Key::Q,
            physical_key: Some(Key::Q),
            pressed: true,
            repeat: false,
            modifiers: Modifiers::ALT,
        },
        egui::Event::Text("œ".into()),
    ]);
    h.run();
    assert_eq!(h.state().tabs[0].doc.snippets[0].text, "Robots");
    assert!(
        h.ctx
            .memory(|m| m.has_focus(reasoncraft::shell::command_search::field_id())),
        "the search box has the focus"
    );
}

#[test]
fn other_map_keys_stay_with_the_snippet_field() {
    let fake = FakeDialogs::default();
    let mut h = harness(doc_with_snippet(), &fake);
    h.run();
    let field = t("snippet.text");
    h.get_by_label(&field).click();
    h.run();
    let selected = h.state().tabs[0].selection;
    // Delete and the arrows edit the text, not the map.
    for key in [Key::ArrowLeft, Key::Delete, Key::Tab] {
        h.key_press(key);
        h.run();
    }
    let tab = &h.state().tabs[0];
    assert_eq!(tab.selection, selected);
    assert_eq!(tab.doc.boxes.len(), 1, "Tab added no reason");
}

#[test]
fn ctrl_s_in_title_field_saves() {
    let fake = FakeDialogs::default();
    let mut h = harness(doc_with_snippet(), &fake);
    h.state_mut().ribbon_tab = RibbonTab::File;
    h.run();
    h.get_by_label(&t("backstage.document")).click();
    h.run();
    let field = t("document.title_field");
    h.get_by_label(&field).click();
    h.run();
    h.get_by_label(&field).type_text("Robots");
    h.run();

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("titled.rcraft");
    fake.set_save(path.clone());
    h.key_press_modifiers(Modifiers::COMMAND, Key::S);
    h.run();
    let tab = &h.state().tabs[0];
    assert_eq!(tab.path.as_deref(), Some(path.as_path()), "saved");
    assert!(!tab.is_dirty());
    assert_eq!(tab.history.undo.len(), 1);
    assert_eq!(
        reasoncraft_core::file::open(&path).unwrap().meta.title,
        "Robots"
    );
}
