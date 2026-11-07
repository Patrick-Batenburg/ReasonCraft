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
