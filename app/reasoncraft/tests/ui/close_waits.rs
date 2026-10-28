//! Closing the window while work runs in the background: running exports
//! keep the window open behind a waiting prompt (with Quit anyway), and
//! several tabs whose Save waits for images close one after the other.

use crate::common::{harness, step_until};
use std::path::{Path, PathBuf};

use egui_kittest::kittest::Queryable;
use reasoncraft::ReasonCraftApp;
use reasoncraft::actions::{Action, UnsavedChoice};
use reasoncraft::doc_tab::DocTab;
use reasoncraft::export::{ExportFormat, ExportOptions, ExportRequest, PageSetup};
use reasoncraft::file_ops::{FakeDialogs, Modal};
use reasoncraft::i18n::t;
use reasoncraft::jobs::Hold;
use reasoncraft_core::{CardKind, Document, Item, NodeId, Parent, file};

use crate::common::H;

/// A document with a claim and one premise, which is selected.
fn tab_with_premise() -> (DocTab, NodeId) {
    let mut doc = Document::new("Claim");
    let root = doc.root;
    let (_, premise) = doc
        .add_card(Parent::Box(root), CardKind::Reason, "Premise")
        .unwrap();
    let mut tab = DocTab::from_document(doc);
    tab.selection = Some(Item::Box(premise));
    (tab, premise)
}

fn names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

/// A harness with a PNG export to `file` running (held until the returned
/// hold is released) and the window asked to close.
fn close_while_exporting(fake: &FakeDialogs, file: PathBuf) -> (H, Hold) {
    let mut app = ReasonCraftApp::new_for_test_with(Box::new(fake.clone()));
    let hold = Hold::default();
    app.export.hold = Some(hold.clone());
    app.left_panel = false;
    let mut h = harness(app);
    fake.set_export(file);
    h.state_mut().push(Action::RunExport(ExportRequest {
        format: ExportFormat::Png,
        options: ExportOptions::default(),
        page: PageSetup::default(),
    }));
    h.step();
    assert!(h.state().export.busy());
    h.state_mut().push(Action::CloseWindow);
    // One frame applies the close, the next lays the prompt out, the
    // third shows it.
    h.run_steps(3);
    assert_eq!(h.state().modal, Some(Modal::ExportsRunning));
    assert!(!h.state().close_confirmed(), "the export is not dropped");
    h.get_by_role_and_label(
        eframe::egui::accesskit::Role::Dialog,
        &t("closing.exports.title"),
    );
    h.get_by_label(&t("closing.exports.waiting"));
    h.get_by_label(&t("closing.exports.quit"));
    (h, hold)
}

#[test]
fn window_close_waits_for_a_running_export() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDialogs::default();
    let file = dir.path().join("map.png");
    let (mut h, hold) = close_while_exporting(&fake, file.clone());
    hold.release();
    step_until(&mut h, ReasonCraftApp::close_confirmed, "the close");
    assert!(!h.state().export.busy());
    assert!(file.exists(), "the export finished before the close");
    assert_eq!(names(dir.path()), ["map.png"]);
}

#[test]
fn quit_anyway_closes_at_once_and_leaves_no_file() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDialogs::default();
    let file = dir.path().join("map.png");
    let (mut h, hold) = close_while_exporting(&fake, file.clone());
    h.get_by_label(&t("closing.exports.quit")).click();
    h.step();
    h.step();
    assert!(h.state().close_confirmed());
    assert!(h.state().modal.is_none());
    // The worker ends without writing anything, not even a temp file.
    hold.release();
    step_until(&mut h, |app| !app.export.busy(), "the export");
    assert!(names(dir.path()).is_empty(), "{:?}", names(dir.path()));
}

#[test]
fn cancel_keeps_the_window_and_the_export() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDialogs::default();
    let file = dir.path().join("map.png");
    let (mut h, hold) = close_while_exporting(&fake, file.clone());
    h.get_by_label(&t("unsaved.cancel")).click();
    h.step();
    assert!(h.state().modal.is_none());
    assert!(h.state().export.busy(), "the export keeps running");
    hold.release();
    step_until(&mut h, |app| !app.export.busy(), "the export");
    h.step();
    assert!(file.exists());
    assert!(!h.state().close_confirmed());
}

#[test]
fn a_failed_export_during_the_wait_shows_and_keeps_the_window() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDialogs::default();
    let file = dir.path().join("missing-folder").join("map.png");
    let (mut h, hold) = close_while_exporting(&fake, file);
    hold.release();
    step_until(&mut h, |app| !app.export.busy(), "the export");
    h.step();
    assert!(
        matches!(h.state().modal, Some(Modal::Error { .. })),
        "{:?}",
        h.state().modal
    );
    assert!(!h.state().close_confirmed());
}

