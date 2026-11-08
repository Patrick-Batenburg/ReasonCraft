//! Images in maps: inserting, undoing, saving and opening them, and the
//! package entries the app keeps.

use std::path::Path;

use reasoncraft_core::file;

use super::*;
use crate::actions::{Action, apply, is_enabled, wait_for_images};
use reasoncraft_core::{NodeId, Package};

/// A noisy `w`×`h` PNG: the noise keeps it from compressing to almost nothing.
fn noisy_png(w: u32, h: u32) -> Vec<u8> {
    let img = image::RgbImage::from_fn(w, h, |x, y| {
        let v =
            (x.wrapping_mul(2_654_435_761) ^ y.wrapping_mul(40_503)).wrapping_mul(2_246_822_519);
        image::Rgb([v as u8, (v >> 8) as u8, (v >> 16) as u8])
    });
    let mut out = Vec::new();
    image::DynamicImage::ImageRgb8(img)
        .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .unwrap();
    out
}

/// An app with a new premise selected and a 240×180 PNG in `dir` queued
/// in the image dialog; returns the premise.
fn app_with_premise(dir: &Path, fake: &FakeDialogs) -> (ReasonCraftApp, NodeId) {
    let mut app = ReasonCraftApp::new_for_test_with(Box::new(fake.clone()));
    apply(&mut app, Action::AddReason);
    let premise = app.active_tab().editing().unwrap();
    apply(&mut app, Action::StopEditing);
    let file = dir.join("photo.png");
    std::fs::write(&file, noisy_png(240, 180)).unwrap();
    fake.set_image(file);
    (app, premise)
}

#[test]
fn image_roundtrip_through_save() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDialogs::default();
    let (mut app, premise) = app_with_premise(dir.path(), &fake);
    apply(&mut app, Action::InsertImage);
    wait_for_images(&mut app);
    assert!(app.modal.is_none(), "{:?}", app.modal);
    let image = app.active_tab().doc.boxes[&premise].image.clone().unwrap();
    assert!((image.aspect - 0.75).abs() < 1e-6);
    let bytes = app.active_tab().assets.get(&image.asset).unwrap();

    let path = dir.path().join("pic.rcraft");
    fake.set_save(path.clone());
    apply(&mut app, Action::Save);
    crate::file_ops::wait_for_saves(&mut app);
    assert!(app.modal.is_none(), "{:?}", app.modal);
    apply(&mut app, Action::CloseTab(0));
    fake.set_open(path);
    apply(&mut app, Action::Open);
    assert!(app.modal.is_none(), "{:?}", app.modal);
    let tab = app.active_tab();
    assert_eq!(tab.doc.boxes[&premise].image.as_ref(), Some(&image));
    assert_eq!(tab.assets.get(&image.asset).unwrap(), bytes);
    assert_eq!(tab.assets.len(), 1);
}

#[test]
fn undo_image_insert_keeps_assets_out_of_history() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDialogs::default();
    let (mut app, premise) = app_with_premise(dir.path(), &fake);
    apply(&mut app, Action::InsertImage);
    wait_for_images(&mut app);
    let key = app.active_tab().doc.boxes[&premise]
        .image
        .clone()
        .unwrap()
        .asset;
    let asset_len = app.active_tab().assets.get(&key).unwrap().len();
    assert!(
        asset_len > 50_000,
        "the test image must be big: {asset_len}"
    );
    apply(&mut app, Action::Undo);

    let tab = app.active_tab();
    assert!(tab.doc.boxes[&premise].image.is_none());
    // `Document` has no byte fields: every snapshot is tiny next to the image.
    let snapshots = tab.history.undo.iter().chain(&tab.history.redo);
    for doc in snapshots.chain([&tab.doc]) {
        let json = serde_json::to_vec(doc).unwrap();
        assert!(json.len() < 4096, "a snapshot holds {} bytes", json.len());
    }
    assert_eq!(tab.history.redo.len(), 1, "the insert can be redone");
    assert!(tab.assets.get(&key).is_some(), "assets are add-only");

    let path = dir.path().join("undone.rcraft");
    fake.set_save(path.clone());
    apply(&mut app, Action::Save);
    crate::file_ops::wait_for_saves(&mut app);
    assert!(app.modal.is_none(), "{:?}", app.modal);
    let pkg = file::open_package(&path).unwrap();
    assert!(
        pkg.assets.is_empty(),
        "no images/ entry for an unused image"
    );
    assert!(
        pkg.other_entries
            .iter()
            .all(|(n, _)| !n.starts_with("images/"))
    );
    // A redo after the save still finds the bytes.
    apply(&mut app, Action::Redo);
    let tab = app.active_tab();
    assert_eq!(tab.doc.boxes[&premise].image.as_ref().unwrap().asset, key);
    assert!(tab.assets.get(&key).is_some());
}

