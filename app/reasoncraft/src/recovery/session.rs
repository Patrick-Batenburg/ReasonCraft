//! Autosave while the app runs: copies of tabs with unsaved work go to the
//! recovery folder every [`Settings::autosave_interval`] seconds, and go
//! away again once the tab is clean or closed.
//!
//! [`Settings::autosave_interval`]: crate::settings::Settings::autosave_interval

use std::path::Path;
use std::time::SystemTime;

use chrono::Utc;
use reasoncraft_core::Package;

use super::{RecoveryMeta, RecoveryStore};
use crate::app::ReasonCraftApp;
use crate::doc_tab::DocTab;

/// Run once per frame with the egui time `now` (seconds). Removes the copies
/// of tabs that became clean; when the interval has passed since the last
/// autosave, snapshots every tab whose unsaved work changed since its last
/// copy and hands it to the worker. Never waits for the disk.
pub fn tick(app: &mut ReasonCraftApp, now: f64) {
    let Some(store) = &app.recovery else {
        return;
    };
    for tab in &mut app.tabs {
        if tab.autosaved_revision != 0 && !tab.has_unsaved_work() {
            store.remove(tab.recovery_key);
            tab.autosaved_revision = 0;
        }
    }
    let Some(secs) = app.settings.autosave_interval() else {
        app.autosave_clock = None;
        return;
    };
    let since = *app.autosave_clock.get_or_insert(now);
    if now - since < f64::from(secs) {
        return;
    }
    app.autosave_clock = Some(now);
    for tab in app.tabs.iter_mut().filter(|t| needs_autosave(t)) {
        autosave_tab(store, tab);
    }
}

/// Seconds until the next autosave is due, when a tab is waiting for one:
/// the frame loop wakes up then even if nothing else happens.
pub(crate) fn due_in(app: &ReasonCraftApp, now: f64) -> Option<f64> {
    app.recovery.as_ref()?;
    let secs = f64::from(app.settings.autosave_interval()?);
    app.tabs.iter().any(needs_autosave).then(|| {
        let since = app.autosave_clock.unwrap_or(now);
        (since + secs - now).max(0.0)
    })
}

/// Unsaved work not in the recovery folder yet.
fn needs_autosave(tab: &DocTab) -> bool {
    tab.has_unsaved_work() && tab.revision() != tab.autosaved_revision
}

/// Snapshot `tab` (live edit-session text included: it is already in the
/// document) and send it to the worker.
pub(super) fn autosave_tab(store: &RecoveryStore, tab: &mut DocTab) {
    let meta = RecoveryMeta {
        original: tab.path.clone(),
        original_mtime: known_mtime(tab),
        title: tab.name(),
        saved_at: Utc::now(),
    };
    let pkg = Package {
        doc: tab.doc.clone(),
        assets: tab.assets.clone(),
        other_entries: tab.other_entries.clone(),
    };
    store.write(tab.recovery_key, pkg, meta);
    tab.autosaved_revision = tab.revision();
}

/// The modification time of the tab's file as the tab knows it: for a
/// restored tab the one recorded before the crash, else the file's own.
fn known_mtime(tab: &DocTab) -> Option<SystemTime> {
    let path = tab.path.as_deref()?;
    match &tab.recovered_from {
        Some(meta) if meta.original.as_deref() == Some(path) => meta.original_mtime,
        _ => mtime(path),
    }
}

fn mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// A tab is closing (saved or discarded): its copy goes.
pub(crate) fn forget(app: &ReasonCraftApp, tab: &DocTab) {
    if let Some(store) = &app.recovery
        && tab.autosaved_revision != 0
    {
        store.remove(tab.recovery_key);
    }
}

/// Whether saving restored tab `tab` to `path` would overwrite its
/// original file, changed since the copy was made (by someone else, or
/// another app).
pub(crate) fn original_changed(tab: &DocTab, path: &Path) -> bool {
    let Some(meta) = &tab.recovered_from else {
        return false;
    };
    meta.original.as_deref() == Some(path) && path.exists() && mtime(path) != meta.original_mtime
}

/// The app exits normally: end the recovery session (its copies go).
pub(crate) fn shutdown(app: &mut ReasonCraftApp) {
    if let Some(store) = app.recovery.take() {
        store.shutdown();
    }
}
