//! UI tests for images in boxes: the ribbon's Image and Remove image
//! buttons, their errors, preparing images off the UI thread and the
//! missing-asset placeholder.

use crate::common;
use std::path::{Path, PathBuf};
use std::time::Instant;

use egui_kittest::kittest::{NodeT, Queryable};
use reasoncraft::ReasonCraftApp;
use reasoncraft::actions::Action;
use reasoncraft::doc_tab::DocTab;
use reasoncraft::file_ops::{FakeDialogs, Modal};
use reasoncraft::i18n::t;
use reasoncraft::jobs::Hold;
use reasoncraft_core::{CardKind, Document, ImageRef, Item, NodeId, Parent};

use crate::common::{H, settle, step_until};

/// A claim with one premise, which is selected; no building panel.
fn harness(fake: &FakeDialogs) -> (H, NodeId) {
    let mut doc = Document::new("Claim");
    let root = doc.root;
    let (_, premise) = doc
        .add_card(Parent::Box(root), CardKind::Reason, "Premise")
        .unwrap();
    let mut app = ReasonCraftApp::new_for_test_with(Box::new(fake.clone()));
    app.tabs[0] = DocTab::from_document(doc);
    app.tabs[0].selection = Some(Item::Box(premise));
    app.left_panel = false;
    let h = common::harness(app);
    (h, premise)
}

/// A `w`×`h` PNG file in `dir`.
fn photo(dir: &Path, name: &str, w: u32, h: u32) -> PathBuf {
    let file = dir.join(name);
    image::RgbImage::from_pixel(w, h, image::Rgb([200, 40, 40]))
        .save(&file)
        .unwrap();
    file
}

#[test]
fn corrupt_image_shows_error() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("holiday.png");
    std::fs::write(&file, b"\x89PNG\r\n\x1a\n this is not really a picture").unwrap();
    let fake = FakeDialogs::default();
    let (mut h, _) = harness(&fake);
    let before = h.state().active_tab().doc.clone();

    fake.set_image(file);
    h.get_by_label(&t("action.image")).click();
    settle(&mut h);

    match &h.state().modal {
        Some(Modal::Error { title, message }) => {
            assert!(title.contains("holiday.png"), "{title}");
            assert_eq!(message, &t("image.error.corrupt"));
        }
        other => panic!("expected an error modal, got {other:?}"),
    }
    h.get_by_label(&t("image.error.corrupt"));
    let tab = h.state().active_tab();
    assert_eq!(tab.doc, before, "the document is unchanged");
    assert!(tab.history.undo.is_empty());
    assert!(tab.assets.is_empty());
}

#[test]
fn image_button_inserts_and_remove_button_removes() {
    let dir = tempfile::tempdir().unwrap();
    let file = photo(dir.path(), "photo.png", 40, 20);
    let fake = FakeDialogs::default();
    let (mut h, premise) = harness(&fake);
    assert!(
        h.get_by_label(&t("action.remove_image"))
            .accesskit_node()
            .is_disabled(),
        "nothing to remove yet"
    );

    fake.set_image(file);
    h.get_by_label(&t("action.image")).click();
    settle(&mut h);
    assert!(h.state().modal.is_none());
    let image = h.state().active_tab().doc.boxes[&premise].image.clone();
    assert!(image.is_some_and(|i| (i.aspect - 0.5).abs() < 1e-6));
    assert_eq!(h.state().images.len(), 1, "its texture is loaded");

    h.get_by_label(&t("action.remove_image")).click();
    h.run();
    assert!(h.state().active_tab().doc.boxes[&premise].image.is_none());
}

#[test]
fn missing_asset_shows_placeholder() {
    let fake = FakeDialogs::default();
    let (mut h, premise) = harness(&fake);
    let image = ImageRef {
        asset: "0000.png".into(),
        aspect: 0.5,
    };
    h.state_mut().tabs[0]
        .doc
        .set_image(premise, Some(image))
        .unwrap();
    h.run();
    // Painted without panicking; the cache marks the asset as missing.
    assert!(h.state().images.texture("0000.png").is_none());
}

