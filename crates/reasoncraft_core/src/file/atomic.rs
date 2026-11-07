//! Replacing a file whole or not at all: write a new temporary file next
//! to it, flush it to disk, rename it over the target. Used for documents,
//! recovery copies and exports.

use std::fs::{File, OpenOptions};
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

/// How many symbolic links in a row a save follows (Linux's limit).
const MAX_LINKS: usize = 40;

/// How many names in a row may be refused with `PermissionDenied` before
/// [`create_first_new_with`] gives up (a folder that refuses every name).
const MAX_DENIED_NAMES: usize = 16;

/// Makes a new, empty file at the first of `candidates` that does not
/// exist yet. `create_new` means an existing file (or a link) with that
/// name is never opened, let alone overwritten. `private`: on Unix, only
/// the user may read the file (mode 0600).
pub fn create_first_new(
    candidates: impl IntoIterator<Item = PathBuf>,
    private: bool,
) -> io::Result<(PathBuf, File)> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    if private {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    #[cfg(not(unix))]
    let _ = private;
    create_first_new_with(candidates, |path| options.open(path))
}

/// [`create_first_new`] making each file with `open`. A name that exists
/// is skipped, and so is one refused with `PermissionDenied`: Windows
/// refuses a name whose file is still being deleted. After
/// [`MAX_DENIED_NAMES`] refusals in a row that error is returned.
pub fn create_first_new_with(
    candidates: impl IntoIterator<Item = PathBuf>,
    mut open: impl FnMut(&Path) -> io::Result<File>,
) -> io::Result<(PathBuf, File)> {
    let mut denied = 0;
    for path in candidates {
        match open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(e) if e.kind() == ErrorKind::AlreadyExists => denied = 0,
            Err(e) if e.kind() == ErrorKind::PermissionDenied && denied + 1 < MAX_DENIED_NAMES => {
                denied += 1;
            }
            Err(e) => return Err(e),
        }
    }
    Err(io::Error::other("no free file name"))
}

/// Makes a new, empty temporary file next to `path`, named
/// `<name>.<pid>-<counter>.tmp`; a file that already has that name (the
/// user's, or another writer's) is never touched.
fn create_temp(path: &Path, private: bool) -> io::Result<(PathBuf, File)> {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let pid = std::process::id();
    let candidates = std::iter::repeat_with(|| {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut name = path.as_os_str().to_owned();
        name.push(format!(".{pid}-{n}.tmp"));
        PathBuf::from(name)
    });
    create_first_new(candidates, private)
}

/// Replace `path` with what `write` writes, whole or not at all: `write`
/// fills a new temporary file next to `path`, which is flushed to disk and
/// renamed over `path`. On failure the temporary file is removed and
/// `path` is left as it was.
///
/// On Unix a `private` file is made with mode 0600 (recovery copies);
/// otherwise a replaced file keeps its permissions, and a read-only one
/// is not replaced (`PermissionDenied`). When `path` is a
/// symbolic link, the file it points to is replaced and the link stays.
pub fn atomic_write<E: From<io::Error>>(
    path: &Path,
    private: bool,
    write: impl FnOnce(&mut File) -> Result<(), E>,
) -> Result<(), E> {
    let path = &link_target(path);
    // A rename would replace a read-only file on Unix (and keep it
    // read-only); Windows refuses it. Refused alike everywhere.
    if !private && std::fs::metadata(path).is_ok_and(|m| m.permissions().readonly()) {
        let read_only = io::Error::new(io::ErrorKind::PermissionDenied, "the file is read-only");
        return Err(read_only.into());
    }
    let (tmp, mut file) = create_temp(path, private)?;
    let result = write(&mut file).and_then(|()| {
        file.sync_all()?;
        drop(file);
        if !private {
            keep_permissions(path, &tmp)?;
        }
        std::fs::rename(&tmp, path)?;
        sync_folder(path);
        Ok(())
    });
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

/// The file `path` names, following symbolic links (at most
/// [`MAX_LINKS`]); a link's relative target is read from the link's
/// folder. `path` itself when it is no link.
fn link_target(path: &Path) -> PathBuf {
    let mut path = path.to_path_buf();
    for _ in 0..MAX_LINKS {
        let Ok(target) = std::fs::read_link(&path) else {
            break;
        };
        path = match path.parent() {
            Some(dir) => dir.join(target),
            None => target,
        };
    }
    path
}

/// Give `tmp` the permissions of the file it replaces, if there is one.
/// Only on Unix: on Windows the only permission is read-only, and a
/// read-only file cannot be replaced anyway.
fn keep_permissions(path: &Path, tmp: &Path) -> io::Result<()> {
    #[cfg(unix)]
    if let Ok(meta) = std::fs::metadata(path) {
        std::fs::set_permissions(tmp, meta.permissions())?;
    }
    #[cfg(not(unix))]
    let _ = (path, tmp);
    Ok(())
}

/// Flush the rename itself to disk (Unix; best effort).
fn sync_folder(path: &Path) {
    #[cfg(unix)]
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty())
        && let Ok(dir) = File::open(dir)
    {
        let _ = dir.sync_all();
    }
    #[cfg(not(unix))]
    let _ = path;
}
