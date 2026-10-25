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
use crate::i18n::{FluentArgs, t_args};

mod dialogs;
pub use dialogs::{FakeDialogs, FileDialogs, RfdDialogs};

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
    Unsaved {
        tab: usize,
        window: bool,
    },
    /// Saving tab `tab` to `path` would replace an existing file that the
    /// save dialog never checked (`.rcraft` was appended to the chosen name).
    ConfirmReplace {
        tab: usize,
        path: PathBuf,
        then: AfterSave,
    },
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileOp {
    Open,
    Save,
}

/// `path` with `.rcraft` appended unless it already ends in it (any case).
pub fn with_rcraft_extension(path: PathBuf) -> PathBuf {
    let has_ext = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(EXTENSION));
    if has_ext {
        return path;
    }
    let mut name = OsString::from(path.as_os_str());
    name.push(".");
    name.push(EXTENSION);
    PathBuf::from(name)
}

/// The i18n key that explains `err`.
pub fn error_key(op: FileOp, err: &FileError) -> &'static str {
    match (op, err) {
        (FileOp::Open, FileError::Io(_)) => "error.open.io",
        (FileOp::Open, FileError::NotAReasonCraftFile) => "error.open.not_a_file",
        (FileOp::Open, FileError::MissingDocument) => "error.open.missing_document",
        (FileOp::Open, FileError::Malformed(_)) => "error.open.malformed",
        (FileOp::Open, FileError::NewerVersion { .. }) => "error.open.newer_version",
        (FileOp::Save, FileError::Io(_)) => "error.save.io",
        (FileOp::Save, _) => "error.save.other",
    }
}

/// The user-facing explanation of `err`.
pub fn error_text(op: FileOp, err: &FileError) -> String {
    let mut args = FluentArgs::new();
    match err {
        FileError::NewerVersion { found, supported } => {
            args.set("found", *found);
            args.set("supported", *supported);
        }
        FileError::Io(e) => args.set("detail", e.to_string()),
        FileError::Malformed(detail) => args.set("detail", detail.clone()),
        FileError::NotAReasonCraftFile | FileError::MissingDocument => {
            args.set("detail", err.to_string());
        }
    }
    t_args(error_key(op, err), &args)
}

fn show_error(app: &mut ReasonCraftApp, op: FileOp, path: &Path, err: &FileError) {
    let mut args = FluentArgs::new();
    let file = path.file_name().unwrap_or(path.as_os_str());
    args.set("file", file.to_string_lossy().into_owned());
    let title_key = match op {
        FileOp::Open => "error.open.title",
        FileOp::Save => "error.save.title",
    };
    app.modal = Some(Modal::Error {
        title: t_args(title_key, &args),
        message: error_text(op, err),
    });
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
    let reused = app.tabs.get(app.active).filter(|t| is_untouched(t));
    tab.mark_loaded(reused.map_or(0, DocTab::revision));
    if reused.is_some() {
        app.tabs[app.active] = tab;
    } else {
        app.tabs.push(tab);
        app.active = app.tabs.len() - 1;
    }
    Ok(())
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
    tab.path = Some(path);
    tab.history.mark_saved();
    Ok(())
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
        Ok(SaveOutcome::NeedsReplaceConfirm(path)) => {
            app.modal = Some(Modal::ConfirmReplace { tab: i, path, then });
            false
        }
        Err((path, err)) => {
            show_error(app, FileOp::Save, &path, &err);
            false
        }
    }
}

/// Save the active tab (Save / Save As).
pub fn save_active(app: &mut ReasonCraftApp, save_as: bool) {
    save_and_report(app, app.active, save_as, AfterSave::Nothing);
}

/// The answer to "replace the existing file?". Cancel ends the save and
/// whatever it was part of (a tab or window close).
pub fn resolve_replace(app: &mut ReasonCraftApp, replace: bool) {
    let Some(Modal::ConfirmReplace { tab, path, then }) = app.modal.take() else {
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

/// Close tab `i`, asking first if it has unsaved changes.
pub fn request_close_tab(app: &mut ReasonCraftApp, i: usize) {
    let Some(tab) = app.tabs.get_mut(i) else {
        return;
    };
    tab.commit_session();
    if tab.is_dirty() {
        app.modal = Some(Modal::Unsaved {
            tab: i,
            window: false,
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
    app.tabs.remove(i);
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

/// Ask about the first dirty tab at or after `from`; with none left, close.
fn ask_from(app: &mut ReasonCraftApp, from: usize) {
    match (from..app.tabs.len()).find(|&i| app.tabs[i].is_dirty()) {
        Some(i) => {
            app.active = i;
            app.modal = Some(Modal::Unsaved {
                tab: i,
                window: true,
            });
        }
        None => {
            app.modal = None;
            app.confirm_close();
        }
    }
}

/// The answer to the unsaved-changes prompt.
pub fn resolve_unsaved(app: &mut ReasonCraftApp, choice: UnsavedChoice) {
    let Some(Modal::Unsaved { tab, window }) = app.modal.take() else {
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
