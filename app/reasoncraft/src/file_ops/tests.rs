use std::path::Path;

use reasoncraft_core::file::{self, FileError};

use super::*;
use crate::actions::{Action, apply, is_enabled, wait_for_images};
use reasoncraft_core::{CardKind, NodeId, Package, Parent};

#[test]
fn rcraft_extension_is_forced() {
    let ext = |p: &str| with_rcraft_extension(PathBuf::from(p));
    assert_eq!(ext("maps/a"), PathBuf::from("maps/a.rcraft"));
    assert_eq!(ext("maps/a.rcraft"), PathBuf::from("maps/a.rcraft"));
    assert_eq!(ext("maps/a.RCraft"), PathBuf::from("maps/a.RCraft"));
    assert_eq!(ext("maps/a.txt"), PathBuf::from("maps/a.txt.rcraft"));
    assert_eq!(ext("my.map"), PathBuf::from("my.map.rcraft"));
}

#[test]
fn error_keys_per_variant() {
    let io = || FileError::Io(std::io::Error::other("disk full"));
    let newer = || FileError::NewerVersion {
        found: 7,
        supported: 1,
    };
    let malformed = || FileError::Malformed("bad json".into());
    use FileOp::{Open, Save};
    assert_eq!(error_key(Open, &io()), "error.open.io");
    assert_eq!(
        error_key(Open, &FileError::NotAReasonCraftFile),
        "error.open.not_a_file"
    );
    assert_eq!(
        error_key(Open, &FileError::MissingDocument),
        "error.open.missing_document"
    );
    assert_eq!(error_key(Open, &malformed()), "error.open.malformed");
    assert_eq!(error_key(Open, &newer()), "error.open.newer_version");
    assert_eq!(error_key(Save, &io()), "error.save.io");
    assert_eq!(error_key(Save, &malformed()), "error.save.other");
    let too_long = || FileError::TooMuchText {
        chars: 8_000_001,
        max: 8_000_000,
    };
    assert_eq!(error_key(Save, &too_long()), "error.save.too_much_text");
    let text = error_text(Save, &too_long());
    assert!(
        text.contains("8000001") && text.contains("8000000"),
        "{text}"
    );

    let text = error_text(Open, &newer());
    assert!(text.contains('7') && text.contains('1'), "{text}");
    assert!(!text.starts_with("error."), "{text}");
    for (op, err) in [
        (Open, io()),
        (Open, FileError::NotAReasonCraftFile),
        (Open, FileError::MissingDocument),
        (Open, malformed()),
        (Save, io()),
        (Save, malformed()),
        (Save, too_long()),
    ] {
        let text = error_text(op, &err);
        assert!(!text.starts_with("error."), "missing ftl entry: {text}");
    }
}

fn dirty_tab() -> DocTab {
    let mut tab = DocTab::new();
    let root = tab.doc.root;
    tab.edit(|d| d.add_card(Parent::Box(root), CardKind::Reason, "x"))
        .unwrap();
    tab
}

#[test]
fn failed_save_keeps_path_and_dirty() {
    let dir = tempfile::tempdir().unwrap();
    let old = dir.path().join("old.rcraft");
    let mut tab = dirty_tab();
    tab.path = Some(old.clone());
    let fake = FakeDialogs::default();
    fake.set_save(dir.path().join("no/such/dir/new.rcraft"));
    assert!(save_tab(&mut tab, &fake, true).is_err());
    assert_eq!(tab.path, Some(old));
    assert!(tab.is_dirty());
}

#[test]
fn cancelled_save_dialog_changes_nothing() {
    let mut tab = dirty_tab();
    let fake = FakeDialogs::default();
    assert!(matches!(
        save_tab(&mut tab, &fake, false),
        Ok(SaveOutcome::Cancelled)
    ));
    assert_eq!(tab.path, None);
    assert!(tab.is_dirty());
}

#[test]
fn save_uses_known_path_without_dialog() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("known.rcraft");
    let mut tab = dirty_tab();
    tab.path = Some(path.clone());
    // No dialog answer queued: a dialog would cancel the save.
    let outcome = save_tab(&mut tab, &FakeDialogs::default(), false).unwrap();
    assert_eq!(outcome, SaveOutcome::Saved);
    assert!(path.is_file());
    assert!(!tab.is_dirty());
}

#[test]
fn open_reuses_untouched_tab_else_adds() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.rcraft");
    let b = dir.path().join("b.rcraft");
    reasoncraft_core::file::save(&reasoncraft_core::Document::new("A"), &a).unwrap();
    reasoncraft_core::file::save(&reasoncraft_core::Document::new("B"), &b).unwrap();

    let mut app = ReasonCraftApp::new_for_test();
    open_into_app(&mut app, &a);
    assert_eq!(app.tabs.len(), 1, "the untouched new tab is reused");
    assert_eq!(app.tabs[0].path.as_deref(), Some(a.as_path()));
    open_into_app(&mut app, &b);
    assert_eq!(app.tabs.len(), 2);
    assert_eq!(app.active, 1);
    // Already open (by another spelling of the path): activate it.
    open_into_app(&mut app, &dir.path().join(".").join("a.rcraft"));
    assert_eq!(app.tabs.len(), 2);
    assert_eq!(app.active, 0);
    assert!(app.modal.is_none());

    open_into_app(&mut app, &dir.path().join("missing.rcraft"));
    assert_eq!(app.tabs.len(), 2);
    assert!(matches!(app.modal, Some(Modal::Error { .. })));
}

