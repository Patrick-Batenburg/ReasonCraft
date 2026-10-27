//! The recovery folder on disk.
//!
//! Each running app has a session folder `<dir>/<session>/` holding
//! `session.lock` (locked for the whole run) and, per tab with unsaved
//! work, `<tab_key>.rcraft` (the document) and `<tab_key>.json` (its
//! [`RecoveryMeta`]). A session folder whose lock can be taken belongs to
//! an app that did not exit normally: its entries are offered for restore.
//!
//! A session's lock file is renamed into place only once it is locked, and
//! a folder without a lock file is never treated as crashed, so another app
//! never sees a starting session as crashed. A write never makes a missing
//! session folder again.
//!
//! Writes and removals run in order on one worker thread, so a removal
//! never overtakes the write before it and the window never waits on the
//! disk. Failed writes are reported back ([`RecoveryStore::failed_writes`]).

use std::collections::HashMap;
use std::fs::{self, File};
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime};

use chrono::{DateTime, Utc};
use reasoncraft_core::{Package, Uuid};
use serde::{Deserialize, Serialize};

use super::disk::{
    LOCK_FILE, create_locked_session, entries, is_lock_file, remove_entry, remove_if_empty,
    take_lock, write_entry,
};

/// A copy of a tab's document, assets and unknown entries, taken on the
/// UI thread (assets share their bytes, so it is cheap).
pub type PackageSnapshot = Package;

/// What the recovery prompt shows about an autosaved document, and what a
/// save of the restored document checks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecoveryMeta {
    /// The file the tab was saved to or opened from, if any.
    pub original: Option<PathBuf>,
    /// That file's modification time when the tab last opened or saved it.
    pub original_mtime: Option<SystemTime>,
    /// The tab's name.
    pub title: String,
    pub saved_at: DateTime<Utc>,
}

/// An autosaved document left behind by an app that did not exit normally.
#[derive(Clone, Debug, PartialEq)]
pub struct RecoveredDoc {
    pub session: Uuid,
    pub tab_key: Uuid,
    pub meta: RecoveryMeta,
    /// The autosaved `.rcraft` file.
    pub package: PathBuf,
}

/// A write that failed: the tab it was for, and why.
#[derive(Clone, Debug, PartialEq)]
pub struct WriteFailure {
    pub tab_key: Uuid,
    pub reason: String,
}

/// Test hooks for writes: hold them back, or make them fail.
#[derive(Clone, Copy, Debug, Default)]
struct WriteHooks {
    delay: Option<Duration>,
    fail: bool,
}

/// Work for the worker thread.
enum Op {
    /// Write `tab_key`'s copy into `folder` (this session's); then, only if
    /// that succeeded, delete the orphaned copy `then_discard` (a restore).
    /// When that fails, the tab's next write deletes it.
    Write {
        folder: PathBuf,
        tab_key: Uuid,
        pkg: Box<PackageSnapshot>,
        meta: RecoveryMeta,
        then_discard: Option<(PathBuf, Uuid)>,
        hooks: WriteHooks,
    },
    /// Remove `tab_key`'s files from `folder`; `tidy`: then remove the
    /// folder too if nothing else is left in it (an orphan's folder).
    Remove {
        folder: PathBuf,
        tab_key: Uuid,
        tidy: bool,
    },
    /// Answer once everything sent before is done.
    Flush(Sender<()>),
}

/// This app's session in the recovery folder.
pub struct RecoveryStore {
    dir: PathBuf,
    session: Uuid,
    _lock: File,
    ops: Sender<Op>,
    worker: JoinHandle<()>,
    /// Writes and removals sent and not yet done.
    pending: Arc<AtomicUsize>,
    failures: Receiver<WriteFailure>,
    hooks: WriteHooks,
}

impl RecoveryStore {
    /// Start a session in `dir` (created if needed): a new session folder
    /// with its lock taken.
    pub fn open(dir: PathBuf) -> io::Result<RecoveryStore> {
        let session = Uuid::new_v4();
        let lock = create_locked_session(&dir, session)?;
        let (ops, rx) = mpsc::channel::<Op>();
        let (failed, failures) = mpsc::channel();
        let pending = Arc::new(AtomicUsize::new(0));
        let done = Arc::clone(&pending);
        let worker = std::thread::spawn(move || {
            // Per tab, the orphaned copy a failed write was to delete: the
            // tab's next write deletes it instead.
            let mut kept_discards = HashMap::new();
            for mut op in rx {
                if let Op::Flush(reply) = op {
                    let _ = reply.send(());
                    continue;
                }
                let mut write = None;
                if let Op::Write {
                    tab_key,
                    then_discard,
                    ..
                } = &mut op
                {
                    if then_discard.is_none() {
                        *then_discard = kept_discards.remove(tab_key);
                    }
                    write = Some((*tab_key, then_discard.clone()));
                }
                match (run(op), write) {
                    (Ok(()), _) => {}
                    (Err(err), Some((tab_key, discard))) => {
                        if let Some(discard) = discard {
                            kept_discards.insert(tab_key, discard);
                        }
                        let reason = err.to_string();
                        let _ = failed.send(WriteFailure { tab_key, reason });
                    }
                    (Err(err), None) => eprintln!("warning: recovery folder: {err}"),
                }
                done.fetch_sub(1, Ordering::SeqCst);
            }
        });
        Ok(RecoveryStore {
            dir,
            session,
            _lock: lock,
            ops,
            worker,
            pending,
            failures,
            hooks: WriteHooks::default(),
        })
    }

    /// The per-user recovery folder, if the platform has a data folder.
    pub fn default_dir() -> Option<PathBuf> {
        super::folder::default_dir()
    }

