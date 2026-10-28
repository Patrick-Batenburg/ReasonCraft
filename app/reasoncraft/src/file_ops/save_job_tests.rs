//! Saves written on a worker: the window goes on meanwhile, the tab counts
//! as saved only for what was written, and a close waits for the result.

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
    assert!(app.saves.saving(&app.tabs[0]));
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
    assert_eq!(app.settings.recent_files, [path]);
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
    assert!(matches!(app.modal, Some(Modal::Unsaved { tab: 0, .. })));
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
