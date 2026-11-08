//! Saves written on a worker: the window goes on meanwhile, the tab counts
//! as saved only for what was written, and a close waits for the result.

use std::path::Path;

use reasoncraft_core::file;

use super::*;
use crate::actions::{Action, apply};
use crate::jobs::Hold;

/// An app with one edited tab, whose saves are held until released, and
/// the file Save writes to.
fn held_app(dir: &Path) -> (ReasonCraftApp, Hold, PathBuf) {
    let fake = FakeDialogs::default();
    let mut app = ReasonCraftApp::new_for_test_with(Box::new(fake.clone()));
    let hold = Hold::default();
    app.saves.hold = Some(hold.clone());
    app.tabs[0].edit(|d| d.set_text(d.root, "Written")).unwrap();
    let path = dir.join("held.rcraft");
    fake.set_save(path.clone());
    (app, hold, path)
}

fn root_text(path: &Path) -> String {
    let doc = file::open(path).unwrap();
    doc.boxes[&doc.root].text.clone()
}

#[test]
fn save_is_written_on_a_worker_and_marks_the_tab_saved_when_done() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, hold, path) = held_app(dir.path());
    apply(&mut app, Action::Save);
    // Frames go on while the file is written; nothing has changed yet.
    for _ in 0..3 {
        poll_saves(&mut app);
    }
    assert!(app.saves.busy());
    assert_eq!(app.saves.pending_path(&app.tabs[0]), Some(path.as_path()));
    assert!(app.tabs[0].is_dirty());
    assert_eq!(app.tabs[0].path, None);
    assert!(!path.exists());
    // The status bar shows it.
    let mut h = egui_kittest::Harness::new_ui_state(
        |ui, app: &mut ReasonCraftApp| {
            crate::shell::statusbar::show(ui, app);
        },
        app,
    );
    h.run_steps(2);
    egui_kittest::kittest::Queryable::get_by_label(&h, &crate::i18n::t("status.saving"));
    let mut app = h.into_state();
    hold.release();
    wait_for_saves(&mut app);
    assert!(!app.tabs[0].is_dirty());
    assert_eq!(app.tabs[0].path.as_deref(), Some(path.as_path()));
    assert_eq!(root_text(&path), "Written");
    // Stored resolved: macOS's temporary folder is behind a link.
    let recent: Vec<_> = app
        .settings
        .recent_files
        .iter()
        .map(|p| p.canonicalize().unwrap())
        .collect();
    assert_eq!(recent, [path.canonicalize().unwrap()]);
}

#[test]
fn edits_made_while_saving_keep_the_tab_unsaved() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, hold, path) = held_app(dir.path());
    apply(&mut app, Action::Save);
    app.tabs[0].edit(|d| d.set_text(d.root, "Later")).unwrap();
    hold.release();
    wait_for_saves(&mut app);
    let tab = &mut app.tabs[0];
    assert_eq!(root_text(&path), "Written", "what was there when Save ran");
    assert_eq!(tab.path.as_deref(), Some(path.as_path()));
    assert!(tab.is_dirty(), "the later edit is not in the file");
    // Not even undone back to what was written: no saved position is
    // known in the history any more.
    tab.undo();
    assert!(tab.is_dirty());
}

#[test]
fn a_close_waits_for_its_save_and_asks_again_after_new_edits() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, hold, path) = held_app(dir.path());
    app.tabs.push(DocTab::new());
    apply(&mut app, Action::CloseTab(0));
    apply(&mut app, Action::ResolveUnsaved(UnsavedChoice::Save));
    // The tab stays open until its file is written.
    assert_eq!(app.tabs.len(), 2);
    assert!(app.modal.is_none());
    poll_saves(&mut app);
    assert_eq!(app.tabs.len(), 2);
    hold.release();
    wait_for_saves(&mut app);
    assert_eq!(app.tabs.len(), 1, "closed once saved");
    assert_eq!(root_text(&path), "Written");

    // Edited while its save was written: the close asks again.
    let (mut app, hold, _) = held_app(dir.path());
    app.tabs.push(DocTab::new());
    apply(&mut app, Action::CloseTab(0));
    apply(&mut app, Action::ResolveUnsaved(UnsavedChoice::Save));
    app.tabs[0].edit(|d| d.set_text(d.root, "Later")).unwrap();
    hold.release();
    wait_for_saves(&mut app);
    assert_eq!(app.tabs.len(), 2, "not closed");
    let key = app.tabs[0].recovery_key;
    assert!(matches!(app.modal, Some(Modal::Unsaved { tab, .. }) if tab == key));
}