    /// This session's folder.
    pub fn session_dir(&self) -> PathBuf {
        self.dir.join(self.session.to_string())
    }

    /// The autosaved documents of sessions that ended without a normal exit
    /// (their lock is free), oldest first. Empty folders of such sessions
    /// are removed.
    pub fn orphaned(&self) -> Vec<RecoveredDoc> {
        let Ok(sessions) = fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        let mut found = Vec::new();
        for entry in sessions.flatten() {
            let folder = entry.path();
            let session = entry.file_name().to_str().and_then(|n| n.parse().ok());
            let Some(session) = session.filter(|s| *s != self.session) else {
                continue;
            };
            // Held while the folder is read, and while an empty one goes.
            let Some(lock) = take_lock(&folder) else {
                continue;
            };
            let before = found.len();
            found.extend(entries(&folder, session));
            if found.len() == before {
                remove_if_empty(&folder, lock);
            }
        }
        found.sort_by_key(|d: &RecoveredDoc| d.meta.saved_at);
        found
    }

    /// Autosave `pkg` as tab `tab_key`'s copy, on the worker.
    pub fn write(&self, tab_key: Uuid, pkg: PackageSnapshot, meta: RecoveryMeta) {
        self.write_then(tab_key, pkg, meta, None);
    }

    /// Like [`Self::write`], then, only if the write succeeded, delete the
    /// orphaned copy `doc` it replaces (as [`Self::discard`] does).
    pub fn write_then_discard(
        &self,
        tab_key: Uuid,
        pkg: PackageSnapshot,
        meta: RecoveryMeta,
        doc: &RecoveredDoc,
    ) {
        let orphan = (self.dir.join(doc.session.to_string()), doc.tab_key);
        self.write_then(tab_key, pkg, meta, Some(orphan));
    }

    fn write_then(
        &self,
        tab_key: Uuid,
        pkg: PackageSnapshot,
        meta: RecoveryMeta,
        then_discard: Option<(PathBuf, Uuid)>,
    ) {
        self.send(Op::Write {
            folder: self.session_dir(),
            tab_key,
            pkg: Box::new(pkg),
            meta,
            then_discard,
            hooks: self.hooks,
        });
    }

    /// The writes that failed since the last call. Never blocks.
    pub fn failed_writes(&self) -> Vec<WriteFailure> {
        self.failures.try_iter().collect()
    }

    /// Remove tab `tab_key`'s copy (it was saved, discarded or closed).
    pub fn remove(&self, tab_key: Uuid) {
        self.send(Op::Remove {
            folder: self.session_dir(),
            tab_key,
            tidy: false,
        });
    }

    /// Remove an orphaned copy, and its session folder once it is empty.
    pub fn discard(&self, doc: &RecoveredDoc) {
        self.send(Op::Remove {
            folder: self.dir.join(doc.session.to_string()),
            tab_key: doc.tab_key,
            tidy: true,
        });
    }

    /// End the session on a normal exit: finish the pending work, delete
    /// the copies while the lock is still held, then release the lock and,
    /// when every copy went, remove the lock file and the folder.
    pub fn shutdown(self) {
        let folder = self.session_dir();
        let RecoveryStore {
            _lock, ops, worker, ..
        } = self;
        drop(ops);
        let _ = worker.join();
        // Every file is tried, even after one fails.
        let all_deleted = fs::read_dir(&folder).is_ok_and(|files| {
            (files.flatten().map(|f| f.path()))
                .filter(|path| !is_lock_file(path))
                .filter(|path| fs::remove_file(path).is_err())
                .count()
                == 0
        });
        drop(_lock);
        // A file left behind keeps the lock file: the next start then sees
        // a crashed session and tidies the folder.
        if all_deleted {
            let _ = fs::remove_file(folder.join(LOCK_FILE));
            let _ = fs::remove_dir(&folder);
        }
    }

    /// True while writes or removals are pending.
    pub fn busy(&self) -> bool {
        self.pending.load(Ordering::SeqCst) > 0
    }

    /// Block until the work sent so far is done (tests use it).
    pub fn wait_idle(&self) {
        let (tx, rx) = mpsc::channel();
        if self.ops.send(Op::Flush(tx)).is_ok() {
            let _ = rx.recv();
        }
    }

    /// Hold each later write back by `delay` on the worker (tests use it to
    /// see that autosave never waits for the disk).
    pub fn set_write_delay(&mut self, delay: Option<Duration>) {
        self.hooks.delay = delay;
    }

    /// Make each later write fail (tests use it to see failures handled).
    pub fn set_fail_writes(&mut self, fail: bool) {
        self.hooks.fail = fail;
    }

    fn send(&self, op: Op) {
        self.pending.fetch_add(1, Ordering::SeqCst);
        if self.ops.send(op).is_err() {
            self.pending.fetch_sub(1, Ordering::SeqCst);
        }
    }
}

/// Carry out a write or a removal.
fn run(op: Op) -> io::Result<()> {
    match op {
        Op::Write {
            folder,
            tab_key,
            pkg,
            meta,
            then_discard,
            hooks,
        } => {
            if let Some(delay) = hooks.delay {
                std::thread::sleep(delay);
            }
            if hooks.fail {
                return Err(io::Error::other("write failure (test)"));
            }
            write_entry(&folder, tab_key, &pkg, &meta)?;
            match then_discard {
                Some((orphan, key)) => remove_entry(&orphan, key, true),
                None => Ok(()),
            }
        }
        Op::Remove {
            folder,
            tab_key,
            tidy,
        } => remove_entry(&folder, tab_key, tidy),
        Op::Flush(_) => Ok(()),
    }
}
