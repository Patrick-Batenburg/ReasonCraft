//! Background work and the close paths: an image still being prepared (or
//! finished but held for an edit) counts as unsaved work, and status notes
//! go away on their own.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use eframe::egui;
use egui_kittest::{Harness, kittest::Queryable};
use reasoncraft::ReasonCraftApp;
use reasoncraft::actions::{Action, UnsavedChoice, apply};
use reasoncraft::doc_tab::DocTab;
use reasoncraft::export::Notice;
use reasoncraft::file_ops::{FakeDialogs, Modal};
use reasoncraft::i18n::t;
use reasoncraft::shell::statusbar::NOTE_SECS;
use reasoncraft_core::{CardKind, Document, Item, NodeId, Parent, file};

type H = Harness<'static, ReasonCraftApp>;

/// An app with a claim and one selected premise.
fn app(fake: &FakeDialogs) -> (ReasonCraftApp, NodeId) {
    let mut doc = Document::new("Claim");
    let root = doc.root;
    let (_, premise) = doc
        .add_card(Parent::Box(root), CardKind::Reason, "Premise")
        .unwrap();
    let mut app = ReasonCraftApp::new_for_test_with(Box::new(fake.clone()));
    app.tabs[0] = DocTab::from_document(doc);
    app.tabs[0].selection = Some(Item::Box(premise));
    app.left_panel = false;
    (app, premise)
}

fn harness(app: ReasonCraftApp) -> H {
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .with_max_steps(32)
        .build_ui_state(|ui, app: &mut ReasonCraftApp| app.show(ui), app);
    h.run();
    h
}

fn photo(dir: &Path) -> PathBuf {
    let file = dir.join("photo.png");
    image::RgbImage::from_pixel(40, 20, image::Rgb([200, 40, 40]))
        .save(&file)
        .unwrap();
    file
}

fn unsaved_prompt(app: &ReasonCraftApp, window: bool) -> bool {
    matches!(app.modal, Some(Modal::Unsaved { tab: 0, window: w }) if w == window)
}

#[test]
fn closing_while_an_image_is_prepared_asks_and_save_waits_for_it() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDialogs::default();
    let (mut app, premise) = app(&fake);
    app.image_jobs.delay = Some(Duration::from_millis(300));
    fake.set_image(photo(dir.path()));
    apply(&mut app, Action::InsertImage);
    assert!(!app.active_tab().is_dirty(), "nothing in the document yet");

    apply(&mut app, Action::CloseTab(0));
    assert!(unsaved_prompt(&app, false), "{:?}", app.modal);
    assert_eq!(app.tabs.len(), 1);

    // Save waits for the image, so the file has it.
    let path = dir.path().join("with-image.rcraft");
    fake.set_save(path.clone());
    apply(&mut app, Action::ResolveUnsaved(UnsavedChoice::Save));
    assert!(app.modal.is_none(), "{:?}", app.modal);
    assert!(app.active_tab().path.is_none(), "the tab closed");
    let saved = file::open_package(&path).unwrap();
    let image = saved.doc.boxes[&premise]
        .image
        .clone()
        .expect("saved image");
    assert!(saved.assets.get(&image.asset).is_some());
}

#[test]
fn closing_the_window_while_an_image_is_prepared_asks() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDialogs::default();
    let (mut app, _) = app(&fake);
    app.image_jobs.delay = Some(Duration::from_millis(300));
    fake.set_image(photo(dir.path()));
    apply(&mut app, Action::InsertImage);

    apply(&mut app, Action::CloseWindow);
    assert!(unsaved_prompt(&app, true), "{:?}", app.modal);
    assert!(!app.close_confirmed());
    apply(&mut app, Action::ResolveUnsaved(UnsavedChoice::Discard));
    assert!(app.close_confirmed());
}

#[test]
fn a_held_image_lands_before_the_close_prompt() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDialogs::default();
    let (mut app, premise) = app(&fake);
    app.image_jobs.delay = Some(Duration::from_millis(100));
    let mut h = harness(app);
    fake.set_image(photo(dir.path()));
    h.state_mut().push(Action::InsertImage);
    h.step();
    h.state_mut().push(Action::Edit);
    h.step();
    let start = Instant::now();
    while h.state().image_jobs.busy() {
        assert!(start.elapsed() < Duration::from_secs(60));
        h.step();
        std::thread::sleep(Duration::from_millis(5));
    }
    h.step();
    assert_eq!(h.state().image_jobs.waiting.len(), 1, "held for the edit");
    // The busy chip stays while the image is held, next to the box's
    // placeholder (both read "Preparing image…").
    assert_eq!(
        h.query_all_by_label(&t("status.preparing_image")).count(),
        2
    );

    h.state_mut().push(Action::CloseTab(0));
    h.step();
    let app = h.state();
    assert!(unsaved_prompt(app, false), "{:?}", app.modal);
    let tab = app.active_tab();
    assert!(tab.doc.boxes[&premise].image.is_some(), "the image landed");
    assert!(tab.pending_images.is_empty());
    assert!(tab.is_dirty());
}

#[test]
fn status_notes_expire() {
    let (mut app, _) = app(&FakeDialogs::default());
    app.status_note = Some("A passing note".into());
    app.export.notice = Some(Notice::Copied);
    let mut h = harness(app);
    h.get_by_label("A passing note");
    h.get_by_label(&t("export.copied"));

    // Each kittest step moves egui's clock by a quarter of a second.
    let steps = (NOTE_SECS * 4.0) as usize + 2;
    for _ in 0..steps {
        h.step();
    }
    assert!(h.state().status_note.is_none());
    assert!(h.state().export.notice.is_none());
    h.step();
    assert!(h.query_by_label("A passing note").is_none());
    assert!(h.query_by_label(&t("export.copied")).is_none());
}

#[test]
fn status_notes_clear_on_the_next_action() {
    let (mut app, _) = app(&FakeDialogs::default());
    let saved = Notice::Saved(std::env::temp_dir().join("map.png"));
    app.export.notice = Some(saved.clone());
    app.status_note = Some("A note".into());
    // "Open folder" acts on the notice, so it keeps it.
    apply(&mut app, Action::OpenExportFolder);
    assert_eq!(app.export.notice, Some(saved));
    apply(&mut app, Action::Select(None));
    assert!(app.status_note.is_none());
    assert!(app.export.notice.is_none());
}