#[test]
fn a_close_request_lets_running_saves_finish_first() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, hold, _) = held_app(dir.path());
    apply(&mut app, Action::Save);
    hold.release();
    // The save is written before the close looks for unsaved work, so the
    // saved tab closes without asking.
    apply(&mut app, Action::CloseTab(0));
    assert!(app.modal.is_none());
    assert!(app.tabs[0].path.is_none(), "a new blank tab");
}

/// A tab saved to `path` with root text `text` (written at once).
fn saved_tab(fake: &FakeDialogs, path: &Path, text: &str) -> DocTab {
    let mut tab = DocTab::new();
    tab.edit(|d| d.set_text(d.root, text)).unwrap();
    fake.set_save(path.to_path_buf());
    assert_eq!(save_tab(&mut tab, fake, false).unwrap(), SaveOutcome::Saved);
    tab
}

#[test]
fn save_during_a_save_as_writes_where_the_save_as_goes() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = (dir.path().join("a.rcraft"), dir.path().join("b.rcraft"));
    let fake = FakeDialogs::default();
    let mut app = ReasonCraftApp::new_for_test_with(Box::new(fake.clone()));
    let hold = Hold::default();
    app.saves.hold = Some(hold.clone());
    app.tabs[0] = saved_tab(&fake, &a, "Original");
    fake.set_save(b.clone());
    apply(&mut app, Action::SaveAs);
    app.tabs[0].edit(|d| d.set_text(d.root, "Newer")).unwrap();
    apply(&mut app, Action::Save);
    assert!(app.modal.is_none(), "{:?}", app.modal);
    hold.release();
    wait_for_saves(&mut app);
    assert_eq!(root_text(&a), "Original", "the old file is left alone");
    assert_eq!(root_text(&b), "Newer");
    assert_eq!(app.tabs[0].path.as_deref(), Some(b.as_path()));
    assert!(!app.tabs[0].is_dirty());
}

/// Three tabs with root texts A, B and C, all edited, saves held.
fn three_dirty_tabs(fake: &FakeDialogs) -> (ReasonCraftApp, Hold) {
    let mut app = ReasonCraftApp::new_for_test_with(Box::new(fake.clone()));
    let hold = Hold::default();
    app.saves.hold = Some(hold.clone());
    app.tabs = ["A", "B", "C"]
        .into_iter()
        .map(|text| {
            let mut tab = DocTab::new();
            tab.edit(|d| d.set_text(d.root, text)).unwrap();
            tab
        })
        .collect();
    (app, hold)
}

#[test]
fn closing_a_tab_while_an_earlier_tab_saves_closes_that_tab() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDialogs::default();
    let (mut app, hold) = three_dirty_tabs(&fake);
    app.tabs[1].history.mark_saved();
    fake.set_save(dir.path().join("a.rcraft"));
    apply(&mut app, Action::CloseTab(0));
    apply(&mut app, Action::ResolveUnsaved(UnsavedChoice::Save));
    hold.release();
    // A's save ends while B's close waits for it: A goes, then B.
    apply(&mut app, Action::CloseTab(1));
    let left: Vec<_> = app
        .tabs
        .iter()
        .map(|t| t.doc.boxes[&t.doc.root].text.clone())
        .collect();
    assert_eq!(left, ["C"]);
}

