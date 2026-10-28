//! The recovery prompt after a crash: restore, discard, decide later.

use crate::common::{harness, settle};
use std::fs::File;
use std::time::{Duration, SystemTime};

use eframe::egui;
use egui_kittest::kittest::Queryable;
use reasoncraft::actions::{self, Action};
use reasoncraft::file_ops::Modal;
use reasoncraft::i18n::t;
use reasoncraft::recovery::{self, RecoveryChoice};

use super::*;

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
    reasoncraft::file_ops::wait_for_saves(&mut app);
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
    let mut h = harness(app);
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
    h.run_steps(2);
    settle(&mut h);
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
fn recovery_prompt_lists_documents_and_restores() {
    let rec = tempfile::tempdir().unwrap();
    let docs = tempfile::tempdir().unwrap();
    crash_with_edited_file(rec.path(), &docs.path().join("robots.rcraft"));
    let mut h = crate::common::build(app_with_recovery(rec.path()));
    h.run();
    // The heading, and the dialog named after it.
    h.get_by_role_and_label(egui::accesskit::Role::Label, &t("recovery.title"));
    h.get_by_role_and_label(egui::accesskit::Role::Dialog, &t("recovery.title"));
    // One label: the message, then a line per document.
    let entry = format!("{}\n• robots (", t("recovery.message"));
    h.get_by_label_contains(&entry);
    h.get_by_label(&t("recovery.discard_all"));
    h.get_by_label(&t("recovery.later"));
    h.get_by_label(&t("recovery.restore_all")).click();
    h.run();
    assert!(h.state().modal.is_none());
    assert_eq!(h.state().active_tab().title(), "robots (recovered) •");
}

#[test]
fn recovery_prompt_ignores_a_click_beside_it() {
    let rec = tempfile::tempdir().unwrap();
    let docs = tempfile::tempdir().unwrap();
    crash_with_edited_file(rec.path(), &docs.path().join("robots.rcraft"));
    let mut h = harness(app_with_recovery(rec.path()));
    let pos = egui::pos2(10.0, 450.0);
    h.event(egui::Event::PointerMoved(pos));
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run();
    assert!(
        matches!(h.state().modal, Some(Modal::Recover(_))),
        "a stray click (to focus the window) keeps the offer"
    );
    h.key_press(egui::Key::Escape);
    h.run();
    assert!(h.state().modal.is_none(), "Esc puts it off");
}

/// Moves `path`'s modification time a minute ahead, as an edit by someone
/// else would.
fn touch_later(path: &Path) {
    let later = SystemTime::now() + Duration::from_secs(60);
    let file = File::options().write(true).open(path).unwrap();
    file.set_modified(later).unwrap();
}

#[test]
fn file_changed_before_first_autosave_still_asks() {
    let rec = tempfile::tempdir().unwrap();
    let docs = tempfile::tempdir().unwrap();
    let path = docs.path().join("robots.rcraft");
    file::save(&reasoncraft_core::Document::new("Saved"), &path).unwrap();
    let mut app = app_with_recovery(rec.path());
    reasoncraft::file_ops::open_into_app(&mut app, &path);
    set_claim(&mut app, "Edited");
    // The file changes after it was opened, before the autosave.
    touch_later(&path);
    autosave(&mut app, 0.0);
    drop(app);

    let mut app = app_with_recovery(rec.path());
    actions::apply(
        &mut app,
        Action::ResolveRecovery(RecoveryChoice::RestoreAll),
    );
    actions::apply(&mut app, Action::Save);
    reasoncraft::file_ops::wait_for_saves(&mut app);
    assert!(matches!(
        app.modal,
        Some(Modal::ConfirmReplace {
            original_changed: true,
            ..
        })
    ));
    assert_eq!(root_text(&path), "Saved");
}

#[test]
fn command_line_error_does_not_swallow_recovery_prompt() {
    let rec = tempfile::tempdir().unwrap();
    let docs = tempfile::tempdir().unwrap();
    crash_with_edited_file(rec.path(), &docs.path().join("robots.rcraft"));
    let bad = docs.path().join("bad.rcraft");
    std::fs::write(&bad, b"not a map").unwrap();

    // As at start: the app offers recovery, then opens the command line.
    let mut app = app_with_recovery(rec.path());
    reasoncraft::file_ops::open_paths(&mut app, [bad.into_os_string()]);
    assert!(matches!(app.modal, Some(Modal::Error { .. })));
    actions::apply(&mut app, Action::DismissModal);
    recovery::tick(&mut app, 0.0);
    assert_eq!(offered(&app).len(), 1);
}