#[test]
fn unknown_zip_entries_survive_open_and_save_in_the_app() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("newer.rcraft");
    let mut pkg = Package::new(reasoncraft_core::Document::new("Claim"));
    pkg.other_entries
        .push(("thumbnail.png".into(), vec![1, 2, 3, 4].into()));
    file::save_package(&pkg, &path).unwrap();

    let mut app = ReasonCraftApp::new_for_test();
    open_into_app(&mut app, &path);
    apply(&mut app, Action::AddReason);
    apply(&mut app, Action::Save);
    crate::file_ops::wait_for_saves(&mut app);
    assert!(app.modal.is_none(), "{:?}", app.modal);
    let saved = file::open_package(&path).unwrap();
    assert_eq!(saved.doc.boxes.len(), 2, "the edit was saved");
    assert_eq!(
        saved.other_entries,
        vec![("thumbnail.png".to_owned(), vec![1, 2, 3, 4].into())]
    );
}

#[test]
fn image_actions_need_a_box_that_can_hold_an_image() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDialogs::default();
    let (mut app, premise) = app_with_premise(dir.path(), &fake);
    let enabled = |app: &ReasonCraftApp, a: &Action| is_enabled(app.active_tab(), a);
    assert!(enabled(&app, &Action::InsertImage));
    assert!(
        !enabled(&app, &Action::RemoveImage),
        "nothing to remove yet"
    );
    apply(&mut app, Action::InsertImage);
    wait_for_images(&mut app);
    assert!(enabled(&app, &Action::RemoveImage));
    apply(&mut app, Action::RemoveImage);
    assert!(app.active_tab().doc.boxes[&premise].image.is_none());
    assert_eq!(
        app.active_tab().history.undo.len(),
        3,
        "add, insert, remove"
    );

    apply(&mut app, Action::AddNote);
    apply(&mut app, Action::StopEditing);
    assert!(!enabled(&app, &Action::InsertImage), "notes hold no image");
    apply(&mut app, Action::Select(None));
    assert!(!enabled(&app, &Action::InsertImage));
}

#[test]
fn image_actions_commit_a_running_text_edit_first() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDialogs::default();
    let (mut app, premise) = app_with_premise(dir.path(), &fake);
    for action in [Action::InsertImage, Action::RemoveImage] {
        let text = format!("typed before {action:?}");
        apply(&mut app, Action::Edit);
        apply(
            &mut app,
            Action::EditText {
                node: premise,
                text: text.clone(),
            },
        );
        let steps = app.active_tab().history.undo.len();
        apply(&mut app, action);
        wait_for_images(&mut app);
        let tab = app.active_tab();
        assert_eq!(tab.editing(), None, "the edit ended");
        assert_eq!(
            tab.history.undo.len(),
            steps + 2,
            "text step, then image step"
        );
        assert_eq!(tab.doc.boxes[&premise].text, text);
    }
    // Undoing the image step leaves the committed text in place.
    apply(&mut app, Action::Undo);
    let node = &app.active_tab().doc.boxes[&premise];
    assert!(node.image.is_some());
    assert_eq!(node.text, "typed before RemoveImage");
}
