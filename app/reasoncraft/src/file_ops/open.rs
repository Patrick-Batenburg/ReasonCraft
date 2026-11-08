//! Opening maps: from the dialog, the recent files, the command line and
//! later launches; finding the tab that shows a file.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use reasoncraft_core::Uuid;
use reasoncraft_core::file::{self, FileError};

use super::{FileOp, remove_tab, show_error, tab_index};
use crate::app::ReasonCraftApp;
use crate::doc_tab::DocTab;

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

/// Open the files named on the command line; the first one that fails to
/// open (also one that does not exist, or a folder) is shown.
pub fn open_paths(app: &mut ReasonCraftApp, args: impl IntoIterator<Item = OsString>) {
    if let Some((path, err)) = open_all(app, args.into_iter().map(PathBuf::from)) {
        // The recovery prompt of this start shows once the error is closed.
        crate::recovery::defer_offer(app);
        show_error(app, FileOp::Open, &path, &err);
    }
}

/// Open the files a later launch handed over (see [`crate::instance`]),
/// leaving the backstage so they show; the first that fails is shown.
pub fn open_handed_over(app: &mut ReasonCraftApp, paths: Vec<PathBuf>) {
    if paths.is_empty() {
        return;
    }
    if app.ribbon_tab == crate::app::RibbonTab::File {
        app.ribbon_tab = crate::app::RibbonTab::Home;
    }
    if let Some((path, err)) = open_all(app, paths) {
        show_error(app, FileOp::Open, &path, &err);
    }
}

/// Open each of `paths`, an open one by activating its tab; returns the
/// first that failed, and why.
pub(crate) fn open_all(
    app: &mut ReasonCraftApp,
    paths: impl IntoIterator<Item = PathBuf>,
) -> Option<(PathBuf, FileError)> {
    let mut first_error = None;
    for path in paths {
        let opened = if path.is_dir() {
            Err(FileError::NotAReasonCraftFile)
        } else {
            try_open(app, &path)
        };
        if let Err(err) = opened {
            first_error.get_or_insert((path, err));
        }
    }
    first_error
}

/// Whether `tab` shows the file `path`, whose resolved form is `resolved`
/// (links and short names followed; `None` when it cannot be resolved,
/// then only the path as given counts). The tab's own resolved path was
/// kept when it opened or saved the file, so this does not touch the disk
/// (but for a tab that never did, such as a restored one).
pub(crate) fn shows_file(tab: &DocTab, path: &Path, resolved: Option<&Path>) -> bool {
    let Some(own) = tab.path.as_deref() else {
        return false;
    };
    if own == path {
        return true;
    }
    let Some(resolved) = resolved else {
        return false;
    };
    match &tab.resolved_path {
        Some(own) => own == resolved,
        None => own.canonicalize().is_ok_and(|own| own == resolved),
    }
}

/// Tab `key` was just saved: any other tab showing the same file would
/// show what is no longer in it (and Open would find that tab first). It
/// closes when it has no unsaved work, else it becomes an untitled,
/// unsaved copy, so its work stays and no save overwrites the file.
pub(crate) fn release_other_tabs(app: &mut ReasonCraftApp, key: Uuid) {
    let Some(saved) = tab_index(app, key) else {
        return;
    };
    let Some(path) = app.tabs[saved].path.clone() else {
        return;
    };
    let resolved = app.tabs[saved].resolved_path.clone();
    while let Some(j) = (app.tabs.iter())
        .position(|t| t.recovery_key != key && shows_file(t, &path, resolved.as_deref()))
    {
        if app.tabs[j].has_unsaved_work() {
            as_unsaved_copy(&mut app.tabs[j]);
        } else {
            remove_tab(app, j);
        }
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
    crate::instance::log_opened(&path);
    Ok(())
}

fn open_or_activate(app: &mut ReasonCraftApp, path: &Path) -> Result<(), FileError> {
    // On this thread: the file is read here anyway.
    let resolved = path.canonicalize().ok();
    let open_tab = (app.tabs.iter()).position(|tab| shows_file(tab, path, resolved.as_deref()));
    if let Some(i) = open_tab {
        app.active = i;
        return Ok(());
    }
    let pkg = file::open_package(path)?;
    if let Some(note) = open_note(&pkg) {
        app.status_note = Some(note);
    }
    let cut = pkg.cut_on_open;
    let mut tab = DocTab::from_package(pkg);
    if cut {
        as_unsaved_copy(&mut tab);
    } else {
        tab.path = Some(path.to_path_buf());
        tab.resolved_path = resolved;
        tab.disk_mtime = crate::recovery::file_mtime(path);
    }
    place_tab(app, tab);
    Ok(())
}

/// What the status bar says about an opened `pkg`: that something was
/// cut or left out, that branches were parked; `None` when neither.
pub(crate) fn open_note(pkg: &reasoncraft_core::Package) -> Option<String> {
    let mut notes = Vec::new();
    if pkg.cut_on_open {
        notes.push(crate::i18n::t("status.open_cut"));
    }
    if pkg.parked_on_open {
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("max", reasoncraft_core::MAX_DEPTH);
        notes.push(crate::i18n::t_args("status.open_too_deep", &args));
    }
    (!notes.is_empty()).then(|| notes.join(" "))
}

/// A document that lost part of its file on opening is an unsaved copy:
/// untitled and dirty, so no save overwrites the file (which still holds
/// what was cut) without the user choosing it in the save dialog.
pub(crate) fn as_unsaved_copy(tab: &mut DocTab) {
    tab.path = None;
    tab.resolved_path = None;
    tab.disk_mtime = None;
    tab.history.saved_marker = None;
}

/// Show a loaded `tab` in place of a tab showing its file without unsaved
/// work (a restored copy of a file open already), else in the active tab
/// when that one is untouched, else in a new tab; it becomes active.
/// Returns its index.
pub(crate) fn place_tab(app: &mut ReasonCraftApp, mut tab: DocTab) -> usize {
    let same_file_tab = tab.path.as_deref().and_then(|path| {
        let resolved = (tab.resolved_path.clone()).or_else(|| path.canonicalize().ok());
        (app.tabs.iter())
            .position(|t| !t.has_unsaved_work() && shows_file(t, path, resolved.as_deref()))
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
