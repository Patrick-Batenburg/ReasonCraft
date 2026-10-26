//! Autosave to the recovery folder, and the offer to restore after a crash.
//! Every test uses its own temp folder: never the user's recovery folder.

use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use eframe::egui;
use egui_kittest::{Harness, kittest::Queryable};
use reasoncraft::ReasonCraftApp;
use reasoncraft::actions::{self, Action, UnsavedChoice};
use reasoncraft::file_ops::Modal;
use reasoncraft::i18n::t;
use reasoncraft::recovery::{self, RecoveryChoice};
use reasoncraft_core::file;

fn app_with_recovery(dir: &Path) -> ReasonCraftApp {
    ReasonCraftApp::new_for_test_with_recovery(dir.to_path_buf())
}

fn set_claim(app: &mut ReasonCraftApp, text: &str) {
    let tab = &mut app.tabs[app.active];
    tab.edit(|d| d.set_text(d.root, text)).unwrap();
}

fn wait(app: &ReasonCraftApp) {
    app.recovery.as_ref().unwrap().wait_idle();
}

/// Two ticks an interval apart, then wait for the writes.
fn autosave(app: &mut ReasonCraftApp, t0: f64) {
    recovery::tick(app, t0);
    recovery::tick(app, t0 + 46.0);
    wait(app);
}

/// The recovery copy of tab `i`.
fn recovery_file(app: &ReasonCraftApp, i: usize) -> PathBuf {
    let store = app.recovery.as_ref().unwrap();
    store
        .session_dir()
        .join(format!("{}.rcraft", app.tabs[i].recovery_key))
}

fn root_text(path: &Path) -> String {
    let pkg = file::open_package(path).unwrap();
    pkg.doc.boxes[&pkg.doc.root].text.clone()
}

/// A saved map at `path` (its claim `text`), opened in a recovery-enabled
/// app, edited to "Edited" and autosaved; then the app "crashes" (is
/// dropped without a normal exit).
fn crash_with_edited_file(rec: &Path, path: &Path) {
    let mut doc = reasoncraft_core::Document::new("");
    doc.set_text(doc.root, "Saved").unwrap();
    file::save(&doc, path).unwrap();
    let mut app = app_with_recovery(rec);
    reasoncraft::file_ops::open_into_app(&mut app, path);
    set_claim(&mut app, "Edited");
    autosave(&mut app, 0.0);
    assert!(recovery_file(&app, 0).is_file());
    drop(app);
}

fn offered(app: &ReasonCraftApp) -> Vec<recovery::RecoveredDoc> {
    match &app.modal {
        Some(Modal::Recover(docs)) => docs.clone(),
        other => panic!("expected the recovery prompt, got {other:?}"),
    }
}

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
fn restore_opens_dirty_tab_with_original_path() {
    let rec = tempfile::tempdir().unwrap();
    let docs = tempfile::tempdir().unwrap();
    let path = docs.path().join("robots.rcraft");
    crash_with_edited_file(rec.path(), &path);

    let mut app = app_with_recovery(rec.path());
    let docs_offered = offered(&app);
    assert_eq!(docs_offered.len(), 1);
    assert_eq!(
        docs_offered[0].meta.original.as_deref(),
        Some(path.as_path())
    );
    assert_eq!(docs_offered[0].meta.title, "robots");
    let orphan = docs_offered[0].package.clone();

    actions::apply(
        &mut app,
        Action::ResolveRecovery(RecoveryChoice::RestoreAll),
    );
    wait(&app);
    assert!(app.modal.is_none());
    assert_eq!(app.tabs.len(), 1, "the untouched start tab is reused");
    let tab = app.active_tab();
    assert_eq!(tab.path.as_deref(), Some(path.as_path()));
    assert!(tab.is_dirty());
    assert_eq!(tab.doc.boxes[&tab.doc.root].text, "Edited");
    assert!(tab.recovered_from.is_some());
    assert_eq!(tab.title(), "robots (recovered) •");
    // The orphan is gone; the tab is autosaved under this session.
    assert!(!orphan.exists());
    assert!(
        !orphan.parent().unwrap().exists(),
        "empty orphan folder removed"
    );
    assert_eq!(root_text(&recovery_file(&app, 0)), "Edited");
    // The file itself was never touched.
    assert_eq!(root_text(&path), "Saved");
}

#[test]
fn restored_doc_saves_without_asking_when_original_unchanged() {
    let rec = tempfile::tempdir().unwrap();
    let docs = tempfile::tempdir().unwrap();
    let path = docs.path().join("robots.rcraft");
    crash_with_edited_file(rec.path(), &path);
    let mut app = app_with_recovery(rec.path());
    actions::apply(
        &mut app,
        Action::ResolveRecovery(RecoveryChoice::RestoreAll),
    );
    actions::apply(&mut app, Action::Save);
    assert!(app.modal.is_none());
    assert_eq!(root_text(&path), "Edited");
    let tab = app.active_tab();
    assert!(tab.recovered_from.is_none());
    assert_eq!(tab.title(), "robots");
}

