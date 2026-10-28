//! Background work and the close paths: an image still being prepared (or
//! finished but held for an edit) counts as unsaved work, "Save" in the
//! close prompt waits for it without blocking frames, and status notes go
//! away on their own (or with the next edit, not with looking around).

use crate::common::harness;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use egui_kittest::kittest::Queryable;
use reasoncraft::ReasonCraftApp;
use reasoncraft::actions::{Action, UnsavedChoice, apply};
use reasoncraft::doc_tab::DocTab;
use reasoncraft::export::Notice;
use reasoncraft::file_ops::{FakeDialogs, Modal};
use reasoncraft::i18n::t;
use reasoncraft::jobs::Hold;
use reasoncraft::shell::statusbar::NOTE_SECS;
use reasoncraft_core::{CardKind, Document, Item, NodeId, Parent, file};

use crate::common::H;

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

fn photo(dir: &Path) -> PathBuf {
    let file = dir.join("photo.png");
    image::RgbImage::from_pixel(40, 20, image::Rgb([200, 40, 40]))
        .save(&file)
        .unwrap();
    file
}

fn unsaved_prompt(app: &ReasonCraftApp, window: bool) -> bool {
    matches!(app.modal, Some(Modal::Unsaved { tab, window: w, waiting: false })
        if w == window && tab == app.tabs[0].recovery_key)
}

/// Steps frames until `done` holds (at most 60 s); no frame may stall, as
/// nothing waits on the UI thread.
fn step_until(h: &mut H, done: impl Fn(&ReasonCraftApp) -> bool) {
    let start = Instant::now();
    while !done(h.state()) {
        assert!(start.elapsed() < Duration::from_secs(60), "never happened");
        let frame = Instant::now();
        h.step();
        assert!(frame.elapsed() < Duration::from_secs(10), "a frame blocked");
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn waiting(app: &ReasonCraftApp) -> bool {
    matches!(app.modal, Some(Modal::Unsaved { waiting: true, .. }))
}

/// A harness whose premise is waiting for an image (held until the
/// returned hold is released), with `close` sent and the unsaved prompt
/// showing; then Save is chosen.
fn save_while_preparing(
    dir: &Path,
    fake: &FakeDialogs,
    close: Action,
    file: PathBuf,
) -> (H, NodeId, Hold) {
    let (mut app, premise) = app(fake);
    let hold = Hold::default();
    app.image_jobs.hold = Some(hold.clone());
    let mut h = harness(app);
    fake.set_image(file);
    h.state_mut().push(Action::InsertImage);
    h.step();
    h.state_mut().push(close);
    h.step();
    assert!(
        matches!(h.state().modal, Some(Modal::Unsaved { waiting: false, .. })),
        "{:?}",
        h.state().modal
    );
    fake.set_save(dir.join("with-image.rcraft"));
    h.state_mut()
        .push(Action::ResolveUnsaved(UnsavedChoice::Save));
    let start = Instant::now();
    h.step();
    // A Save that waited for the image would wait until the hold gives up.
    assert!(start.elapsed() < Duration::from_secs(10), "Save blocked");
    // The frame that applied Save drew the prompt before it; one more.
    h.step();
    // The prompt stays, waiting, with a spinner and no Save button.
    assert!(waiting(h.state()), "{:?}", h.state().modal);
    // "Preparing image…": the box, the busy chip and now the prompt.
    assert_eq!(
        h.query_all_by_label(&t("unsaved.waiting_images")).count(),
        3
    );
    assert!(h.query_by_label(&t("unsaved.save_changes")).is_none());
    h.get_by_label(&t("unsaved.cancel"));
    (h, premise, hold)
}

fn saved_image(path: &Path, premise: NodeId) -> bool {
    let saved = file::open_package(path).unwrap();
    let image = saved.doc.boxes[&premise].image.clone();
    image.is_some_and(|i| saved.assets.get(&i.asset).is_some())
}

#[test]
fn save_on_close_waits_for_the_image_without_blocking() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDialogs::default();
    let (mut h, premise, hold) =
        save_while_preparing(dir.path(), &fake, Action::CloseTab(0), photo(dir.path()));
    hold.release();
    step_until(&mut h, |app| app.modal.is_none() && !app.saves.busy());
    assert!(h.state().active_tab().path.is_none(), "the tab closed");
    assert!(saved_image(&dir.path().join("with-image.rcraft"), premise));
}

#[test]
fn cancel_while_save_waits_keeps_the_tab_and_the_image() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDialogs::default();
    let (mut h, premise, hold) =
        save_while_preparing(dir.path(), &fake, Action::CloseTab(0), photo(dir.path()));
    h.state_mut()
        .push(Action::ResolveUnsaved(UnsavedChoice::Cancel));
    h.step();
    assert!(h.state().modal.is_none());
    assert_eq!(h.state().active_tab().pending_images.len(), 1);
    hold.release();
    step_until(&mut h, |app| app.active_tab().pending_images.is_empty());
    h.step();
    let tab = h.state().active_tab();
    assert!(tab.doc.boxes[&premise].image.is_some(), "the image landed");
    assert!(h.state().modal.is_none(), "nothing was saved or closed");
    assert!(!dir.path().join("with-image.rcraft").exists());
}

