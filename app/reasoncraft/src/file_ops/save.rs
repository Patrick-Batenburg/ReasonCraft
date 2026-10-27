//! Saving tabs: the dialogs, the replace prompts and the write itself.

use std::path::PathBuf;

use reasoncraft_core::Package;
use reasoncraft_core::file::{self, FileError};

use super::{
    AfterSave, EXTENSION, FileDialogs, FileOp, Modal, SaveOutcome, carry_on, show_error,
    with_rcraft_extension,
};
use crate::app::ReasonCraftApp;
use crate::doc_tab::DocTab;

/// Save `tab` to its path, asking for one when it has none or `save_as` is
/// set. Only [`SaveOutcome::Saved`] wrote the file; otherwise, and on
/// error, the tab keeps its path and stays dirty.
pub fn save_tab(
    tab: &mut DocTab,
    dialogs: &dyn FileDialogs,
    save_as: bool,
) -> Result<SaveOutcome, FileError> {
    save_tab_at(tab, dialogs, save_as).map_err(|(_, err)| err)
}

/// Like [`save_tab`]; a failure also names the file it tried to write.
pub(super) fn save_tab_at(
    tab: &mut DocTab,
    dialogs: &dyn FileDialogs,
    save_as: bool,
) -> Result<SaveOutcome, (PathBuf, FileError)> {
    tab.commit_session();
    let path = match (&tab.path, save_as) {
        (Some(path), false) if crate::recovery::original_changed(tab, path) => {
            return Ok(SaveOutcome::OriginalChanged(path.clone()));
        }
        (Some(path), false) => path.clone(),
        _ => {
            let suggested = format!("{}.{EXTENSION}", tab.name());
            let Some(chosen) = dialogs.pick_save(&suggested) else {
                return Ok(SaveOutcome::Cancelled);
            };
            let path = with_rcraft_extension(chosen.clone());
            // The dialog asked about overwriting `chosen`, not this file.
            if path != chosen && path.exists() {
                return Ok(SaveOutcome::NeedsReplaceConfirm(path));
            }
            path
        }
    };
    write_tab(tab, path)?;
    Ok(SaveOutcome::Saved)
}

/// Write `tab` to `path`. Only on success does the tab take the path and
/// count as saved.
pub(super) fn write_tab(tab: &mut DocTab, path: PathBuf) -> Result<(), (PathBuf, FileError)> {
    // Only the assets the document refers to are written; the tab keeps
    // the rest, so a redo after the save still finds its image.
    let pkg = Package {
        doc: tab.doc.clone(),
        assets: tab.assets.clone(),
        other_entries: std::mem::take(&mut tab.other_entries),
    };
    let result = file::save_package(&pkg, &path);
    tab.other_entries = pkg.other_entries;
    result.map_err(|err| (path.clone(), err))?;
    tab.history.mark_saved();
    tab.disk_mtime = crate::recovery::file_mtime(&path);
    tab.path = Some(path);
    tab.recovered_from = None;
    Ok(())
}

/// Save tab `i`. On failure show why; when a file would be replaced, ask
/// first (`then` runs once the user agrees). True when the tab was saved
/// now, so the caller may carry on.
pub(super) fn save_and_report(
    app: &mut ReasonCraftApp,
    i: usize,
    save_as: bool,
    then: AfterSave,
) -> bool {
    let Some(tab) = app.tabs.get_mut(i) else {
        return false;
    };
    match save_tab_at(tab, &*app.dialogs, save_as) {
        Ok(SaveOutcome::Saved) => {
            note_saved(app, i);
            true
        }
        Ok(SaveOutcome::Cancelled) => false,
        Ok(SaveOutcome::NeedsReplaceConfirm(path)) => ask_replace(app, i, path, then, false),
        Ok(SaveOutcome::OriginalChanged(path)) => ask_replace(app, i, path, then, true),
        Err((path, err)) => {
            show_error(app, FileOp::Save, &path, &err);
            false
        }
    }
}

/// Ask before tab `tab`'s save replaces `path`; false (nothing saved yet).
pub(super) fn ask_replace(
    app: &mut ReasonCraftApp,
    tab: usize,
    path: PathBuf,
    then: AfterSave,
    original_changed: bool,
) -> bool {
    app.modal = Some(Modal::ConfirmReplace {
        tab,
        path,
        then,
        original_changed,
    });
    false
}

/// Save the active tab (Save / Save As).
pub fn save_active(app: &mut ReasonCraftApp, save_as: bool) {
    save_and_report(app, app.active, save_as, AfterSave::Nothing);
}

/// The answer to "replace the existing file?". Cancel ends the save and
/// whatever it was part of (a tab or window close).
pub fn resolve_replace(app: &mut ReasonCraftApp, replace: bool) {
    let Some(Modal::ConfirmReplace {
        tab, path, then, ..
    }) = app.modal.take()
    else {
        return;
    };
    let Some(doc_tab) = app.tabs.get_mut(tab).filter(|_| replace) else {
        return;
    };
    if let Err((path, err)) = write_tab(doc_tab, path) {
        show_error(app, FileOp::Save, &path, &err);
        return;
    }
    note_saved(app, tab);
    carry_on(app, tab, then);
}

/// Tab `i` was just saved: its file goes first in the recent files.
fn note_saved(app: &mut ReasonCraftApp, i: usize) {
    if let Some(path) = app.tabs.get(i).and_then(|t| t.path.clone()) {
        app.settings.push_recent(&path);
    }
}
