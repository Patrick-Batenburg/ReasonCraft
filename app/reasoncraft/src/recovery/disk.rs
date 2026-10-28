//! The files of the recovery folder: making a session, writing and
//! removing copies, reading and tidying crashed sessions.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

use reasoncraft_core::{Package, Uuid, file};

use super::{RecoveredDoc, RecoveryMeta};

pub(super) const LOCK_FILE: &str = "session.lock";

/// Make session `session`'s folder in `dir` with its lock file locked. The
/// lock file is made and locked under a temporary name, then renamed into
/// place: a folder without `session.lock` is never read as a session, so a
/// new session is never seen with its lock free.
pub(super) fn create_locked_session(dir: &Path, session: Uuid) -> io::Result<File> {
    let folder = dir.join(session.to_string());
    create_private_dir(dir)?;
    create_private_dir(&folder)?;
    let staging = folder.join("session.lock.tmp");
    let made = File::create(&staging).and_then(|lock| {
        lock.try_lock()?;
        fs::rename(&staging, folder.join(LOCK_FILE))?;
        Ok(lock)
    });
    if made.is_err() {
        let _ = fs::remove_dir_all(&folder);
    }
    made
}

/// Make `dir` and any missing parents; on Unix `dir` itself is made
/// (or set) mode 0700, so other users cannot list or read the copies.
#[cfg(unix)]
fn create_private_dir(dir: &Path) -> io::Result<()> {
    use std::os::unix::fs::{DirBuilderExt as _, PermissionsExt as _};
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)?;
    fs::set_permissions(dir, fs::Permissions::from_mode(0o700))
}

#[cfg(not(unix))]
fn create_private_dir(dir: &Path) -> io::Result<()> {
    fs::create_dir_all(dir)
}

/// Write `tab_key`'s document and meta into `folder`, each to a temp file
/// renamed into place, so a crash mid-write leaves the last copy intact.
/// Only the user may read them (mode 0600 on Unix).
/// A session folder that went missing is not made again: it would have no
/// lock, and its copies would look crashed.
pub(super) fn write_entry(
    folder: &Path,
    tab_key: Uuid,
    pkg: &Package,
    meta: &RecoveryMeta,
) -> io::Result<()> {
    if !folder.join(LOCK_FILE).is_file() {
        let gone = crate::i18n::t("recovery.session_gone");
        return Err(io::Error::new(io::ErrorKind::NotFound, gone));
    }
    let copy = folder.join(format!("{tab_key}.rcraft"));
    file::save_package_private(pkg, &copy).map_err(io::Error::other)?;
    let json = serde_json::to_vec_pretty(meta).map_err(io::Error::other)?;
    let meta = folder.join(format!("{tab_key}.json"));
    file::atomic_write(&meta, true, |f| f.write_all(&json))
}

/// Remove `tab_key`'s files from `folder`; `tidy`: then remove an orphan's
/// folder if it is empty and its lock can be taken.
pub(super) fn remove_entry(folder: &Path, tab_key: Uuid, tidy: bool) -> io::Result<()> {
    remove_file(&folder.join(format!("{tab_key}.json")))?;
    remove_file(&folder.join(format!("{tab_key}.rcraft")))?;
    if tidy && let Some(lock) = take_lock(folder) {
        remove_if_empty(folder, lock);
    }
    Ok(())
}

fn remove_file(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(err) if err.kind() != io::ErrorKind::NotFound => Err(err),
        _ => Ok(()),
    }
}

pub(super) fn is_lock_file(path: &Path) -> bool {
    path.file_name().is_some_and(|n| n == LOCK_FILE)
}

/// `folder`'s session lock, when nobody holds it. A folder without a lock
/// file is not a crashed session (it is being removed): `None`.
pub(super) fn take_lock(folder: &Path) -> Option<File> {
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .open(folder.join(LOCK_FILE))
        .ok()?;
    lock.try_lock().ok()?;
    Some(lock)
}

/// The autosaved documents in `folder`: each `<tab_key>.json` whose meta
/// reads and whose `.rcraft` exists.
pub(super) fn entries(folder: &Path, session: Uuid) -> Vec<RecoveredDoc> {
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

/// Remove an orphaned session folder, whose `lock` is held, when only its
/// lock file and leftover temp files are in it. The lock file goes last.
pub(super) fn remove_if_empty(folder: &Path, lock: File) {
    let Ok(files) = fs::read_dir(folder) else {
        return;
    };
    let leftovers: Vec<PathBuf> = files.flatten().map(|f| f.path()).collect();
    let only_leftovers = leftovers
        .iter()
        .all(|p| is_lock_file(p) || p.extension().is_some_and(|e| e == "tmp"));
    if !only_leftovers {
        return;
    }
    for path in leftovers.iter().filter(|p| !is_lock_file(p)) {
        let _ = fs::remove_file(path);
    }
    drop(lock);
    let _ = fs::remove_file(folder.join(LOCK_FILE));
    let _ = fs::remove_dir(folder);
}