#[test]
fn window_close_save_waits_for_the_image() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDialogs::default();
    let (mut h, premise, hold) =
        save_while_preparing(dir.path(), &fake, Action::CloseWindow, photo(dir.path()));
    hold.release();
    step_until(&mut h, ReasonCraftApp::close_confirmed);
    assert!(saved_image(&dir.path().join("with-image.rcraft"), premise));
}

#[test]
fn a_failed_image_still_lets_the_waiting_save_go_ahead() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDialogs::default();
    let broken = dir.path().join("broken.png");
    std::fs::write(&broken, b"\x89PNG\r\n\x1a\n not a picture").unwrap();
    let (mut h, premise, hold) =
        save_while_preparing(dir.path(), &fake, Action::CloseTab(0), broken);
    hold.release();
    step_until(&mut h, |app| !waiting(app) && !app.saves.busy());
    let app = h.state();
    // The failure is reported: as a message, as no other one shows. The
    // tab is saved, and closes once the message is closed.
    assert!(
        matches!(app.modal, Some(Modal::Error { .. })),
        "{:?}",
        app.modal
    );
    assert!(!app.active_tab().is_dirty(), "saved");
    assert!(!saved_image(&dir.path().join("with-image.rcraft"), premise));
    h.state_mut().push(Action::DismissModal);
    h.step();
    h.step();
    assert!(h.state().active_tab().path.is_none(), "closed");
}

#[test]
fn closing_the_window_while_an_image_is_prepared_asks() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDialogs::default();
    let (mut app, _) = app(&fake);
    let hold = Hold::default();
    app.image_jobs.hold = Some(hold.clone());
    fake.set_image(photo(dir.path()));
    apply(&mut app, Action::InsertImage);

    apply(&mut app, Action::CloseWindow);
    assert!(unsaved_prompt(&app, true), "{:?}", app.modal);
    assert!(!app.close_confirmed());
    apply(&mut app, Action::ResolveUnsaved(UnsavedChoice::Discard));
    assert!(app.close_confirmed());
    hold.release();
}

#[test]
fn a_held_image_lands_before_the_close_prompt() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDialogs::default();
    let (mut app, premise) = app(&fake);
    let hold = Hold::default();
    app.image_jobs.hold = Some(hold.clone());
    let mut h = harness(app);
    fake.set_image(photo(dir.path()));
    h.state_mut().push(Action::InsertImage);
    h.step();
    h.state_mut().push(Action::Edit);
    h.step();
    hold.release();
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
fn status_notes_clear_on_the_next_edit_not_on_looking_around() {
    let (mut app, premise) = app(&FakeDialogs::default());
    let saved = Notice::Saved(std::env::temp_dir().join("map.png"));
    app.export.notice = Some(saved.clone());
    app.status_note = Some("A note".into());
    // "Open folder" acts on the notice; selecting, zooming and switching
    // panels only look around.
    let root = app.active_tab().doc.root;
    for a in [
        Action::OpenExportFolder,
        Action::Select(Some(Item::Box(root))),
        Action::Navigate(reasoncraft::actions::Dir::Down),
        Action::ZoomIn,
        Action::SetZoom(0.8),
        Action::ZoomFit,
        Action::TogglePanel(reasoncraft::actions::Panel::Right),
    ] {
        apply(&mut app, a);
    }
    assert_eq!(app.export.notice, Some(saved));
    assert_eq!(app.status_note.as_deref(), Some("A note"));
    // A document edit clears both.
    apply(&mut app, Action::Select(Some(Item::Box(premise))));
    apply(&mut app, Action::AddReason);
    assert!(app.status_note.is_none());
    assert!(app.export.notice.is_none());
}
