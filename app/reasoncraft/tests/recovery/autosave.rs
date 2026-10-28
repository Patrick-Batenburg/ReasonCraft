//! Autosave while the app runs: when copies are written and removed.

use std::time::Duration;

use egui_kittest::kittest::Queryable;
use reasoncraft::ReasonCraftApp;
use reasoncraft::actions::{self, Action, UnsavedChoice};
use reasoncraft::i18n::t;
use reasoncraft::recovery;

use super::*;

#[test]
fn test_apps_run_without_recovery() {
    assert!(ReasonCraftApp::new_for_test().recovery.is_none());
}

#[test]
fn autosave_writes_after_interval() {
    let rec = tempfile::tempdir().unwrap();
    let mut app = app_with_recovery(rec.path());
    set_claim(&mut app, "Robots help");
    recovery::tick(&mut app, 10.0);
    recovery::tick(&mut app, 30.0);
    wait(&app);
    let copy = recovery_file(&app, 0);
    assert!(!copy.exists(), "nothing before the interval");
    recovery::tick(&mut app, 56.0);
    wait(&app);
    assert!(copy.is_file());
    assert!(copy.with_extension("json").is_file());
    assert_eq!(root_text(&copy), "Robots help");
}

#[test]
fn no_autosave_when_clean() {
    let rec = tempfile::tempdir().unwrap();
    let mut app = app_with_recovery(rec.path());
    autosave(&mut app, 0.0);
    let store = app.recovery.as_ref().unwrap();
    let names: Vec<_> = std::fs::read_dir(store.session_dir())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names, ["session.lock"]);
}

#[test]
fn clean_close_removes_recovery() {
    let rec = tempfile::tempdir().unwrap();
    let mut app = app_with_recovery(rec.path());
    set_claim(&mut app, "Robots help");
    autosave(&mut app, 0.0);
    let copy = recovery_file(&app, 0);
    assert!(copy.is_file());
    actions::apply(&mut app, Action::CloseTab(0));
    actions::apply(&mut app, Action::ResolveUnsaved(UnsavedChoice::Discard));
    wait(&app);
    assert!(!copy.exists());
    assert!(!copy.with_extension("json").exists());
}

#[test]
fn undo_to_saved_state_removes_recovery() {
    let rec = tempfile::tempdir().unwrap();
    let mut app = app_with_recovery(rec.path());
    set_claim(&mut app, "Robots help");
    autosave(&mut app, 0.0);
    let copy = recovery_file(&app, 0);
    assert!(copy.is_file());
    actions::apply(&mut app, Action::Undo);
    recovery::tick(&mut app, 50.0);
    wait(&app);
    assert!(!copy.exists());
}

#[test]
fn save_removes_recovery() {
    let rec = tempfile::tempdir().unwrap();
    let docs = tempfile::tempdir().unwrap();
    let mut app = app_with_recovery(rec.path());
    app.tabs[0].path = Some(docs.path().join("map.rcraft"));
    set_claim(&mut app, "Robots help");
    autosave(&mut app, 0.0);
    let copy = recovery_file(&app, 0);
    assert!(copy.is_file());
    actions::apply(&mut app, Action::Save);
    assert!(!app.tabs[0].is_dirty());
    recovery::tick(&mut app, 50.0);
    wait(&app);
    assert!(!copy.exists());
}

#[test]
fn normal_exit_removes_own_session() {
    let rec = tempfile::tempdir().unwrap();
    let mut app = app_with_recovery(rec.path());
    set_claim(&mut app, "Robots help");
    autosave(&mut app, 0.0);
    let session = app.recovery.as_ref().unwrap().session_dir();
    assert!(session.is_dir());
    actions::apply(&mut app, Action::CloseWindow);
    actions::apply(&mut app, Action::ResolveUnsaved(UnsavedChoice::Discard));
    assert!(app.close_confirmed());
    assert!(app.recovery.is_none());
    assert!(!session.exists());
    assert!(app_with_recovery(rec.path()).modal.is_none());
}

#[test]
fn autosave_interval_clamped() {
    let mut app = ReasonCraftApp::new_for_test();
    assert_eq!(app.settings.autosave_secs, 45);
    actions::apply(&mut app, Action::SetAutosaveSecs(5));
    assert_eq!(app.settings.autosave_secs, 30);
    actions::apply(&mut app, Action::SetAutosaveSecs(10_000));
    assert_eq!(app.settings.autosave_secs, 600);
    actions::apply(&mut app, Action::SetAutosaveSecs(0));
    assert_eq!(app.settings.autosave_interval(), None);
    // Stored settings outside the range are clamped when used.
    app.settings.autosave_secs = 7;
    assert_eq!(app.settings.autosave_interval(), Some(30));

    // 0 turns autosave off.
    let rec = tempfile::tempdir().unwrap();
    let mut app = app_with_recovery(rec.path());
    actions::apply(&mut app, Action::SetAutosaveSecs(0));
    set_claim(&mut app, "Robots help");
    autosave(&mut app, 0.0);
    recovery::tick(&mut app, 1000.0);
    wait(&app);
    assert!(!recovery_file(&app, 0).exists());
}