#[test]
fn restore_keeps_the_orphan_when_its_new_copy_fails() {
    let rec = tempfile::tempdir().unwrap();
    let docs = tempfile::tempdir().unwrap();
    crash_with_edited_file(rec.path(), &docs.path().join("robots.rcraft"));
    let mut app = app_with_recovery(rec.path());
    let orphan = offered(&app)[0].package.clone();
    app.recovery.as_mut().unwrap().set_fail_writes(true);
    actions::apply(
        &mut app,
        Action::ResolveRecovery(RecoveryChoice::RestoreAll),
    );
    wait(&app);
    assert_eq!(
        app.active_tab().doc.boxes[&app.active_tab().doc.root].text,
        "Edited"
    );
    assert!(orphan.is_file(), "the only copy on disk is kept");
    // A crash now still leaves it to restore.
    drop(app);
    let next = app_with_recovery(rec.path());
    assert_eq!(offered(&next).len(), 1);
}

#[test]
fn unreadable_copy_is_named_by_its_document_and_kept() {
    let rec = tempfile::tempdir().unwrap();
    let docs = tempfile::tempdir().unwrap();
    crash_with_edited_file(rec.path(), &docs.path().join("robots.rcraft"));
    let mut app = app_with_recovery(rec.path());
    let orphan = offered(&app)[0].package.clone();
    std::fs::write(&orphan, b"damaged").unwrap();
    actions::apply(
        &mut app,
        Action::ResolveRecovery(RecoveryChoice::RestoreAll),
    );
    wait(&app);
    match &app.modal {
        Some(Modal::Error { title, .. }) => assert!(title.contains("robots"), "{title}"),
        other => panic!("expected an error, got {other:?}"),
    }
    assert!(orphan.is_file());
}

#[test]
fn retry_keeps_then_discard() {
    let rec = tempfile::tempdir().unwrap();
    let docs = tempfile::tempdir().unwrap();
    crash_with_edited_file(rec.path(), &docs.path().join("robots.rcraft"));
    let mut app = app_with_recovery(rec.path());
    let orphan = offered(&app)[0].package.clone();
    app.recovery.as_mut().unwrap().set_fail_writes(true);
    actions::apply(
        &mut app,
        Action::ResolveRecovery(RecoveryChoice::RestoreAll),
    );
    wait(&app);
    recovery::tick(&mut app, 0.0);
    assert!(orphan.is_file());
    // The folder works again: the retried copy still replaces the orphan.
    app.recovery.as_mut().unwrap().set_fail_writes(false);
    recovery::tick(&mut app, 47.0);
    wait(&app);
    assert_eq!(root_text(&recovery_file(&app, 0)), "Edited");
    assert!(
        !orphan.exists(),
        "the orphan went once the retry was written"
    );
    assert!(!orphan.parent().unwrap().exists());
}

/// A copy whose original path is relative (from an older build) cannot
/// tell which folder it meant: it is restored untitled, never saved to
/// wherever the app happens to start.
#[test]
fn relative_original_path_restores_untitled() {
    let rec = tempfile::tempdir().unwrap();
    let docs = tempfile::tempdir().unwrap();
    crash_with_edited_file(rec.path(), &docs.path().join("r.rcraft"));
    let metas: Vec<_> = walk(rec.path())
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    assert_eq!(metas.len(), 1);
    let mut meta: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&metas[0]).unwrap()).unwrap();
    meta["original"] = "r.rcraft".into();
    std::fs::write(&metas[0], meta.to_string()).unwrap();

    let mut app = app_with_recovery(rec.path());
    actions::apply(
        &mut app,
        Action::ResolveRecovery(RecoveryChoice::RestoreAll),
    );
    wait(&app);
    let tab = app.active_tab();
    assert_eq!(tab.doc.boxes[&tab.doc.root].text, "Edited");
    assert_eq!(tab.path, None);
}

fn walk(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(walk(&p));
        } else {
            out.push(p);
        }
    }
    out
}

/// The startup prompt has the keyboard focus on Restore all, so Enter
/// restores (the finding: Enter did nothing and Tab landed on "Decide
/// later") (ROB-7).
#[test]
fn recovery_prompt_enter_restores() {
    let rec = tempfile::tempdir().unwrap();
    let docs = tempfile::tempdir().unwrap();
    crash_with_edited_file(rec.path(), &docs.path().join("robots.rcraft"));
    let mut h = harness(app_with_recovery(rec.path()));
    assert!(matches!(h.state().modal, Some(Modal::Recover(_))));
    h.key_press(egui::Key::Enter);
    h.run();
    assert!(h.state().modal.is_none(), "Enter answered");
    assert!(
        h.state()
            .tabs
            .iter()
            .any(|t| t.doc.boxes[&t.doc.root].text == "Edited"),
        "the crashed document is back"
    );
}
