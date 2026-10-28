//! Saving tabs: the dialogs and the replace prompts, then the write (on a
//! worker in the app, see [`super::save_job`]).

use std::path::{Path, PathBuf};

use reasoncraft_core::file::FileError;

use super::save_job::{mark_written, snapshot, write};
use super::{AfterSave, EXTENSION, FileDialogs, Modal, SaveOutcome, with_rcraft_extension};
use crate::app::ReasonCraftApp;
use crate::doc_tab::DocTab;

/// Save `tab` to its path now, on this thread, asking for one when it has
/// none or `save_as` is set. Only [`SaveOutcome::Saved`] wrote the file;
/// otherwise, and on error, the tab keeps its path and stays dirty. (The
/// app writes on a worker instead: see [`save_active`].)
pub fn save_tab(
    tab: &mut DocTab,
    dialogs: &dyn FileDialogs,
    save_as: bool,
) -> Result<SaveOutcome, FileError> {
    let path = match target(tab, dialogs, save_as, None) {
        Ok(path) => path,
        Err(outcome) => return Ok(outcome),
    };
    let mtime = write(&snapshot(tab), &path)?;
    let revision = tab.revision();
    mark_written(tab, path, revision, mtime);
    Ok(SaveOutcome::Saved)
}

/// Where a save of `tab` writes, after ending its edit session: its path,
/// or one asked for when it has none or `save_as` is set. Otherwise how
/// the save ends before writing (cancelled, or a replace to confirm).
/// `pending`: where the last save of the tab still being written goes. A
/// Save writes there too (after a Save As, not to the old file), and that
/// file changing on disk is that save's doing.
fn target(
    tab: &mut DocTab,
    dialogs: &dyn FileDialogs,
    save_as: bool,
    pending: Option<&Path>,
) -> Result<PathBuf, SaveOutcome> {
    tab.commit_session();
    let path = match (pending.or(tab.path.as_deref()), save_as) {
        (Some(path), false) if pending.is_none() && changed_on_disk(tab, path) => {
            return Err(SaveOutcome::OriginalChanged(path.to_path_buf()));
        }
        (Some(path), false) => path.to_path_buf(),
        _ => {
            let suggested = format!("{}.{EXTENSION}", tab.file_stem());
            let Some(chosen) = dialogs.pick_save(&suggested) else {
                return Err(SaveOutcome::Cancelled);
            };
            let path = with_rcraft_extension(chosen.clone());
            // The dialog asked about overwriting `chosen`, not this file.
            if path != chosen && path.exists() {
                return Err(SaveOutcome::NeedsReplaceConfirm(path));
            }
            path
        }
    };
    Ok(path)
}

/// Whether saving `tab` to its file `path` would overwrite changes made
/// there since the tab opened or last saved it (or, for a restored tab,
/// since the crashed app did): by someone else, or another app.
fn changed_on_disk(tab: &DocTab, path: &Path) -> bool {
    path.exists() && crate::recovery::file_mtime(path) != tab.disk_mtime
}

/// Save tab `i`: start writing it (`then` carries on once it is written,
/// see [`super::save_job::poll_saves`]), or, when a file would be
/// replaced, ask first. A cancelled dialog ends the save and what it was
/// part of.
pub(super) fn save_and_report(app: &mut ReasonCraftApp, i: usize, save_as: bool, then: AfterSave) {
    let Some(tab) = app.tabs.get_mut(i) else {
        return;
    };
    let pending = app.saves.pending_path(tab).map(Path::to_path_buf);
    match target(tab, &*app.dialogs, save_as, pending.as_deref()) {
        Ok(path) => app.saves.start(tab, path, then),
        Err(SaveOutcome::NeedsReplaceConfirm(path)) => ask_replace(app, i, path, then, false),
        Err(SaveOutcome::OriginalChanged(path)) => ask_replace(app, i, path, then, true),
        Err(SaveOutcome::Cancelled | SaveOutcome::Saved) => {}
    }
}

/// Ask before tab `tab`'s save replaces `path`.
fn ask_replace(
    app: &mut ReasonCraftApp,
    tab: usize,
    path: PathBuf,
    then: AfterSave,
    original_changed: bool,
) {
    app.modal = Some(Modal::ConfirmReplace {
        tab: app.tabs[tab].recovery_key,
        path,
        then,
        original_changed,
    });
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
    let Some(doc_tab) = (app.tabs.iter())
        .find(|t| t.recovery_key == tab)
        .filter(|_| replace)
    else {
        return;
    };
    app.saves.start(doc_tab, path, then);
}

/// Tab `i` was just saved: its file goes first in the recent files.
pub(super) fn note_saved(app: &mut ReasonCraftApp, i: usize) {
    if let Some(path) = app.tabs.get(i).and_then(|t| t.path.clone()) {
        app.settings.push_recent(&path);
    }
}
