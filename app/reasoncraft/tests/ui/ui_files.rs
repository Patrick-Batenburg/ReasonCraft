//! UI tests for file operations: save, open, close prompts.

use crate::common::{build, wait_saves};
use eframe::egui;
use egui_kittest::{Harness, kittest::Queryable};
use reasoncraft::ReasonCraftApp;
use reasoncraft::file_ops::FakeDialogs;
use reasoncraft::i18n::t;

fn type_text(h: &mut Harness<'static, ReasonCraftApp>, text: &str) {
    h.event(egui::Event::Text(text.to_owned()));
    h.run();
}

// ---- Task 11: file operations ----

fn file_harness() -> (Harness<'static, ReasonCraftApp>, FakeDialogs) {
    let fake = FakeDialogs::default();
    let app = ReasonCraftApp::new_for_test_with(Box::new(fake.clone()));
    let h = build(app);
    (h, fake)
}

/// Press Ctrl+`key` (Cmd on macOS); a save it starts is written before
/// this returns.
fn cmd(h: &mut Harness<'static, ReasonCraftApp>, key: egui::Key) {
    h.key_press_modifiers(egui::Modifiers::COMMAND, key);
    h.run_steps(2);
    wait_saves(h);
}

/// Type `text` on the selected root claim and finish the edit.
fn type_claim(h: &mut Harness<'static, ReasonCraftApp>, text: &str) {
    type_text(h, text);
    h.key_press(egui::Key::Escape);
    h.run();
}

fn close_label(h: &Harness<'static, ReasonCraftApp>, i: usize) -> String {
    let mut args = reasoncraft::i18n::FluentArgs::new();
    args.set("name", h.state().tabs[i].title());
    reasoncraft::i18n::t_args("tabs.close", &args)
}

#[test]
fn save_then_open_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, fake) = file_harness();
    h.run();
    type_claim(&mut h, "Robots help");
    assert!(h.state().active_tab().is_dirty());

    // No extension given: Save forces `.rcraft`.
    fake.set_save(dir.path().join("robots"));
    cmd(&mut h, egui::Key::S);
    let saved = dir.path().join("robots.rcraft");
    assert!(saved.is_file());
    let tab = h.state().active_tab();
    assert_eq!(tab.path.as_deref(), Some(saved.as_path()));
    assert!(!tab.is_dirty());
    assert_eq!(tab.title(), "robots");

    // A clean tab closes without asking; the last tab is replaced by a new one.
    let label = close_label(&h, 0);
    h.get_by_label(&label).click();
    h.run();
    assert!(h.state().modal.is_none());
    assert_eq!(h.state().tabs.len(), 1);
    assert_eq!(h.state().tabs[0].path, None);

    // File → Open reuses the untouched new tab.
    fake.set_open(saved.clone());
    h.get_by_label(&t("ribbon.tab.file")).click();
    h.run();
    h.get_by_label(&t("action.open")).click();
    h.run();
    let app = h.state();
    assert_eq!(app.tabs.len(), 1);
    let tab = app.active_tab();
    assert_eq!(tab.doc.boxes[&tab.doc.root].text, "Robots help");
    assert_eq!(tab.path.as_deref(), Some(saved.as_path()));
    assert!(!tab.is_dirty());

    // Opening the same file again just activates its tab.
    h.get_by_label(&t("tabs.new")).click();
    h.run();
    type_claim(&mut h, "Other");
    assert_eq!(h.state().active, 1);
    fake.set_open(saved);
    cmd(&mut h, egui::Key::O);
    assert_eq!(h.state().tabs.len(), 2);
    assert_eq!(h.state().active, 0);
}

#[test]
fn failed_open_keeps_tabs() {
    let dir = tempfile::tempdir().unwrap();
    let garbage = dir.path().join("garbage.rcraft");
    std::fs::write(&garbage, b"this is not a zip file").unwrap();
    let (mut h, fake) = file_harness();
    h.run();
    type_claim(&mut h, "Unsaved work");
    h.get_by_label(&t("tabs.new")).click();
    h.run();
    assert_eq!(h.state().tabs.len(), 2);

    fake.set_open(garbage);
    cmd(&mut h, egui::Key::O);
    let app = h.state();
    assert_eq!(app.tabs.len(), 2);
    assert_eq!(app.active, 1);
    let first = &app.tabs[0];
    assert_eq!(first.doc.boxes[&first.doc.root].text, "Unsaved work");
    assert!(app.modal.is_some());
    h.get_by_label(&t("error.open.not_a_file"));
    // In the heading and in the dialog's name.
    assert!(h.query_all_by_label_contains("garbage.rcraft").count() >= 1);

    h.get_by_label(&t("modal.ok")).click();
    h.run();
    assert!(h.state().modal.is_none());
    assert_eq!(h.state().tabs.len(), 2);
}

