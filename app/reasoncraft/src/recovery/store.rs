//! The recovery folder on disk.
//!
//! Each running app has a session folder `<dir>/<session>/` holding
//! `session.lock` (locked for the whole run) and, per tab with unsaved
//! work, `<tab_key>.rcraft` (the document) and `<tab_key>.json` (its
//! [`RecoveryMeta`]). A session folder whose lock can be taken belongs to
//! an app that did not exit normally: its entries are offered for restore.
//!
//! Writes and removals run in order on one worker thread, so a removal
//! never overtakes the write before it and the window never waits on the
//! disk.

use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Sender};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime};

use chrono::{DateTime, Utc};
use reasoncraft_core::{Package, Uuid, file};
use serde::{Deserialize, Serialize};

const LOCK_FILE: &str = "session.lock";

/// A copy of a tab's document, assets and unknown entries, taken on the
/// UI thread (assets share their bytes, so it is cheap).
pub type PackageSnapshot = Package;

/// What the recovery prompt shows about an autosaved document, and what a
/// save of the restored document checks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecoveryMeta {
    /// The file the tab was saved to or opened from, if any.
    pub original: Option<PathBuf>,
    /// That file's modification time as the tab knew it.
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

/// Work for the worker thread.
enum Op {
    /// Write `tab_key`'s copy into `folder` (this session's).
    Write {
        folder: PathBuf,
        tab_key: Uuid,
        pkg: Box<PackageSnapshot>,
        meta: RecoveryMeta,
        delay: Option<Duration>,
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
    write_delay: Option<Duration>,
}

impl RecoveryStore {
    /// Start a session in `dir` (created if needed): a new session folder
    /// with its lock taken.
    pub fn open(dir: PathBuf) -> io::Result<RecoveryStore> {
        let session = Uuid::new_v4();
        let folder = dir.join(session.to_string());
        fs::create_dir_all(&folder)?;
        let lock = File::create(folder.join(LOCK_FILE))?;
        lock.try_lock()?;
        let (ops, rx) = mpsc::channel::<Op>();
        let pending = Arc::new(AtomicUsize::new(0));
        let done = Arc::clone(&pending);
        let worker = std::thread::spawn(move || {
            for op in rx {
                if let Op::Flush(reply) = op {
                    let _ = reply.send(());
                    continue;
                }
                if let Err(err) = run(op) {
                    eprintln!("warning: recovery folder: {err}");
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
            write_delay: None,
        })
    }

    /// The per-user recovery folder, if the platform has a data folder.
    pub fn default_dir() -> Option<PathBuf> {
        directories::ProjectDirs::from("", "", "ReasonCraft")
            .map(|dirs| dirs.data_local_dir().join("recovery"))
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
            if !folder.is_dir() || !lock_is_free(&folder) {
                continue;
            }
            let before = found.len();
            found.extend(entries(&folder, session));
            if found.len() == before {
                remove_if_empty(&folder);
            }
        }
        found.sort_by_key(|d: &RecoveredDoc| d.meta.saved_at);
        found
    }

    /// Autosave `pkg` as tab `tab_key`'s copy, on the worker.
    pub fn write(&self, tab_key: Uuid, pkg: PackageSnapshot, meta: RecoveryMeta) {
        self.send(Op::Write {
            folder: self.session_dir(),
            tab_key,
            pkg: Box::new(pkg),
            meta,
            delay: self.write_delay,
        });
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

    /// End the session on a normal exit: finish the pending work and
    /// remove the session folder.
    pub fn shutdown(self) {
        let RecoveryStore {
            dir,
            session,
            _lock,
            ops,
            worker,
            ..
        } = self;
        drop(ops);
        let _ = worker.join();
        drop(_lock);
        let _ = fs::remove_dir_all(dir.join(session.to_string()));
    }

    /// True while writes or removals are pending.
    pub fn busy(&self) -> bool {
        self.pending.load(Ordering::SeqCst) > 0
    }

    /// Block until the work sent so far is done (tests and shutdown).
    pub fn wait_idle(&self) {
        let (tx, rx) = mpsc::channel();
        if self.ops.send(Op::Flush(tx)).is_ok() {
            let _ = rx.recv();
        }
    }

    /// Hold each later write back by `delay` on the worker (tests use it to
    /// see that autosave never waits for the disk).
    pub fn set_write_delay(&mut self, delay: Option<Duration>) {
        self.write_delay = delay;
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
            delay,
        } => {
            if let Some(delay) = delay {
                std::thread::sleep(delay);
            }
            write_entry(&folder, tab_key, &pkg, &meta)
        }
        Op::Remove {
            folder,
            tab_key,
            tidy,
        } => {
            remove_file(&folder.join(format!("{tab_key}.json")))?;
            remove_file(&folder.join(format!("{tab_key}.rcraft")))?;
            if tidy {
                remove_if_empty(&folder);
            }
            Ok(())
        }
        Op::Flush(_) => Ok(()),
    }
}

/// Write `tab_key`'s document and meta into `folder`, each to a temp file
/// renamed into place, so a crash mid-write leaves the last copy intact.
fn write_entry(folder: &Path, tab_key: Uuid, pkg: &Package, meta: &RecoveryMeta) -> io::Result<()> {
    fs::create_dir_all(folder)?;
    file::save_package(pkg, &folder.join(format!("{tab_key}.rcraft"))).map_err(io::Error::other)?;
    let json = serde_json::to_vec_pretty(meta).map_err(io::Error::other)?;
    let tmp = folder.join(format!("{tab_key}.json.tmp"));
    fs::write(&tmp, json)?;
    fs::rename(&tmp, folder.join(format!("{tab_key}.json")))
}

fn remove_file(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(err) if err.kind() != io::ErrorKind::NotFound => Err(err),
        _ => Ok(()),
    }
}

/// Whether nobody holds `folder`'s session lock (a missing lock file
/// counts as free).
fn lock_is_free(folder: &Path) -> bool {
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .open(folder.join(LOCK_FILE));
    match lock {
        Ok(file) => file.try_lock().is_ok(),
        Err(err) => err.kind() == io::ErrorKind::NotFound,
    }
}

/// The autosaved documents in `folder`: each `<tab_key>.json` whose meta
/// reads and whose `.rcraft` exists.
fn entries(folder: &Path, session: Uuid) -> Vec<RecoveredDoc> {
    let Ok(files) = fs::read_dir(folder) else {
        return Vec::new();
    };
    files
        .flatten()
        .filter_map(|file| {
            let path = file.path();
            if path.extension()? != "json" {
                return None;
            }
            let tab_key = path.file_stem()?.to_str()?.parse().ok()?;
            let meta = serde_json::from_slice(&fs::read(&path).ok()?).ok()?;
            let package = path.with_extension("rcraft");
            package.is_file().then_some(RecoveredDoc {
                session,
                tab_key,
                meta,
                package,
            })
        })
        .collect()
}

/// Remove an orphaned session folder when only its lock file and leftover
/// temp files are in it.
fn remove_if_empty(folder: &Path) {
    let Ok(files) = fs::read_dir(folder) else {
        return;
    };
    let leftovers: Vec<PathBuf> = files.flatten().map(|f| f.path()).collect();
    let only_leftovers = leftovers.iter().all(|p| {
        p.file_name().is_some_and(|n| n == LOCK_FILE) || p.extension().is_some_and(|e| e == "tmp")
    });
    if only_leftovers {
        for path in leftovers {
            let _ = fs::remove_file(path);
        }
        let _ = fs::remove_dir(folder);
    }
}
