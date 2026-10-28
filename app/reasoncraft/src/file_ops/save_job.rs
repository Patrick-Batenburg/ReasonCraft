//! Saves run on a worker thread, like autosave, so a slow disk (a network
//! or synced folder) never freezes the window. They run one after another,
//! so two saves of one file never interleave. The frame loop picks up each
//! result ([`poll_saves`]): the tab takes the path and counts as saved,
//! unless it changed while the file was written, and what the save was
//! part of (a tab or window close) carries on once no dialog is open.
//! Tabs are named by their [`DocTab::recovery_key`] throughout: a close
//! that a save carries on may remove a tab before the others.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use reasoncraft_core::file::{self, FileError};
use reasoncraft_core::{Package, Uuid};

use super::{AfterSave, FileOp, ask_from, ask_or_close, show_error, tab_index};
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

/// What a finished save leaves to do once no dialog is open: carry on
/// with what it was part of, or say why it failed.
enum Next {
    Then(Uuid, AfterSave),
    Failed(PathBuf, FileError),
}

/// How long a close waits on the UI thread for the saves being written
/// (they are usually done). A save still running after that goes on in
/// the background, and its tab is asked about as unsaved meanwhile.
pub const SAVE_CLOSE_WAIT: Duration = Duration::from_secs(30);

/// The saves being written, for all tabs.
#[derive(Default)]
pub struct SaveJobs {
    running: Vec<Running>,
    queue: Queue,
    /// Finished saves whose follow-up waits for the open dialog to close.
    next: VecDeque<Next>,
    /// Hold each save back until released, before it writes (tests only).
    pub hold: Option<Hold>,
}

impl SaveJobs {
    /// Whether a save is still being written.
    pub fn busy(&self) -> bool {
        !self.running.is_empty()
    }

    /// Where the last save of `tab` that is still being written goes: a
    /// Save meanwhile writes there too, not to the tab's old file.
    pub fn pending_path(&self, tab: &DocTab) -> Option<&Path> {
        (self.running.iter().rev())
            .find(|r| r.tab == tab.recovery_key)
            .map(|r| r.path.as_path())
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
        parked_on_open: false,
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

/// Picks up the saves that ended: marks their tabs saved, then carries on
/// with what each was part of, or shows why it failed (which ends a close
/// it was part of). While a dialog is open (a message, the export dialog
/// or the link editor), that waits: the dialog acts on a tab, and a new
/// message must not replace it. Run once per frame.
pub fn poll_saves(app: &mut ReasonCraftApp) {
    for (r, result) in app.saves.finished() {
        let Some(i) = tab_index(app, r.tab) else {
            continue;
        };
        match result {
            Ok(mtime) => {
                mark_written(&mut app.tabs[i], r.path, r.revision, mtime);
                super::save::note_saved(app, i);
                if r.then != AfterSave::Nothing {
                    app.saves.next.push_back(Next::Then(r.tab, r.then));
                }
            }
            Err(err) => app.saves.next.push_back(Next::Failed(r.path, err)),
        }
    }
    while !app.dialog_open()
        && let Some(next) = app.saves.next.pop_front()
    {
        match next {
            Next::Then(tab, then) => after_save(app, tab, then),
            Next::Failed(path, err) => show_error(app, FileOp::Save, &path, &err),
        }
    }
}

/// Carry on with `then` for tab `tab` once no dialog is open, after what
/// the saves that ended left to do: a close asked for while a save's
/// error shows waits behind it.
pub(super) fn after_dialogs(app: &mut ReasonCraftApp, tab: Uuid, then: AfterSave) {
    app.saves.next.push_back(Next::Then(tab, then));
    poll_saves(app);
}

/// Carry on with what tab `tab`'s save was part of. A tab changed while
/// it was being written still has unsaved work: its close asks again.
fn after_save(app: &mut ReasonCraftApp, tab: Uuid, then: AfterSave) {
    let Some(i) = tab_index(app, tab) else {
        return;
    };
    match then {
        AfterSave::Nothing => {}
        AfterSave::CloseTab => ask_or_close(app, i),
        AfterSave::ContinueWindowClose => ask_from(app, i),
    }
}

/// Waits for every running save (at most [`SAVE_CLOSE_WAIT`]) and applies
/// the results: before a close (so a tab being saved is not asked about),
/// and for tests and scripted runs, which have no frame loop.
pub fn wait_for_saves(app: &mut ReasonCraftApp) {
    let start = Instant::now();
    while app.saves.busy() {
        poll_saves(app);
        if !app.saves.busy() || start.elapsed() > SAVE_CLOSE_WAIT {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}