#[test]
fn undo_after_save_marks_dirty() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, fake) = file_harness();
    h.run();
    type_claim(&mut h, "Claim");
    fake.set_save(dir.path().join("map.rcraft"));
    cmd(&mut h, egui::Key::S);
    assert!(!h.state().active_tab().is_dirty());
    assert_eq!(h.state().active_tab().title(), "map");

    cmd(&mut h, egui::Key::Z);
    assert!(h.state().active_tab().is_dirty());
    assert_eq!(h.state().active_tab().title(), "map •");
    cmd(&mut h, egui::Key::Y);
    assert!(!h.state().active_tab().is_dirty());
}

#[test]
fn save_while_editing_commits_the_text() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, fake) = file_harness();
    h.run();
    type_text(&mut h, "Half typed");
    assert!(h.state().active_tab().editing().is_some());
    assert!(h.state().active_tab().is_dirty(), "a pending edit is dirty");
    let path = dir.path().join("typed.rcraft");
    fake.set_save(path.clone());
    cmd(&mut h, egui::Key::S);
    let tab = h.state().active_tab();
    assert_eq!(tab.editing(), None);
    assert!(!tab.is_dirty());
    let doc = reasoncraft_core::file::open(&path).unwrap();
    assert_eq!(doc.boxes[&doc.root].text, "Half typed");
}

#[test]
fn close_dirty_tab_asks_to_save() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, fake) = file_harness();
    h.run();
    // Still editing: the pending text counts as an unsaved change.
    type_text(&mut h, "Draft");
    let label = close_label(&h, 0);
    h.get_by_label(&label).click();
    h.run();
    assert!(h.state().modal.is_some());
    h.get_by_label(&t("unsaved.message"));

    // Cancel keeps the tab and its work.
    h.get_by_label(&t("unsaved.cancel")).click();
    h.run();
    assert!(h.state().modal.is_none());
    let tab = h.state().active_tab();
    assert_eq!(tab.doc.boxes[&tab.doc.root].text, "Draft");
    assert!(tab.is_dirty());

    // Save writes the file, then closes the tab.
    let label = close_label(&h, 0);
    h.get_by_label(&label).click();
    h.run();
    let path = dir.path().join("draft.rcraft");
    fake.set_save(path.clone());
    h.get_by_label(&t("unsaved.save_changes")).click();
    h.run_steps(2);
    wait_saves(&mut h);
    assert!(h.state().modal.is_none());
    assert!(path.is_file());
    assert_eq!(h.state().tabs.len(), 1);
    assert_eq!(h.state().tabs[0].path, None);

    // Don't save discards the changes.
    type_claim(&mut h, "Throwaway");
    let label = close_label(&h, 0);
    h.get_by_label(&label).click();
    h.run();
    h.get_by_label(&t("unsaved.discard")).click();
    h.run();
    let tab = h.state().active_tab();
    assert_eq!(tab.doc.boxes[&tab.doc.root].text, "");
    assert!(!tab.is_dirty());
}

#[test]
fn window_close_asks_per_dirty_tab() {
    let (mut h, _fake) = file_harness();
    h.run();
    type_claim(&mut h, "One");
    h.get_by_label(&t("tabs.new")).click();
    h.run();
    h.get_by_label(&t("tabs.new")).click();
    h.run();
    type_claim(&mut h, "Three");

    // The OS asks to close: ask about tab 0 first, then tab 2 (tab 1 is clean).
    h.input_mut()
        .viewports
        .get_mut(&egui::ViewportId::ROOT)
        .unwrap()
        .events
        .push(egui::ViewportEvent::Close);
    h.run();
    assert!(h.state().modal.is_some());
    assert!(!h.state().close_confirmed());
    assert_eq!(h.state().active, 0, "the tab in question is shown");
    h.get_by_label(&t("unsaved.discard")).click();
    h.run();
    assert_eq!(h.state().active, 2);
    // Cancel aborts the whole close.
    h.get_by_label(&t("unsaved.cancel")).click();
    h.run();
    assert!(h.state().modal.is_none());
    assert!(!h.state().close_confirmed());
    assert_eq!(h.state().tabs.len(), 3);

    // The title bar's close button goes the same way.
    h.state_mut()
        .push(reasoncraft::actions::Action::CloseWindow);
    h.run();
    h.get_by_label(&t("unsaved.discard")).click();
    h.run();
    h.get_by_label(&t("unsaved.discard")).click();
    h.run();
    assert!(h.state().modal.is_none());
    assert!(h.state().close_confirmed());
}

fn replace_title(name: &str) -> String {
    let mut args = reasoncraft::i18n::FluentArgs::new();
    args.set("file", name);
    reasoncraft::i18n::t_args("replace.title", &args)
}

fn root_text_in(path: &std::path::Path) -> String {
    let doc = reasoncraft_core::file::open(path).unwrap();
    doc.boxes[&doc.root].text.clone()
}

