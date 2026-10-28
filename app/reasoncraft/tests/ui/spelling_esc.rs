//! Esc closes the spelling box (F7) when the focus is in it, on the map or
//! nowhere, but not when it is in another field.

use crate::common::{app_with_doc, harness};
use eframe::egui::{self, Key};
use egui_kittest::kittest::Queryable;
use reasoncraft::i18n::t;
use reasoncraft_core::Document;

use crate::common::H;

/// A map whose claim is selected, with the spelling box open.
fn with_spelling_open() -> H {
    let doc = Document::new("Teh claim");
    let root = doc.root;
    let mut app = app_with_doc(doc);
    app.tabs[0].selection = Some(reasoncraft_core::Item::Box(root));
    let mut h = harness(app);
    h.key_press(Key::F7);
    h.run();
    assert!(h.state().tabs[0].spelling.is_some(), "F7 opens it");
    h
}

#[test]
fn esc_closes_spelling_from_the_map() {
    let mut h = with_spelling_open();
    h.key_press(Key::Escape);
    h.run();
    assert!(h.state().tabs[0].spelling.is_none());
}

#[test]
fn esc_closes_spelling_from_inside_it() {
    let mut h = with_spelling_open();
    h.get_by_label(&t("spell.close_name")).focus();
    h.run();
    h.key_press(Key::Escape);
    h.run();
    assert!(h.state().tabs[0].spelling.is_none());
}

#[test]
fn esc_in_the_find_field_leaves_spelling_open() {
    let mut h = with_spelling_open();
    h.key_press_modifiers(egui::Modifiers::COMMAND, Key::F);
    h.run();
    assert!(h.state().tabs[0].find.open);
    h.key_press(Key::Escape);
    h.run();
    let tab = &h.state().tabs[0];
    assert!(!tab.find.open, "Esc closes the find bar");
    assert!(tab.spelling.is_some(), "and leaves the spelling box");
}
