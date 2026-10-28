//! Autosave to the recovery folder, and the offer to restore after a crash.
//! Every test uses its own temp folder: never the user's recovery folder.

#[path = "../common/mod.rs"]
mod common;

use std::path::{Path, PathBuf};

use eframe::egui;
use reasoncraft::ReasonCraftApp;
use reasoncraft::file_ops::Modal;
use reasoncraft::recovery;
use reasoncraft_core::file;

mod autosave;
mod restore;
mod restore_open;

pub fn app_with_recovery(dir: &Path) -> ReasonCraftApp {
    ReasonCraftApp::new_for_test_with_recovery(dir.to_path_buf())
}

pub fn set_claim(app: &mut ReasonCraftApp, text: &str) {
    let tab = &mut app.tabs[app.active];
    tab.edit(|d| d.set_text(d.root, text)).unwrap();
}

pub fn wait(app: &ReasonCraftApp) {
    app.recovery.as_ref().unwrap().wait_idle();
}

/// Two ticks an interval apart, then wait for the writes.
pub fn autosave(app: &mut ReasonCraftApp, t0: f64) {
    recovery::tick(app, t0);
    recovery::tick(app, t0 + 46.0);
    wait(app);
}

/// The recovery copy of tab `i`.
pub fn recovery_file(app: &ReasonCraftApp, i: usize) -> PathBuf {
    let store = app.recovery.as_ref().unwrap();
    store
        .session_dir()
        .join(format!("{}.rcraft", app.tabs[i].recovery_key))
}

pub fn root_text(path: &Path) -> String {
    let pkg = file::open_package(path).unwrap();
    pkg.doc.boxes[&pkg.doc.root].text.clone()
}

/// A saved map at `path` (its claim `text`), opened in a recovery-enabled
/// app, edited to "Edited" and autosaved; then the app "crashes" (is
/// dropped without a normal exit).
pub fn crash_with_edited_file(rec: &Path, path: &Path) {
    let mut doc = reasoncraft_core::Document::new("");
    doc.set_text(doc.root, "Saved").unwrap();
    file::save(&doc, path).unwrap();
    let mut app = app_with_recovery(rec);
    reasoncraft::file_ops::open_into_app(&mut app, path);
    set_claim(&mut app, "Edited");
    autosave(&mut app, 0.0);
    assert!(recovery_file(&app, 0).is_file());
    drop(app);
}

pub fn offered(app: &ReasonCraftApp) -> Vec<recovery::RecoveredDoc> {
    match &app.modal {
        Some(Modal::Recover(docs)) => docs.to_vec(),
        other => panic!("expected the recovery prompt, got {other:?}"),
    }
}
