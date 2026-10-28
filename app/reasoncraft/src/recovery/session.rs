//! Autosave while the app runs: copies of tabs with unsaved work go to the
//! recovery folder every [`Settings::autosave_interval`] seconds, and go
//! away again once the tab is clean or closed.
//!
//! [`Settings::autosave_interval`]: crate::settings::Settings::autosave_interval

use std::path::Path;

use chrono::Utc;
use reasoncraft_core::Package;

use super::{RecoveredDoc, RecoveryMeta, RecoveryStore};
use crate::app::ReasonCraftApp;
use crate::doc_tab::DocTab;
use crate::i18n::{FluentArgs, t_args};

/// [`DocTab::autosaved_revision`] after a failed write: no revision
/// matches it, so the next autosave tries again, and the tab still counts
/// as having a copy (an older one may be there), so it is removed once the
/// tab is clean or closed.
const RETRY: Option<u64> = Some(u64::MAX);

/// Run once per frame with the egui time `now` (seconds). Shows a recovery
/// prompt that waited for another message to close, and notes failed
/// writes (their tabs are written again next time). Removes the copies
/// of tabs that became clean; when the interval has passed since the last
/// autosave, snapshots every tab whose unsaved work changed since its last
/// copy and hands it to the worker. Never waits for the disk.
pub fn tick(app: &mut ReasonCraftApp, now: f64) {
    super::prompt::resume_offer(app);
    let Some(store) = &app.recovery else {
        return;
    };
    let failures = store.failed_writes();
    if let Some(last) = failures.last() {
        let mut args = FluentArgs::new();
        args.set("detail", last.reason.clone());
        app.status_note = Some(t_args("recovery.write_failed", &args));
    }
    for failure in &failures {
        let tab = app
            .tabs
            .iter_mut()
            .find(|t| t.recovery_key == failure.tab_key);
        if let Some(tab) = tab {
            tab.autosaved_revision = RETRY;
        }
    }
    for tab in &mut app.tabs {
        if tab.autosaved_revision.is_some() && !tab.has_unsaved_work() {
            store.remove(tab.recovery_key);
            tab.autosaved_revision = None;
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
        autosave_tab(store, tab, None);
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
    tab.has_unsaved_work() && Some(tab.revision()) != tab.autosaved_revision
}

/// Snapshot `tab` (live edit-session text included: it is already in the
/// document) and send it to the worker. `replaces`: the orphaned copy the
/// tab was restored from, deleted only once this copy is written.
pub(super) fn autosave_tab(
    store: &RecoveryStore,
    tab: &mut DocTab,
    replaces: Option<&RecoveredDoc>,
) {
    let meta = RecoveryMeta {
        original: tab.path.clone(),
        original_mtime: tab.disk_mtime,
        title: tab.name(),
        saved_at: Utc::now(),
    };
    let pkg = Package {
        doc: tab.doc.clone(),
        assets: tab.assets.clone(),
        other_entries: tab.other_entries.clone(),
        cut_on_open: false,
    };
    match replaces {
        Some(doc) => store.write_then_discard(tab.recovery_key, pkg, meta, doc),
        None => store.write(tab.recovery_key, pkg, meta),
    }
    tab.autosaved_revision = Some(tab.revision());
}

/// A tab is closing (saved or discarded): its copy goes.
pub(crate) fn forget(app: &ReasonCraftApp, tab: &DocTab) {
    if let Some(store) = &app.recovery
        && tab.autosaved_revision.is_some()
    {
        store.remove(tab.recovery_key);
    }
}

/// When `path` was last modified, if it can be read.
pub(crate) fn file_mtime(path: &Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// The app exits normally: end the recovery session (its copies go).
pub(crate) fn shutdown(app: &mut ReasonCraftApp) {
    if let Some(store) = app.recovery.take() {
        store.shutdown();
    }
}