#[test]
fn restored_doc_save_asks_when_original_changed() {
    let rec = tempfile::tempdir().unwrap();
    let docs = tempfile::tempdir().unwrap();
    let path = docs.path().join("robots.rcraft");
    crash_with_edited_file(rec.path(), &path);
    // Someone changed the file after the crash.
    let later = SystemTime::now() + Duration::from_secs(60);
    File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(later)
        .unwrap();

    let mut app = app_with_recovery(rec.path());
    actions::apply(
        &mut app,
        Action::ResolveRecovery(RecoveryChoice::RestoreAll),
    );
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .with_max_steps(32)
        .build_ui_state(|ui, app: &mut ReasonCraftApp| app.show(ui), app);
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
    h.run();
    assert!(matches!(
        h.state().modal,
        Some(Modal::ConfirmReplace {
            original_changed: true,
            ..
        })
    ));
    h.get_by_label(&t("recovery.original_changed"));
    assert_eq!(root_text(&path), "Saved", "nothing written yet");
    h.get_by_label(&t("replace.confirm")).click();
    h.run();
    assert!(h.state().modal.is_none());
    assert_eq!(root_text(&path), "Edited");
    assert!(!h.state().active_tab().is_dirty());
}

#[test]
fn discard_all_removes_files() {
    let rec = tempfile::tempdir().unwrap();
    let docs = tempfile::tempdir().unwrap();
    crash_with_edited_file(rec.path(), &docs.path().join("robots.rcraft"));
    let mut app = app_with_recovery(rec.path());
    let orphan = offered(&app)[0].package.clone();
    actions::apply(
        &mut app,
        Action::ResolveRecovery(RecoveryChoice::DiscardAll),
    );
    wait(&app);
    assert!(app.modal.is_none());
    assert_eq!(app.tabs.len(), 1);
    assert!(!app.tabs[0].is_dirty());
    assert!(!orphan.exists());
    assert!(!orphan.parent().unwrap().exists());
}

#[test]
fn decide_later_keeps_files_for_next_start() {
    let rec = tempfile::tempdir().unwrap();
    let docs = tempfile::tempdir().unwrap();
    crash_with_edited_file(rec.path(), &docs.path().join("robots.rcraft"));
    let mut app = app_with_recovery(rec.path());
    let orphan = offered(&app)[0].package.clone();
    actions::apply(&mut app, Action::ResolveRecovery(RecoveryChoice::Later));
    assert!(app.modal.is_none());
    assert!(orphan.is_file());
    // Still offered by the next start (this one exits normally).
    actions::apply(&mut app, Action::CloseWindow);
    assert!(app.close_confirmed());
    let next = app_with_recovery(rec.path());
    assert_eq!(offered(&next).len(), 1);
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

fn harness(app: ReasonCraftApp) -> Harness<'static, ReasonCraftApp> {
    Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .with_max_steps(32)
        .build_ui_state(|ui, app: &mut ReasonCraftApp| app.show(ui), app)
}

#[test]
fn recovery_prompt_lists_documents_and_restores() {
    let rec = tempfile::tempdir().unwrap();
    let docs = tempfile::tempdir().unwrap();
    crash_with_edited_file(rec.path(), &docs.path().join("robots.rcraft"));
    let mut h = harness(app_with_recovery(rec.path()));
    h.run();
    h.get_by_label(&t("recovery.title"));
    // One label: the message, then a line per document.
    let entry = format!(
        "{}
• robots (",
        t("recovery.message")
    );
    h.get_by_label_contains(&entry);
    h.get_by_label(&t("recovery.discard_all"));
    h.get_by_label(&t("recovery.later"));
    h.get_by_label(&t("recovery.restore_all")).click();
    h.run();
    assert!(h.state().modal.is_none());
    assert_eq!(h.state().active_tab().title(), "robots (recovered) •");
}

#[test]
fn settings_page_sets_the_autosave_interval() {
    let mut h = harness(ReasonCraftApp::new_for_test());
    h.state_mut().ribbon_tab = reasoncraft::app::RibbonTab::File;
    h.run();
    h.get_by_label(&t("backstage.settings")).click();
    h.run();
    h.get_by_label(&t("settings.autosave_interval"));
    h.get_by_role_and_label(egui::accesskit::Role::CheckBox, &t("settings.autosave"))
        .click();
    h.run();
    assert_eq!(h.state().settings.autosave_interval(), None);
    assert!(
        h.query_by_label(&t("settings.autosave_interval")).is_none(),
        "no interval while off"
    );
    h.get_by_role_and_label(egui::accesskit::Role::CheckBox, &t("settings.autosave"))
        .click();
    h.run();
    assert_eq!(h.state().settings.autosave_interval(), Some(45));
    // Older settings without the field autosave every 45 seconds.
    let old: reasoncraft::app::Settings = serde_json::from_str(r#"{"theme":"Light"}"#).unwrap();
    assert_eq!(old.autosave_secs, 45);
}