#[test]
fn a_replace_prompt_answers_for_its_own_tab_after_an_earlier_tab_closed() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDialogs::default();
    let (mut app, hold) = three_dirty_tabs(&fake);
    fake.set_save(dir.path().join("a.rcraft"));
    apply(&mut app, Action::CloseTab(0));
    apply(&mut app, Action::ResolveUnsaved(UnsavedChoice::Save));
    // B is saved as "x", and x.rcraft exists: the replace prompt.
    let x = dir.path().join("x.rcraft");
    std::fs::write(&x, b"old").unwrap();
    fake.set_save(dir.path().join("x"));
    apply(&mut app, Action::ActivateTab(1));
    apply(&mut app, Action::SaveAs);
    assert!(matches!(app.modal, Some(Modal::ConfirmReplace { .. })));
    // A's save ends behind the prompt; the prompt stays.
    hold.release();
    wait_for_saves(&mut app);
    poll_saves(&mut app);
    assert!(matches!(app.modal, Some(Modal::ConfirmReplace { .. })));
    apply(&mut app, Action::ResolveReplace(true));
    wait_for_saves(&mut app);
    assert_eq!(root_text(&x), "B");
    // A's close goes on once the prompt is answered.
    poll_saves(&mut app);
    let left: Vec<_> = app
        .tabs
        .iter()
        .map(|t| t.doc.boxes[&t.doc.root].text.clone())
        .collect();
    assert_eq!(left, ["B", "C"]);
    assert_eq!(app.tabs[0].path.as_deref(), Some(x.as_path()));
    assert_eq!(app.tabs[1].path, None);
}

/// The root texts of the tabs still open.
fn tab_texts(app: &ReasonCraftApp) -> Vec<String> {
    app.tabs
        .iter()
        .map(|t| t.doc.boxes[&t.doc.root].text.clone())
        .collect()
}

#[test]
fn a_close_asked_while_a_save_fails_waits_behind_its_error() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDialogs::default();
    let (mut app, hold) = three_dirty_tabs(&fake);
    fake.set_save(dir.path().join("missing").join("a.rcraft"));
    apply(&mut app, Action::CloseTab(0));
    apply(&mut app, Action::ResolveUnsaved(UnsavedChoice::Save));
    hold.release();
    // A's save fails while B's close waits for it: its error shows, and
    // B is asked about once that is closed.
    apply(&mut app, Action::CloseTab(1));
    assert!(
        matches!(app.modal, Some(Modal::Error { .. })),
        "{:?}",
        app.modal
    );
    apply(&mut app, Action::DismissModal);
    poll_saves(&mut app);
    let b = app.tabs[1].recovery_key;
    assert!(
        matches!(app.modal, Some(Modal::Unsaved { tab, .. }) if tab == b),
        "{:?}",
        app.modal
    );
    assert_eq!(tab_texts(&app), ["A", "B", "C"]);
}

#[test]
fn a_window_close_asked_while_a_save_fails_waits_behind_its_error() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDialogs::default();
    let (mut app, hold) = three_dirty_tabs(&fake);
    fake.set_save(dir.path().join("missing").join("a.rcraft"));
    apply(&mut app, Action::CloseTab(0));
    apply(&mut app, Action::ResolveUnsaved(UnsavedChoice::Save));
    hold.release();
    apply(&mut app, Action::CloseWindow);
    assert!(
        matches!(app.modal, Some(Modal::Error { .. })),
        "{:?}",
        app.modal
    );
    apply(&mut app, Action::DismissModal);
    poll_saves(&mut app);
    let a = app.tabs[0].recovery_key;
    assert!(
        matches!(app.modal, Some(Modal::Unsaved { tab, window: true, .. }) if tab == a),
        "{:?}",
        app.modal
    );
}

/// What a finished save was part of waits while the export dialog or the
/// link editor is open too, not only a message (ruling R1-2).
#[test]
fn a_close_after_save_waits_while_the_export_dialog_is_open() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDialogs::default();
    let (mut app, hold) = three_dirty_tabs(&fake);
    fake.set_save(dir.path().join("a.rcraft"));
    apply(&mut app, Action::CloseTab(0));
    apply(&mut app, Action::ResolveUnsaved(UnsavedChoice::Save));
    apply(
        &mut app,
        Action::OpenExport(crate::export::ExportFormat::Png),
    );
    assert!(app.export_dialog.is_some());
    hold.release();
    wait_for_saves(&mut app);
    poll_saves(&mut app);
    assert_eq!(
        tab_texts(&app),
        ["A", "B", "C"],
        "A stays while Export is open"
    );
    assert!(app.export_dialog.is_some());
    app.export_dialog = None;
    poll_saves(&mut app);
    assert_eq!(tab_texts(&app), ["B", "C"]);
}