#[test]
fn image_insert_runs_off_ui_thread() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDialogs::default();
    let (mut h, premise) = harness(&fake);
    let hold = Hold::default();
    h.state_mut().image_jobs.hold = Some(hold.clone());
    let steps = h.state().active_tab().history.undo.len();
    fake.set_image(photo(dir.path(), "slow.png", 40, 20));
    h.get_by_label(&t("action.image")).click();
    // Frames keep coming while the image is prepared (a frame that
    // waited for it would wait until the hold gives up).
    let start = Instant::now();
    for _ in 0..6 {
        h.step();
    }
    assert!(start.elapsed() < Hold::MAX_WAIT / 2, "a frame waited");
    assert!(h.state().image_jobs.busy());
    assert!(h.state().active_tab().pending_images.contains_key(&premise));
    assert!(h.state().active_tab().doc.boxes[&premise].image.is_none());
    // The box shows a placeholder; the status bar shows the busy chip
    // (both read "Preparing image…").
    assert_eq!(t("image.preparing"), t("status.preparing_image"));
    assert_eq!(h.get_all_by_label(&t("image.preparing")).count(), 2);

    hold.release();
    settle(&mut h);
    let tab = h.state().active_tab();
    assert!(tab.doc.boxes[&premise].image.is_some());
    assert_eq!(tab.history.undo.len(), steps + 1, "one history step");
    assert!(tab.pending_images.is_empty());
    assert_eq!(h.query_all_by_label(&t("image.preparing")).count(), 0);
}

#[test]
fn image_prepared_for_deleted_box_is_discarded() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDialogs::default();
    let (mut h, premise) = harness(&fake);
    let hold = Hold::default();
    h.state_mut().image_jobs.hold = Some(hold.clone());
    fake.set_image(photo(dir.path(), "late.png", 40, 20));
    h.get_by_label(&t("action.image")).click();
    h.step();
    assert!(h.state().image_jobs.busy());
    h.state_mut().push(Action::Delete);
    h.step();
    assert!(!h.state().active_tab().doc.boxes.contains_key(&premise));
    let steps = h.state().active_tab().history.undo.len();

    hold.release();
    settle(&mut h);
    let app = h.state();
    let tab = app.active_tab();
    assert!(tab.doc.boxes.values().all(|b| b.image.is_none()));
    assert!(tab.assets.is_empty(), "nothing stored");
    assert_eq!(tab.history.undo.len(), steps, "no history step");
    assert!(app.modal.is_none());
    assert_eq!(
        app.status_note.as_deref(),
        Some(t("image.discarded").as_str())
    );
    h.get_by_label(&t("image.discarded"));

    // Undoing the delete brings the box back, without the image.
    h.state_mut().push(Action::Undo);
    h.run();
    assert!(h.state().active_tab().doc.boxes[&premise].image.is_none());
}

#[test]
fn second_insert_for_the_same_box_replaces_the_first() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDialogs::default();
    let (mut h, premise) = harness(&fake);
    let hold = Hold::default();
    h.state_mut().image_jobs.hold = Some(hold.clone());
    let steps = h.state().active_tab().history.undo.len();
    fake.set_image(photo(dir.path(), "wide.png", 40, 20));
    h.get_by_label(&t("action.image")).click();
    h.step();
    // Images are prepared one at a time: the second waits for the first,
    // whose result arrives first and is dropped.
    fake.set_image(photo(dir.path(), "tall.png", 20, 40));
    h.get_by_label(&t("action.image")).click();
    h.step();
    assert_eq!(h.state().image_jobs.running(), 2);
    hold.release();
    settle(&mut h);
    let tab = h.state().active_tab();
    let image = tab.doc.boxes[&premise].image.clone().unwrap();
    assert!((image.aspect - 2.0).abs() < 1e-6, "the newer image wins");
    assert_eq!(tab.history.undo.len(), steps + 1, "one history step");
    assert_eq!(tab.assets.len(), 1, "the older result was dropped");
}

#[test]
fn prepared_image_waits_for_the_running_edit() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDialogs::default();
    let (mut h, premise) = harness(&fake);
    let hold = Hold::default();
    h.state_mut().image_jobs.hold = Some(hold.clone());
    fake.set_image(photo(dir.path(), "photo.png", 40, 20));
    h.get_by_label(&t("action.image")).click();
    h.step();
    h.state_mut().push(Action::Edit);
    h.step();
    assert_eq!(h.state().active_tab().editing(), Some(premise));
    hold.release();

    step_until(&mut h, |app| !app.image_jobs.busy(), "the image");
    h.step();
    let tab = h.state().active_tab();
    assert_eq!(tab.editing(), Some(premise), "the edit keeps running");
    assert!(tab.doc.boxes[&premise].image.is_none(), "the image waits");
    // While it is held, the box's placeholder and the busy chip both show.
    assert_eq!(t("image.preparing"), t("status.preparing_image"));
    assert_eq!(
        h.query_all_by_label(&t("image.preparing")).count(),
        2,
        "placeholder and busy chip"
    );

    h.state_mut().push(Action::StopEditing);
    settle(&mut h);
    let tab = h.state().active_tab();
    assert!(tab.doc.boxes[&premise].image.is_some());
    assert!(tab.pending_images.is_empty());
}
