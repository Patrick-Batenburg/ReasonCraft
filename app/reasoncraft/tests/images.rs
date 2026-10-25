//! UI tests for images in boxes: the ribbon's Image and Remove image
//! buttons, their errors and the missing-asset placeholder.

use eframe::egui;
use egui_kittest::{
    Harness,
    kittest::{NodeT, Queryable},
};
use reasoncraft::ReasonCraftApp;
use reasoncraft::doc_tab::DocTab;
use reasoncraft::file_ops::{FakeDialogs, Modal};
use reasoncraft::i18n::t;
use reasoncraft_core::{CardKind, Document, ImageRef, Item, NodeId, Parent};

type H = Harness<'static, ReasonCraftApp>;

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
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .with_max_steps(32)
        .build_ui_state(|ui, app: &mut ReasonCraftApp| app.show(ui), app);
    h.run();
    (h, premise)
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
    h.run();

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
    let file = dir.path().join("photo.png");
    image::RgbImage::from_pixel(40, 20, image::Rgb([200, 40, 40]))
        .save(&file)
        .unwrap();
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
    h.run();
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