/// The dialog checked `robots` for overwriting, not `robots.rcraft`: the app
/// asks before replacing the file the appended extension points at.
#[test]
fn appended_extension_asks_before_replacing() {
    let dir = tempfile::tempdir().unwrap();
    let existing = dir.path().join("robots.rcraft");
    reasoncraft_core::file::save(&reasoncraft_core::Document::new("Old"), &existing).unwrap();
    let before = std::fs::read(&existing).unwrap();
    let (mut h, fake) = file_harness();
    h.run();
    type_claim(&mut h, "New");

    fake.set_save(dir.path().join("robots"));
    cmd(&mut h, egui::Key::S);
    assert!(h.state().modal.is_some());
    h.get_by_role_and_label(
        eframe::egui::accesskit::Role::Dialog,
        &replace_title("robots.rcraft"),
    );
    h.get_by_label(&t("replace.cancel")).click();
    h.run();
    assert!(h.state().modal.is_none());
    assert_eq!(std::fs::read(&existing).unwrap(), before);
    let tab = h.state().active_tab();
    assert!(tab.is_dirty());
    assert_eq!(tab.path, None);

    fake.set_save(dir.path().join("robots"));
    cmd(&mut h, egui::Key::S);
    h.get_by_label(&t("replace.confirm")).click();
    h.run_steps(2);
    wait_saves(&mut h);
    assert!(h.state().modal.is_none());
    assert_eq!(root_text_in(&existing), "New");
    let tab = h.state().active_tab();
    assert!(!tab.is_dirty());
    assert_eq!(tab.path.as_deref(), Some(existing.as_path()));
}

/// A name that already ends in `.rcraft` was checked by the dialog itself.
#[test]
fn chosen_rcraft_name_saves_without_extra_confirm() {
    let dir = tempfile::tempdir().unwrap();
    let existing = dir.path().join("robots.rcraft");
    reasoncraft_core::file::save(&reasoncraft_core::Document::new("Old"), &existing).unwrap();
    let (mut h, fake) = file_harness();
    h.run();
    type_claim(&mut h, "New");
    fake.set_save(existing.clone());
    cmd(&mut h, egui::Key::S);
    assert!(h.state().modal.is_none());
    assert_eq!(root_text_in(&existing), "New");
    assert!(!h.state().active_tab().is_dirty());
}

fn two_dirty_tabs() -> (Harness<'static, ReasonCraftApp>, FakeDialogs) {
    let (mut h, fake) = file_harness();
    h.run();
    type_claim(&mut h, "One");
    h.get_by_label(&t("tabs.new")).click();
    h.run();
    type_claim(&mut h, "Two");
    (h, fake)
}

#[test]
fn failed_save_during_window_close_aborts_it() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, fake) = two_dirty_tabs();
    h.state_mut()
        .push(reasoncraft::actions::Action::CloseWindow);
    h.run();
    fake.set_save(dir.path().join("no/such/dir/one.rcraft"));
    h.get_by_label(&t("unsaved.save_changes")).click();
    h.run_steps(2);
    wait_saves(&mut h);
    assert!(matches!(
        h.state().modal,
        Some(reasoncraft::file_ops::Modal::Error { .. })
    ));
    assert!(!h.state().close_confirmed());
    h.get_by_label(&t("modal.ok")).click();
    h.run();
    let app = h.state();
    assert!(app.modal.is_none());
    assert!(!app.close_confirmed());
    assert_eq!(app.tabs.len(), 2);
    assert!(app.tabs.iter().all(|tab| tab.is_dirty()));
}

#[test]
fn replace_prompt_during_window_close() {
    let dir = tempfile::tempdir().unwrap();
    let existing = dir.path().join("one.rcraft");
    reasoncraft_core::file::save(&reasoncraft_core::Document::new("Old"), &existing).unwrap();
    let (mut h, fake) = two_dirty_tabs();

    // Cancel at the replace prompt aborts the whole close.
    h.state_mut()
        .push(reasoncraft::actions::Action::CloseWindow);
    h.run();
    fake.set_save(dir.path().join("one"));
    h.get_by_label(&t("unsaved.save_changes")).click();
    h.run_steps(2);
    wait_saves(&mut h);
    h.get_by_label(&t("replace.cancel")).click();
    h.run();
    assert!(h.state().modal.is_none());
    assert!(!h.state().close_confirmed());
    assert_eq!(root_text_in(&existing), "Old");

    // Replace saves tab 0, then the close moves on to tab 1.
    h.state_mut()
        .push(reasoncraft::actions::Action::CloseWindow);
    h.run();
    fake.set_save(dir.path().join("one"));
    h.get_by_label(&t("unsaved.save_changes")).click();
    h.run_steps(2);
    wait_saves(&mut h);
    h.get_by_label(&t("replace.confirm")).click();
    h.run_steps(2);
    wait_saves(&mut h);
    assert_eq!(root_text_in(&existing), "One");
    assert_eq!(h.state().active, 1);
    h.get_by_label(&t("unsaved.discard")).click();
    h.run();
    assert!(h.state().close_confirmed());
}
