//! The keyboard on the File page with no field focused: the file keys,
//! Search commands, F1, and Esc back to the map.

use crate::common::{H, build};
use eframe::egui::{Key, Modifiers};
use egui_kittest::kittest::Queryable;
use reasoncraft::ReasonCraftApp;
use reasoncraft::app::RibbonTab;
use reasoncraft::file_ops::FakeDialogs;
use reasoncraft::i18n::t;
use reasoncraft::settings::TextTab;

fn on_file_page(fake: &FakeDialogs) -> H {
    let mut app = ReasonCraftApp::new_for_test_with(Box::new(fake.clone()));
    app.ribbon_tab = RibbonTab::File;
    let mut h = build(app);
    h.run();
    assert!(!h.ctx.text_edit_focused());
    h
}

#[test]
fn esc_goes_back_to_the_map() {
    let mut h = on_file_page(&FakeDialogs::default());
    h.key_press(Key::Escape);
    h.run();
    assert_eq!(h.state().ribbon_tab, RibbonTab::Home);
}

#[test]
fn ctrl_n_makes_a_map_and_shows_it() {
    let mut h = on_file_page(&FakeDialogs::default());
    h.key_press_modifiers(Modifiers::COMMAND, Key::N);
    h.run();
    assert_eq!(h.state().tabs.len(), 2);
    assert_eq!(h.state().ribbon_tab, RibbonTab::Home);
}

#[test]
fn ctrl_s_saves() {
    let fake = FakeDialogs::default();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("saved.rcraft");
    fake.set_save(path.clone());
    let mut h = on_file_page(&fake);
    h.key_press_modifiers(Modifiers::COMMAND, Key::S);
    h.run();
    assert_eq!(h.state().tabs[0].path.as_deref(), Some(path.as_path()));
    assert!(path.is_file());
}

#[test]
fn f1_shows_help() {
    let mut h = on_file_page(&FakeDialogs::default());
    h.key_press(Key::F1);
    h.run();
    assert_eq!(h.state().ribbon_tab, RibbonTab::Home);
    assert_eq!(h.state().settings.text_tab, TextTab::Help);
}

#[test]
fn alt_q_opens_search_commands() {
    let mut h = on_file_page(&FakeDialogs::default());
    h.key_press_modifiers(Modifiers::ALT, Key::Q);
    h.run();
    assert!(h.ctx.text_edit_focused(), "the search field has the focus");
}

#[test]
fn esc_in_a_field_only_leaves_the_field() {
    let mut h = on_file_page(&FakeDialogs::default());
    h.get_by_label(&t("backstage.document")).click();
    h.run();
    let title = t("document.title_field");
    h.get_by_label(&title).click();
    h.run();
    assert!(h.ctx.text_edit_focused());
    h.key_press(Key::Escape);
    h.run();
    assert!(!h.ctx.text_edit_focused());
    assert_eq!(h.state().ribbon_tab, RibbonTab::File, "still on the page");
    h.key_press(Key::Escape);
    h.run();
    assert_eq!(h.state().ribbon_tab, RibbonTab::Home);
}
