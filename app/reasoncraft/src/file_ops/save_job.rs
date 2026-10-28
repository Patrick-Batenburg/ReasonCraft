//! Saves run on a worker thread, like autosave, so a slow disk (a network
//! or synced folder) never freezes the window. They run one after another,
//! so two saves of one file never interleave. The frame loop picks up each
//! result ([`poll_saves`]): the tab takes the path and counts as saved,
//! unless it changed while the file was written, and what the save was
//! part of (a tab or window close) carries on.

use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

use reasoncraft_core::file::{self, FileError};
use reasoncraft_core::{Package, Uuid};

use super::{AfterSave, FileOp, ask_from, remove_tab, request_close_tab, show_error};
use crate::app::ReasonCraftApp;
use crate::doc_tab::DocTab;
use crate::jobs::{Hold, Job, JobState, Queue};

/// What writing a file gave: its modification time, when it can be read.
type WriteResult = Result<Option<SystemTime>, FileError>;

struct Running {
    /// The tab's [`DocTab::recovery_key`]: tabs may close or move meanwhile.
    tab: Uuid,
    path: PathBuf,
    /// The tab's revision when it was snapshotted.
    revision: u64,
    then: AfterSave,
    job: Job<WriteResult>,
}

/// The saves being written, for all tabs.
#[derive(Default)]
pub struct SaveJobs {
    running: Vec<Running>,
    queue: Queue,
    /// Hold each save back until released, before it writes (tests only).
    pub hold: Option<Hold>,
}

impl SaveJobs {
    /// Whether a save is still being written.
    pub fn busy(&self) -> bool {
        !self.running.is_empty()
    }

    /// Whether a save of `tab` is being written.
    pub fn saving(&self, tab: &DocTab) -> bool {
        self.running.iter().any(|r| r.tab == tab.recovery_key)
    }

    /// Start writing `tab` as it is now to `path`; `then` carries on once
    /// it is written.
    pub(super) fn start(&mut self, tab: &DocTab, path: PathBuf, then: AfterSave) {
        let pkg = snapshot(tab);
        let to = path.clone();
        let job = (self.queue).push(self.hold.clone(), move || write(&pkg, &to));
        self.running.push(Running {
            tab: tab.recovery_key,
            path,
            revision: tab.revision(),
            then,
            job,
        });
    }

    /// The saves that ended since the last call, in the order started. A
    /// worker that died reads as an I/O error.
    fn finished(&mut self) -> Vec<(Running, WriteResult)> {
        let mut done = Vec::new();
        let mut still = Vec::new();
        for r in std::mem::take(&mut self.running) {
            let result = match r.job.poll() {
                JobState::Running => {
                    still.push(r);
                    continue;
                }
                JobState::Done(result) => result,
                JobState::Lost => Err(std::io::Error::other("the save stopped").into()),
            };
            done.push((r, result));
        }
        self.running = still;
        done
    }
}

/// What a save of `tab` writes. Only the assets the document refers to are
/// written; the tab keeps the rest, so a redo after the save still finds
/// its image. The bytes are shared, so nothing large is copied.
pub(super) fn snapshot(tab: &DocTab) -> Package {
    Package {
        doc: tab.doc.clone(),
        assets: tab.assets.clone(),
        other_entries: tab.other_entries.clone(),
        cut_on_open: false,
    }
}

/// Write `pkg` to `path` (on the worker).
pub(super) fn write(pkg: &Package, path: &std::path::Path) -> WriteResult {
    file::save_package(pkg, path)?;
    Ok(crate::recovery::file_mtime(path))
}

/// `tab`, snapshotted at `revision`, was written to `path`: it takes the
/// path, and counts as saved unless it changed since (then it stays
/// unsaved until saved again, even undone back to what was written).
pub(super) fn mark_written(
    tab: &mut DocTab,
    path: PathBuf,
    revision: u64,
    mtime: Option<SystemTime>,
) {
    if tab.revision() == revision {
        tab.history.mark_saved();
    } else {
        tab.history.saved_marker = None;
    }
    tab.disk_mtime = mtime;
    tab.path = Some(path);
    tab.recovered_from = None;
}

/// Picks up the saves that ended: marks their tabs saved and carries on
/// with what each was part of, or shows why it failed (which ends a close
/// it was part of). Run once per frame.
pub fn poll_saves(app: &mut ReasonCraftApp) {
    for (r, result) in app.saves.finished() {
        let Some(i) = app.tabs.iter().position(|t| t.recovery_key == r.tab) else {
            continue;
        };
        match result {
            Ok(mtime) => {
                mark_written(&mut app.tabs[i], r.path, r.revision, mtime);
                super::save::note_saved(app, i);
                after_save(app, i, r.then);
            }
            Err(err) => show_error(app, FileOp::Save, &r.path, &err),
        }
    }
}

/// Carry on with what tab `i`'s save was part of. A tab changed while it
/// was being written still has unsaved work: its close asks again.
fn after_save(app: &mut ReasonCraftApp, i: usize, then: AfterSave) {
    match then {
        AfterSave::Nothing => {}
        AfterSave::CloseTab if app.tabs[i].has_unsaved_work() => request_close_tab(app, i),
        AfterSave::CloseTab => remove_tab(app, i),
        AfterSave::ContinueWindowClose => ask_from(app, i),
    }
}

/// Waits for every running save and applies the results: before a close
/// (so a tab being saved is not asked about), and for tests and scripted
/// runs, which have no frame loop.
pub fn wait_for_saves(app: &mut ReasonCraftApp) {
    let start = Instant::now();
    while app.saves.busy() {
        poll_saves(app);
        if !app.saves.busy() || start.elapsed() > Hold::MAX_WAIT {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}