#[test]
fn autosave_never_blocks_ui() {
    let rec = tempfile::tempdir().unwrap();
    let mut app = app_with_recovery(rec.path());
    app.recovery
        .as_mut()
        .unwrap()
        .set_write_delay(Some(Duration::from_millis(400)));
    set_claim(&mut app, "Robots help");
    recovery::tick(&mut app, 0.0);
    recovery::tick(&mut app, 46.0);
    // The write runs on a worker: tick returned while it is pending.
    let copy = recovery_file(&app, 0);
    assert!(app.recovery.as_ref().unwrap().busy());
    assert!(!copy.exists());
    wait(&app);
    assert!(!app.recovery.as_ref().unwrap().busy());
    assert!(copy.is_file());
}

#[test]
fn unavailable_recovery_folder_is_noted() {
    let tmp = tempfile::tempdir().unwrap();
    let not_a_dir = tmp.path().join("file");
    std::fs::write(&not_a_dir, b"x").unwrap();
    let app = app_with_recovery(&not_a_dir);
    assert!(app.recovery.is_none());
    assert_eq!(
        app.status_note.as_deref(),
        Some(t("recovery.unavailable").as_str())
    );
}

#[test]
fn settings_page_sets_the_autosave_interval() {
    let mut h = crate::common::build(ReasonCraftApp::new_for_test());
    h.state_mut().ribbon_tab = reasoncraft::app::RibbonTab::File;
    h.run();
    h.get_by_label(&t("backstage.settings")).click();
    h.run();
    h.get_by_label(&t("settings.autosave_interval"));
    h.state_mut().settings.autosave_secs = 120;
    h.run();
    let checkbox = || (egui::accesskit::Role::CheckBox, t("settings.autosave"));
    let (role, label) = checkbox();
    h.get_by_role_and_label(role, &label).click();
    h.run();
    assert_eq!(h.state().settings.autosave_interval(), None);
    assert!(
        h.query_by_label(&t("settings.autosave_interval")).is_none(),
        "no interval while off"
    );
    // Back on: the interval it had before.
    h.get_by_role_and_label(role, &label).click();
    h.run();
    assert_eq!(h.state().settings.autosave_interval(), Some(120));
    // Older settings without the field autosave every 45 seconds.
    let old: reasoncraft::app::Settings = serde_json::from_str(r#"{"theme":"Light"}"#).unwrap();
    assert_eq!(old.autosave_secs, 45);
}

#[test]
fn failed_write_is_noted_and_retried() {
    let rec = tempfile::tempdir().unwrap();
    let mut app = app_with_recovery(rec.path());
    app.recovery.as_mut().unwrap().set_fail_writes(true);
    set_claim(&mut app, "Robots help");
    autosave(&mut app, 0.0);
    let copy = recovery_file(&app, 0);
    assert!(!copy.exists());
    // The next frame picks up the failure.
    recovery::tick(&mut app, 47.0);
    let note = app.status_note.clone().unwrap_or_default();
    let prefix = t("recovery.write_failed");
    let prefix = prefix.split(':').next().unwrap();
    assert!(note.starts_with(prefix), "{note}");
    // Once the folder works again, the next autosave writes the copy.
    app.recovery.as_mut().unwrap().set_fail_writes(false);
    recovery::tick(&mut app, 100.0);
    wait(&app);
    assert!(copy.is_file());
    assert_eq!(root_text(&copy), "Robots help");
}

#[test]
fn imported_tab_is_autosaved_before_any_edit() {
    let rec = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("coffee.argdown");
    std::fs::write(
        &source,
        "[A]: Coffee is good.
",
    )
    .unwrap();
    let fake = reasoncraft::file_ops::FakeDialogs::default();
    fake.set_import(source);
    let mut app = app_with_recovery(rec.path());
    app.dialogs = Box::new(fake);
    actions::apply(&mut app, Action::ImportArgdown);
    assert_eq!(app.tabs.len(), 2);
    autosave(&mut app, 0.0);
    let copy = recovery_file(&app, 1);
    assert!(copy.is_file(), "the unedited import has a recovery copy");
    assert_eq!(root_text(&copy), "Coffee is good.");
}
