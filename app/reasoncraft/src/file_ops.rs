//! New, Open, Save, Save As and the unsaved-changes prompts.
//!
//! File dialogs sit behind [`FileDialogs`], so tests drive every flow with
//! [`FakeDialogs`]. Failures never touch open work: a failed open changes no
//! tabs, and a failed save keeps the tab dirty and its old path.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use reasoncraft_core::Package;
use reasoncraft_core::file::{self, FileError};

use crate::actions::UnsavedChoice;
use crate::app::ReasonCraftApp;
use crate::doc_tab::DocTab;

mod dialogs;
mod errors;
pub use dialogs::{FakeDialogs, FileDialogs, RfdDialogs};
pub(crate) use errors::show_error;
pub use errors::{error_key, error_text};

/// The file extension of ReasonCraft maps.
pub const EXTENSION: &str = "rcraft";

/// A dialog shown over the whole window.
#[derive(Clone, Debug, PartialEq)]
pub enum Modal {
    Error {
        title: String,
        message: String,
    },
    /// Tab `tab` has unsaved changes and is about to close. `window`: the
    /// whole window is closing, so the remaining dirty tabs come next.
    /// `waiting`: the user chose Save, which happens once the tab's images
    /// are prepared (Cancel still works meanwhile).
    Unsaved {
        tab: usize,
        window: bool,
        waiting: bool,
    },
    /// Saving tab `tab` to `path` would replace an existing file that the
    /// save dialog never checked (`.rcraft` was appended to the chosen name),
    /// or, `original_changed`, a restored tab's original that changed since.
    ConfirmReplace {
        tab: usize,
        path: PathBuf,
        then: AfterSave,
        original_changed: bool,
    },
    /// The window is closing, every tab is saved or discarded, and exports
    /// are still running: it closes once they finish (or on Quit anyway).
    ExportsRunning,
    /// An export to `path` would replace an existing file that the file
    /// dialog never checked (the extension was added to the chosen name).
    ConfirmExportReplace {
        path: PathBuf,
        request: crate::export::ExportRequest,
    },
    /// A crashed session left these documents behind: restore them?
    Recover(Vec<crate::recovery::RecoveredDoc>),
}

/// What a save was part of, so it can carry on once the file is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AfterSave {
    Nothing,
    /// Close the tab (the unsaved-changes prompt of a tab close).
    CloseTab,
    /// Ask about the next dirty tab (the prompt of a window close).
    ContinueWindowClose,
}

/// How a save ended, short of an error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SaveOutcome {
    Saved,
    /// The save dialog was cancelled; nothing was written.
    Cancelled,
    /// Nothing was written yet: `.rcraft` was appended to the chosen name
    /// and that file exists, so the user must confirm replacing it.
    NeedsReplaceConfirm(PathBuf),
    /// Nothing was written yet: the tab was restored after a crash and its
    /// original file changed since, so the user must confirm replacing it.
    OriginalChanged(PathBuf),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileOp {
    Open,
    Save,
}

/// `path` with `.rcraft` appended unless it already ends in it (any case).
pub fn with_rcraft_extension(path: PathBuf) -> PathBuf {
    with_extension(path, EXTENSION)
}

/// `path` with `.ext` appended unless it already ends in it (any case).
pub fn with_extension(path: PathBuf, ext: &str) -> PathBuf {
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(ext))
    {
        return path;
    }
    let mut name = OsString::from(path.as_os_str());
    name.push(".");
    name.push(ext);
    PathBuf::from(name)
}

/// Ask for a file and open it.
pub fn open_with_dialog(app: &mut ReasonCraftApp) {
    if let Some(path) = app.dialogs.pick_open() {
        open_into_app(app, &path);
    }
}

/// Open `path` in a tab, or show why it cannot be opened (no tab changes).
pub fn open_into_app(app: &mut ReasonCraftApp, path: &Path) {
    if let Err(err) = try_open(app, path) {
        show_error(app, FileOp::Open, path, &err);
    }
}

