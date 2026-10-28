//! New, Open, Save, Save As and the unsaved-changes prompts.
//!
//! File dialogs sit behind [`FileDialogs`], so tests drive every flow with
//! [`FakeDialogs`]. Failures never touch open work: a failed open changes no
//! tabs, and a failed save keeps the tab dirty and its old path.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use reasoncraft_core::Uuid;
use reasoncraft_core::file::{self, FileError};

use crate::actions::UnsavedChoice;
use crate::app::ReasonCraftApp;
use crate::doc_tab::DocTab;

mod dialogs;
mod errors;
mod save;
mod save_job;
pub use dialogs::{FakeDialogs, FileDialogs, RfdDialogs};
pub(crate) use errors::show_error;
pub use errors::{error_key, error_text};
use save::save_and_report;
pub use save::{resolve_replace, save_active, save_tab};
pub use save_job::{SAVE_CLOSE_WAIT, SaveJobs, poll_saves, wait_for_saves};

/// The file extension of ReasonCraft maps.
pub const EXTENSION: &str = "rcraft";

/// A dialog shown over the whole window. Tabs are named by their
/// [`DocTab::recovery_key`]: a save ending behind the dialog may close a
/// tab before them.
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
        tab: Uuid,
        window: bool,
        waiting: bool,
    },
    /// Saving tab `tab` to `path` would replace an existing file that the
    /// save dialog never checked (`.rcraft` was appended to the chosen name),
    /// or, `original_changed`, the tab's file, changed on disk since the
    /// tab opened or saved it (or, restored, since the crashed app did).
    ConfirmReplace {
        tab: Uuid,
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
    Recover(crate::recovery::Orphans),
    /// An Argdown import changed or left out these constructs.
    ImportReport(Vec<reasoncraft_core::ImportNote>),
    /// Put every setting back to its default?
    ResetSettings,
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
    /// Nothing was written yet: the tab's file changed on disk since the
    /// tab opened or saved it, so the user must confirm replacing it.
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

/// Open `path` from the recent-files list. When it cannot be opened, say
/// why; when it is gone, it also leaves the list.
pub fn open_recent(app: &mut ReasonCraftApp, path: &Path) {
    if let Err(err) = try_open(app, path) {
        if matches!(path.try_exists(), Ok(false)) {
            app.settings.recent_files.retain(|p| p != path);
        }
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

/// Whether `a` and `b` name the same file (compared as given when either
/// cannot be resolved).
pub(crate) fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

/// A blank tab nobody has worked in: untitled, with no history, no edit,
/// no image being prepared and only an empty claim (an example or
/// template opened clean is not blank, so it stays).
fn is_untouched(tab: &DocTab) -> bool {
    tab.path.is_none()
        && tab.history.undo.is_empty()
        && tab.history.redo.is_empty()
        && !tab.has_unsaved_work()
        && tab.doc.boxes.len() == 1
        && tab
            .doc
            .boxes
            .get(&tab.doc.root)
            .is_some_and(|b| b.text.is_empty())
}

/// Open `path`: activate its tab if it is already open, else load it into
/// the active tab when that one is untouched, else into a new tab. Either
/// way it goes first in the recent files. A relative path (from the
/// command line) is made absolute first: the tab, the recent files and a
/// recovered copy must not depend on the folder the app was started in.
fn try_open(app: &mut ReasonCraftApp, path: &Path) -> Result<(), FileError> {
    let path = std::path::absolute(path)?;
    open_or_activate(app, &path)?;
    app.settings.push_recent(&path);
    Ok(())
}

fn open_or_activate(app: &mut ReasonCraftApp, path: &Path) -> Result<(), FileError> {
    let open_tab = app
        .tabs
        .iter()
        .position(|tab| tab.path.as_deref().is_some_and(|p| same_file(p, path)));
    if let Some(i) = open_tab {
        app.active = i;
        return Ok(());
    }
    let pkg = file::open_package(path)?;
    if pkg.cut_on_open {
        app.status_note = Some(crate::i18n::t("status.open_cut"));
    }
    let mut tab = DocTab::from_package(pkg);
    tab.path = Some(path.to_path_buf());
    tab.disk_mtime = crate::recovery::file_mtime(path);
    place_tab(app, tab);
    Ok(())
}

/// Show a loaded `tab` in place of a tab showing its file without unsaved
/// work (a restored copy of a file open already), else in the active tab
/// when that one is untouched, else in a new tab; it becomes active.
/// Returns its index.
pub(crate) fn place_tab(app: &mut ReasonCraftApp, mut tab: DocTab) -> usize {
    let same_file_tab = tab.path.as_deref().and_then(|path| {
        app.tabs.iter().position(|t| {
            !t.has_unsaved_work() && t.path.as_deref().is_some_and(|p| same_file(p, path))
        })
    });
    let untouched = Some(app.active).filter(|&i| app.tabs.get(i).is_some_and(is_untouched));
    match same_file_tab.or(untouched) {
        Some(i) => {
            tab.mark_loaded(app.tabs[i].revision());
            app.tabs[i] = tab;
            app.active = i;
        }
        None => {
            app.tabs.push(tab);
            app.active = app.tabs.len() - 1;
        }
    }
    app.active
}

/// Where the tab `key` names is now, if it is still open.
pub(crate) fn tab_index(app: &ReasonCraftApp, key: Uuid) -> Option<usize> {
    app.tabs.iter().position(|t| t.recovery_key == key)
}

/// Continue what a save of tab `tab` was part of.
fn carry_on(app: &mut ReasonCraftApp, tab: Uuid, then: AfterSave) {
    let Some(i) = tab_index(app, tab) else {
        return;
    };
    match then {
        AfterSave::Nothing => {}
        AfterSave::CloseTab => remove_tab(app, i),
        AfterSave::ContinueWindowClose => ask_from(app, i + 1),
    }
}

/// Close the tab `key` names, asking first if it has unsaved changes (or
/// an image still being prepared). Saves being written finish first; one
/// may close another tab, so the tab is looked for after them.
pub fn request_close_tab(app: &mut ReasonCraftApp, key: Uuid) {
    wait_for_saves(app);
    if let Some(i) = tab_index(app, key) {
        ask_or_close(app, i);
    }
}

/// Close tab `i`, asking first if it has unsaved changes.
fn ask_or_close(app: &mut ReasonCraftApp, i: usize) {
    let tab = &mut app.tabs[i];
    tab.commit_session();
    if tab.has_unsaved_work() {
        app.modal = Some(Modal::Unsaved {
            tab: tab.recovery_key,
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

/// Close the window, asking about each dirty tab in order first (saves
/// being written finish first). The link editor and the export dialog
/// close first: they act on the active tab, and asking may turn to
/// another one.
pub fn request_close_window(app: &mut ReasonCraftApp) {
    wait_for_saves(app);
    app.link_edit = None;
    app.export_dialog = None;
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
                tab: app.tabs[i].recovery_key,
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
    match choice {
        UnsavedChoice::Cancel => {}
        UnsavedChoice::Discard => carry_on(app, tab, then),
        // `then` runs once the file is written (after a replace prompt,
        // once it is answered); a failed or cancelled save ends the close
        // (a failure shows its error).
        UnsavedChoice::Save => {
            if let Some(i) = tab_index(app, tab) {
                save_and_report(app, i, false, then);
            }
        }
    }
}

#[cfg(test)]
mod open_tests;
#[cfg(test)]
mod save_job_tests;
#[cfg(test)]
mod tests;