fn photo(dir: &Path) -> PathBuf {
    let file = dir.join("photo.png");
    image::RgbImage::from_pixel(40, 20, image::Rgb([200, 40, 40]))
        .save(&file)
        .unwrap();
    file
}

fn saved_image(path: &Path, premise: NodeId) -> bool {
    let saved = file::open_package(path).unwrap();
    let image = saved.doc.boxes[&premise].image.clone();
    image.is_some_and(|i| saved.assets.get(&i.asset).is_some())
}

/// Waits for the close prompt of tab `i` (not yet waiting), then answers
/// Save with `path`.
fn save_tab_when_asked(h: &mut H, fake: &FakeDialogs, i: usize, path: PathBuf) {
    let asked = |app: &ReasonCraftApp| {
        app.modal
            == Some(Modal::Unsaved {
                tab: app.tabs[i].recovery_key,
                window: true,
                waiting: false,
            })
    };
    step_until(h, asked, "the close prompt");
    fake.set_save(path);
    h.state_mut()
        .push(Action::ResolveUnsaved(UnsavedChoice::Save));
    h.step();
}

#[test]
fn two_tabs_waiting_for_images_both_save_then_the_window_closes() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDialogs::default();
    let (first, premise_a) = tab_with_premise();
    let (second, premise_b) = tab_with_premise();
    let mut app = ReasonCraftApp::new_for_test_with(Box::new(fake.clone()));
    app.tabs[0] = first;
    app.tabs.push(second);
    app.left_panel = false;
    let mut h = harness(app);
    // Both tabs prepare an image; the second is still being prepared
    // when its own prompt comes up.
    let (hold_a, hold_b) = (Hold::default(), Hold::default());
    h.state_mut().image_jobs.hold = Some(hold_a.clone());
    fake.set_image(photo(dir.path()));
    h.state_mut().push(Action::InsertImage);
    h.step();
    h.state_mut().image_jobs.hold = Some(hold_b.clone());
    h.state_mut().push(Action::ActivateTab(1));
    fake.set_image(photo(dir.path()));
    h.state_mut().push(Action::InsertImage);
    h.step();
    assert_eq!(h.state().tabs[0].pending_images.len(), 1);
    assert_eq!(h.state().tabs[1].pending_images.len(), 1);

    h.state_mut().push(Action::CloseWindow);
    h.step();
    let (a, b) = (dir.path().join("a.rcraft"), dir.path().join("b.rcraft"));
    save_tab_when_asked(&mut h, &fake, 0, a.clone());
    h.step();
    let key = h.state().tabs[0].recovery_key;
    assert!(matches!(
        h.state().modal,
        Some(Modal::Unsaved {
            tab,
            waiting: true,
            ..
        }) if tab == key
    ));
    hold_a.release();
    save_tab_when_asked(&mut h, &fake, 1, b.clone());
    h.step();
    assert!(
        matches!(
            h.state().modal,
            Some(Modal::Unsaved {
                tab,
                waiting: true,
                ..
            }) if tab == h.state().tabs[1].recovery_key
        ),
        "the second tab waits too: {:?}",
        h.state().modal
    );
    assert!(!h.state().close_confirmed());
    hold_b.release();
    step_until(&mut h, ReasonCraftApp::close_confirmed, "the close");
    assert!(saved_image(&a, premise_a));
    assert!(saved_image(&b, premise_b));
}

#[test]
fn an_export_failing_behind_the_unsaved_prompt_still_shows() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDialogs::default();
    let mut app = ReasonCraftApp::new_for_test_with(Box::new(fake.clone()));
    let hold = Hold::default();
    app.export.hold = Some(hold.clone());
    app.left_panel = false;
    let mut h = harness(app);
    fake.set_export(dir.path().join("missing-folder").join("map.png"));
    h.state_mut().push(Action::RunExport(ExportRequest {
        format: ExportFormat::Png,
        options: ExportOptions::default(),
        page: PageSetup::default(),
    }));
    h.step();
    // Unsaved work, so the close asks first.
    h.state_mut().push(Action::AddSnippet("unsaved".into()));
    h.state_mut().push(Action::CloseWindow);
    h.step();
    assert!(matches!(
        h.state().modal,
        Some(Modal::Unsaved { window: true, .. })
    ));
    // The export fails while the prompt is open: its message waits.
    hold.release();
    step_until(&mut h, |app| !app.export.busy(), "the export");
    assert_eq!(h.state().export.waiting_errors.len(), 1);
    h.state_mut()
        .push(Action::ResolveUnsaved(UnsavedChoice::Discard));
    h.step();
    assert!(!h.state().close_confirmed(), "the failure was never shown");
    h.step();
    assert!(
        matches!(h.state().modal, Some(Modal::Error { .. })),
        "{:?}",
        h.state().modal
    );
}
