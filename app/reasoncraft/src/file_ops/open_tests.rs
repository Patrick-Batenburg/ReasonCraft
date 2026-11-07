//! Opening into a blank tab, and saves over a file changed on disk.

use std::fs::File;
use std::time::{Duration, SystemTime};

use super::*;
use crate::actions::{Action, apply};
use crate::jobs::JobId;
use reasoncraft_core::Document;

/// Moves `path`'s modification time a minute ahead, as a save by another
/// app would.
fn touch_later(path: &Path) {
    let later = SystemTime::now() + Duration::from_secs(60);
    let file = File::options().write(true).open(path).unwrap();
    file.set_modified(later).unwrap();
}

fn root_text(path: &Path) -> String {
    let doc = file::open_package(path).unwrap().doc;
    doc.boxes[&doc.root].text.clone()
}

#[test]
fn open_keeps_a_blank_tab_with_an_image_coming() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.rcraft");
    file::save(&Document::new("A"), &a).unwrap();
    let mut app = ReasonCraftApp::new_for_test();
    let root = app.tabs[0].doc.root;
    app.tabs[0].pending_images.insert(root, JobId::next());
    open_into_app(&mut app, &a);
    assert_eq!(app.tabs.len(), 2, "the image being prepared is kept");
    assert_eq!(app.active, 1);
    assert!(!app.tabs[0].pending_images.is_empty());
}

#[test]
fn save_asks_when_the_file_changed_on_disk() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.rcraft");
    file::save(&Document::new("Theirs"), &path).unwrap();
    let mut app = ReasonCraftApp::new_for_test();
    open_into_app(&mut app, &path);
    let tab = &mut app.tabs[0];
    tab.edit(|d| d.set_text(d.root, "Mine")).unwrap();
    // Another app saves the file meanwhile.
    touch_later(&path);
    apply(&mut app, Action::Save);
    crate::file_ops::wait_for_saves(&mut app);
    assert!(matches!(
        app.modal,
        Some(Modal::ConfirmReplace {
            original_changed: true,
            ..
        })
    ));
    assert_eq!(root_text(&path), "Theirs", "nothing written yet");
    apply(&mut app, Action::ResolveReplace(true));
    crate::file_ops::wait_for_saves(&mut app);
    assert_eq!(root_text(&path), "Mine");
    assert!(!app.tabs[0].is_dirty());
    // Saved: the next save no longer asks.
    let tab = &mut app.tabs[0];
    tab.edit(|d| d.set_text(d.root, "Mine again")).unwrap();
    apply(&mut app, Action::Save);
    crate::file_ops::wait_for_saves(&mut app);
    assert!(app.modal.is_none());
    assert_eq!(root_text(&path), "Mine again");
}

#[test]
fn a_map_too_long_to_open_whole_stays_unsaved() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("long.rcraft");
    let mut tab = DocTab::new();
    tab.path = Some(path.clone());
    let big = "word ".repeat(10_000);
    tab.edit(|d| {
        for _ in 0..161 {
            d.add_snippet(&big);
        }
        Ok::<_, reasoncraft_core::EditError>(())
    })
    .unwrap();
    let err = save_tab(&mut tab, &FakeDialogs::default(), false).unwrap_err();
    assert!(matches!(err, FileError::TooMuchText { .. }), "{err:?}");
    assert!(!path.exists());
    assert!(tab.is_dirty());
    assert_eq!(tab.path, Some(path));
}

/// UX-9: whether the file changed on disk is asked with the write, on the
/// worker, not on the UI thread before it (a slow network share would
/// freeze the window).
#[test]
fn the_changed_on_disk_check_runs_on_the_worker() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.rcraft");
    file::save(&Document::new("Theirs"), &path).unwrap();
    let mut app = ReasonCraftApp::new_for_test();
    open_into_app(&mut app, &path);
    app.tabs[0].edit(|d| d.set_text(d.root, "Mine")).unwrap();
    touch_later(&path);
    let hold = crate::jobs::Hold::default();
    app.saves.hold = Some(hold.clone());
    apply(&mut app, Action::CloseTab(0));
    apply(
        &mut app,
        Action::ResolveUnsaved(crate::actions::UnsavedChoice::Save),
    );
    assert!(
        app.modal.is_none(),
        "the file was looked at on the UI thread"
    );
    assert!(app.saves.busy());
    hold.release();
    wait_for_saves(&mut app);
    let Some(Modal::ConfirmReplace {
        original_changed: true,
        then: AfterSave::CloseTab,
        ..
    }) = app.modal
    else {
        panic!("{:?}", app.modal);
    };
    assert_eq!(root_text(&path), "Theirs", "nothing written yet");
    apply(&mut app, Action::ResolveReplace(true));
    wait_for_saves(&mut app);
    assert_eq!(root_text(&path), "Mine");
    assert!(app.tabs[0].path.is_none(), "the tab closed after the save");
}

/// A tab with root text `text`, saved to `path` (written at once).
fn tab_saved_as(app: &mut ReasonCraftApp, fake: &FakeDialogs, path: &Path, text: &str) {
    apply(app, Action::New);
    let tab = app.tabs.last_mut().unwrap();
    tab.edit(|d| d.set_text(d.root, text)).unwrap();
    fake.set_save(path.to_path_buf());
    apply(app, Action::SaveAs);
    wait_for_saves(app);
    assert!(app.modal.is_none(), "{:?}", app.modal);
}

fn shown_text(app: &ReasonCraftApp) -> String {
    let doc = &app.active_tab().doc;
    doc.boxes[&doc.root].text.clone()
}

/// APP-2: Save As onto the file another tab shows leaves one tab with that
/// path: the other one closes when it has no unsaved work.
#[test]
fn save_as_onto_a_file_open_in_a_clean_tab_closes_that_tab() {
    let dir = tempfile::tempdir().unwrap();
    let x = dir.path().join("x.rcraft");
    let fake = FakeDialogs::default();
    let mut app = ReasonCraftApp::new_for_test_with(Box::new(fake.clone()));
    tab_saved_as(&mut app, &fake, &x, "Old");
    tab_saved_as(&mut app, &fake, &x, "New");
    let with_x = app.tabs.iter().filter(|t| t.path.is_some()).count();
    assert_eq!(with_x, 1, "two tabs show x");
    apply(&mut app, Action::ActivateTab(0));
    fake.set_open(x.clone());
    apply(&mut app, Action::Open);
    assert_eq!(shown_text(&app), "New", "Open of x shows a stale tab");
}

/// …and keeps it, untitled and unsaved, when it has unsaved work.
#[test]
fn save_as_onto_a_file_open_in_a_dirty_tab_makes_that_tab_untitled() {
    let dir = tempfile::tempdir().unwrap();
    let x = dir.path().join("x.rcraft");
    let fake = FakeDialogs::default();
    let mut app = ReasonCraftApp::new_for_test_with(Box::new(fake.clone()));
    tab_saved_as(&mut app, &fake, &x, "Old");
    let old = app.active;
    app.tabs[old]
        .edit(|d| d.set_text(d.root, "Old, edited"))
        .unwrap();
    let old_key = app.tabs[old].recovery_key;
    tab_saved_as(&mut app, &fake, &x, "New");
    let old = tab_index(&app, old_key).expect("the tab with unsaved work stays");
    assert_eq!(app.tabs[old].path, None);
    assert!(app.tabs[old].is_dirty());
    let with_x: Vec<_> = app
        .tabs
        .iter()
        .filter(|t| t.path.as_deref() == Some(&x))
        .collect();
    assert_eq!(with_x.len(), 1);
}