/// Open the files named on the command line. Arguments that are not
/// existing files are skipped; the first file that fails to open is shown.
pub fn open_paths(app: &mut ReasonCraftApp, args: impl IntoIterator<Item = OsString>) {
    let mut first_error = None;
    for path in args.into_iter().map(PathBuf::from).filter(|p| p.is_file()) {
        if let Err(err) = try_open(app, &path) {
            first_error.get_or_insert((path, err));
        }
    }
    if let Some((path, err)) = first_error {
        // The recovery prompt of this start shows once the error is closed.
        crate::recovery::defer_offer(app);
        show_error(app, FileOp::Open, &path, &err);
    }
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

/// A tab nobody has worked in: untitled, with no history and no edit.
fn is_untouched(tab: &DocTab) -> bool {
    tab.path.is_none()
        && tab.history.undo.is_empty()
        && tab.history.redo.is_empty()
        && !tab.is_dirty()
}

/// Open `path`: activate its tab if it is already open, else load it into
/// the active tab when that one is untouched, else into a new tab.
fn try_open(app: &mut ReasonCraftApp, path: &Path) -> Result<(), FileError> {
    let open_tab = app
        .tabs
        .iter()
        .position(|tab| tab.path.as_deref().is_some_and(|p| same_file(p, path)));
    if let Some(i) = open_tab {
        app.active = i;
        return Ok(());
    }
    let mut tab = DocTab::from_package(file::open_package(path)?);
    tab.path = Some(path.to_path_buf());
    tab.disk_mtime = file_mtime(path);
    place_tab(app, tab);
    Ok(())
}

/// Show a loaded `tab` in the active tab when that one is untouched, else
/// in a new tab; it becomes active. Returns its index.
pub(crate) fn place_tab(app: &mut ReasonCraftApp, mut tab: DocTab) -> usize {
    let reused = app.tabs.get(app.active).filter(|t| is_untouched(t));
    tab.mark_loaded(reused.map_or(0, DocTab::revision));
    if reused.is_some() {
        app.tabs[app.active] = tab;
    } else {
        app.tabs.push(tab);
        app.active = app.tabs.len() - 1;
    }
    app.active
}

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
fn save_tab_at(
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
fn write_tab(tab: &mut DocTab, path: PathBuf) -> Result<(), (PathBuf, FileError)> {
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
    tab.disk_mtime = file_mtime(&path);
    tab.path = Some(path);
    tab.recovered_from = None;
    Ok(())
}

/// When `path` was last modified, if it can be read.
pub(crate) fn file_mtime(path: &Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// Save tab `i`. On failure show why; when a file would be replaced, ask
/// first (`then` runs once the user agrees). True when the tab was saved
/// now, so the caller may carry on.
fn save_and_report(app: &mut ReasonCraftApp, i: usize, save_as: bool, then: AfterSave) -> bool {
    let Some(tab) = app.tabs.get_mut(i) else {
        return false;
    };
    match save_tab_at(tab, &*app.dialogs, save_as) {
        Ok(SaveOutcome::Saved) => true,
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
fn ask_replace(
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
    carry_on(app, tab, then);
}

/// Continue what a save of tab `tab` was part of.
fn carry_on(app: &mut ReasonCraftApp, tab: usize, then: AfterSave) {
    match then {
        AfterSave::Nothing => {}
        AfterSave::CloseTab => remove_tab(app, tab),
        AfterSave::ContinueWindowClose => ask_from(app, tab + 1),
    }
}

/// Close tab `i`, asking first if it has unsaved changes (or an image
/// still being prepared).
pub fn request_close_tab(app: &mut ReasonCraftApp, i: usize) {
    let Some(tab) = app.tabs.get_mut(i) else {
        return;
    };
    tab.commit_session();
    if tab.has_unsaved_work() {
        app.modal = Some(Modal::Unsaved {
            tab: i,
            window: false,
            waiting: false,
        });
    } else {
        remove_tab(app, i);
    }
}

/// Remove tab `i` without asking. The window always keeps one tab.
pub fn remove_tab(app: &mut ReasonCraftApp, i: usize) {
    if i >= app.tabs.len() {
        return;
    }
    let tab = app.tabs.remove(i);
    crate::recovery::forget(app, &tab);
    if app.tabs.is_empty() {
        app.tabs.push(DocTab::new());
    }
    if app.active > i || app.active >= app.tabs.len() {
        app.active = app.active.saturating_sub(1);
    }
}

/// Close the window, asking about each dirty tab in order first.
pub fn request_close_window(app: &mut ReasonCraftApp) {
    for tab in &mut app.tabs {
        tab.commit_session();
    }
    ask_from(app, 0);
}

/// Ask about the first dirty tab at or after `from`; with none left, close
/// (once the running exports finish).
fn ask_from(app: &mut ReasonCraftApp, from: usize) {
    match (from..app.tabs.len()).find(|&i| app.tabs[i].has_unsaved_work()) {
        Some(i) => {
            app.active = i;
            app.modal = Some(Modal::Unsaved {
                tab: i,
                window: true,
                waiting: false,
            });
        }
        None => crate::actions::close_after_exports(app),
    }
}

/// The answer to the unsaved-changes prompt.
pub fn resolve_unsaved(app: &mut ReasonCraftApp, choice: UnsavedChoice) {
    let Some(Modal::Unsaved { tab, window, .. }) = app.modal.take() else {
        return;
    };
    let then = if window {
        AfterSave::ContinueWindowClose
    } else {
        AfterSave::CloseTab
    };
    let proceed = match choice {
        UnsavedChoice::Cancel => false,
        UnsavedChoice::Discard => true,
        // A failed or cancelled save ends the close (a failure shows its
        // error); after a replace prompt, `then` runs once it is answered.
        UnsavedChoice::Save => save_and_report(app, tab, false, then),
    };
    if proceed {
        carry_on(app, tab, then);
    }
}

#[cfg(test)]
mod tests;