/// UX-8: a missing file (or a folder) on the command line says so.
#[test]
fn command_line_reports_missing_files_and_bad_ones() {
    let dir = tempfile::tempdir().unwrap();
    let good = dir.path().join("good.rcraft");
    let bad = dir.path().join("bad.rcraft");
    reasoncraft_core::file::save(&reasoncraft_core::Document::new("G"), &good).unwrap();
    std::fs::write(&bad, "junk").unwrap();

    let mut app = ReasonCraftApp::new_for_test();
    open_paths(&mut app, [dir.path().join("nope.rcraft").into_os_string()]);
    match app.modal.take() {
        Some(Modal::Error { title, message }) => {
            assert!(title.contains("nope.rcraft"), "{title}");
            assert_eq!(message, crate::i18n::t("error.open.not_found"));
        }
        other => panic!("a missing file is not reported: {other:?}"),
    }
    open_paths(&mut app, [dir.path().as_os_str().to_owned()]);
    match app.modal.take() {
        Some(Modal::Error { message, .. }) => {
            assert_eq!(message, crate::i18n::t("error.open.not_a_file"));
        }
        other => panic!("a folder is not reported: {other:?}"),
    }
    assert_eq!(app.tabs.len(), 1);
    open_paths(&mut app, [bad.into_os_string(), good.into_os_string()]);
    assert_eq!(app.tabs.len(), 1);
    assert_eq!(app.tabs[0].doc.boxes[&app.tabs[0].doc.root].text, "G");
    match &app.modal {
        Some(Modal::Error { title, .. }) => assert!(title.contains("bad.rcraft"), "{title}"),
        other => panic!("expected an error modal, got {other:?}"),
    }
}

#[test]
fn appended_extension_onto_existing_file_needs_confirm() {
    let dir = tempfile::tempdir().unwrap();
    let existing = dir.path().join("robots.rcraft");
    std::fs::write(&existing, "old bytes").unwrap();
    let mut tab = dirty_tab();
    let fake = FakeDialogs::default();
    fake.set_save(dir.path().join("robots"));
    let outcome = save_tab(&mut tab, &fake, true).unwrap();
    assert_eq!(outcome, SaveOutcome::NeedsReplaceConfirm(existing.clone()));
    assert_eq!(std::fs::read(&existing).unwrap(), b"old bytes");
    assert_eq!(tab.path, None);
    assert!(tab.is_dirty());
}

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

#[test]
fn opening_a_file_with_oversized_text_says_it_was_cut() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("huge.rcraft");
    let mut doc = reasoncraft_core::Document::new("c");
    let root = doc.root;
    // Saved unchecked, as a hostile file would hold it.
    doc.boxes[&root].text = "x".repeat(reasoncraft_core::MAX_BOX_CHARS + 1);
    file::save(&doc, &path).unwrap();

    let mut app = ReasonCraftApp::new_for_test();
    open_into_app(&mut app, &path);
    let tab = app.active_tab();
    assert_eq!(
        tab.doc.boxes[&tab.doc.root].text.chars().count(),
        reasoncraft_core::MAX_BOX_CHARS
    );
    assert_eq!(app.status_note, Some(crate::i18n::t("status.open_cut")));
    // An unsaved copy: a save never overwrites the file unasked.
    assert_eq!(tab.path, None);
    assert!(tab.is_dirty());

    let normal = dir.path().join("normal.rcraft");
    file::save(&reasoncraft_core::Document::new("n"), &normal).unwrap();
    let mut app = ReasonCraftApp::new_for_test();
    open_into_app(&mut app, &normal);
    assert_eq!(app.status_note, None);
    assert_eq!(app.active_tab().path.as_deref(), Some(normal.as_path()));
    assert!(!app.active_tab().is_dirty());
}

#[test]
fn command_line_paths_are_stored_absolute() {
    let cwd = std::env::current_dir().unwrap();
    let dir = tempfile::tempdir_in(&cwd).unwrap();
    let file = dir.path().join("rel.rcraft");
    reasoncraft_core::file::save(&reasoncraft_core::Document::new("R"), &file).unwrap();
    let relative = file.strip_prefix(&cwd).unwrap().to_path_buf();
    assert!(relative.is_relative());

    let mut app = ReasonCraftApp::new_for_test();
    open_paths(&mut app, [relative.into_os_string()]);
    let stored = app.active_tab().path.clone().unwrap();
    assert!(stored.is_absolute(), "{stored:?}");
    assert_eq!(stored.canonicalize().unwrap(), file.canonicalize().unwrap());
    assert!(app.settings.recent_files.iter().all(|p| p.is_absolute()));
}
