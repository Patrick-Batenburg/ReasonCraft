//! Restoring a document whose file is open already.

use reasoncraft::actions::{self, Action};
use reasoncraft::file_ops::open_into_app;
use reasoncraft::recovery::RecoveryChoice;

use super::*;

#[test]
fn restore_takes_the_place_of_its_open_file() {
    let rec = tempfile::tempdir().unwrap();
    let docs = tempfile::tempdir().unwrap();
    let path = docs.path().join("robots.rcraft");
    crash_with_edited_file(rec.path(), &path);
    let mut app = app_with_recovery(rec.path());
    // Opened from the command line while the prompt shows, then a new tab.
    open_into_app(&mut app, &path);
    actions::apply(&mut app, Action::New);
    assert_eq!(app.tabs.len(), 2);
    actions::apply(
        &mut app,
        Action::ResolveRecovery(RecoveryChoice::RestoreAll),
    );
    wait(&app);
    assert_eq!(app.tabs.len(), 2, "no second tab for the same file");
    assert_eq!(app.active, 0);
    let tab = app.active_tab();
    assert_eq!(tab.path.as_deref(), Some(path.as_path()));
    assert_eq!(tab.doc.boxes[&tab.doc.root].text, "Edited");
    assert!(tab.recovered_from.is_some());
}

#[test]
fn a_restored_copy_that_was_cut_opens_as_an_unsaved_copy() {
    let rec = tempfile::tempdir().unwrap();
    let docs = tempfile::tempdir().unwrap();
    let path = docs.path().join("long.rcraft");
    file::save(&reasoncraft_core::Document::new("Saved"), &path).unwrap();
    let mut app = app_with_recovery(rec.path());
    open_into_app(&mut app, &path);
    // More text than opening keeps: the map cannot be saved, but its
    // recovery copy is written all the same.
    let big = "word ".repeat(10_000);
    app.tabs[0]
        .edit(|d| {
            for _ in 0..161 {
                d.add_snippet(&big);
            }
            Ok::<_, reasoncraft_core::EditError>(())
        })
        .unwrap();
    autosave(&mut app, 0.0);
    drop(app);

    let mut app = app_with_recovery(rec.path());
    actions::apply(
        &mut app,
        Action::ResolveRecovery(RecoveryChoice::RestoreAll),
    );
    wait(&app);
    let tab = app.active_tab();
    assert_eq!(tab.path, None, "a save never overwrites the file unasked");
    assert!(tab.is_dirty());
    assert_eq!(
        app.status_note,
        Some(reasoncraft::i18n::t("status.open_cut"))
    );
    assert_eq!(super::root_text(&path), "Saved");
}
