//! The channel on Linux and macOS: a Unix socket in a folder only this
//! user can enter (mode 0700, owned by this user, checked before use):
//! under `$XDG_RUNTIME_DIR` (in the Flatpak sandbox the app's own part of
//! it, `$XDG_RUNTIME_DIR/app/<id>`, which every copy of the app shares),
//! else in the temp folder (per user on macOS). Which copy serves is
//! settled by a lock on a file beside the socket, which the system drops
//! when its process ends: a socket file a crashed copy left behind is
//! replaced by the next copy that gets the lock.

use std::fs::{self, DirBuilder, File, TryLockError};
use std::io;
use std::os::unix::fs::{DirBuilderExt as _, MetadataExt as _, OpenOptionsExt as _};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use super::Elected;

/// A connection: either end of the socket.
pub(super) type Conn = UnixStream;

/// The first copy's socket, and the lock that makes it the first.
pub(super) struct Server {
    listener: UnixListener,
    _lock: File,
}

/// Become the copy that serves `scope`, unless another one does.
pub(super) fn elect(scope: &str) -> io::Result<Elected<Server>> {
    let dir = private_dir()?;
    let lock = File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .mode(0o600)
        .open(dir.join(format!("{scope}.lock")))?;
    match lock.try_lock() {
        Ok(()) => {}
        Err(TryLockError::WouldBlock) => return Ok(Elected::Taken),
        Err(TryLockError::Error(e)) => return Err(e),
    }
    // Holding the lock, any socket file there is a crashed copy's.
    let socket = dir.join(format!("{scope}.sock"));
    match fs::remove_file(&socket) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
        _ => {}
    }
    let listener = UnixListener::bind(&socket)?;
    Ok(Elected::Serve(Server {
        listener,
        _lock: lock,
    }))
}

impl Server {
    /// Wait for the next launch to connect.
    pub(super) fn accept(&mut self) -> io::Result<Conn> {
        self.listener.accept().map(|(conn, _)| conn)
    }
}

/// Connect to the copy serving `scope`. A socket not there yet (its copy
/// is starting) or any more is `NotFound` or `ConnectionRefused`.
pub(super) fn connect(scope: &str, _deadline: Instant) -> io::Result<Conn> {
    UnixStream::connect(private_dir()?.join(format!("{scope}.sock")))
}

/// Reads and writes on `conn` give up after `timeout`.
pub(super) fn set_timeouts(conn: &Conn, timeout: Duration) {
    let timeout = Some(timeout.max(Duration::from_millis(1)));
    let _ = conn.set_read_timeout(timeout);
    let _ = conn.set_write_timeout(timeout);
}

/// Whether `e` means that no copy serves the scope (yet, or any more):
/// try to become it, or wait for it.
pub(super) fn not_there(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
    )
}

/// Nothing to do: the answer is read before the socket closes.
pub(super) fn finish(_: &Conn) {}

/// The folder for the lock and socket files, made (mode 0700) when
/// missing; refused unless it is a real folder of this user's that no one
/// else may enter.
fn private_dir() -> io::Result<PathBuf> {
    // SAFETY: getuid has no preconditions and cannot fail.
    let uid = unsafe { libc::getuid() };
    let app = crate::settings_store::APP_ID;
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute());
    let dir = match (runtime, std::env::var_os("FLATPAK_ID")) {
        (Some(runtime), Some(id)) => runtime.join("app").join(id).join("instance"),
        (Some(runtime), None) => runtime.join(app),
        (None, _) => std::env::temp_dir().join(format!("{app}-{uid}")),
    };
    match DirBuilder::new().mode(0o700).create(&dir) {
        Err(e) if e.kind() != io::ErrorKind::AlreadyExists => return Err(e),
        _ => {}
    }
    let meta = fs::symlink_metadata(&dir)?;
    if !meta.is_dir() || meta.uid() != uid || meta.mode() & 0o077 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("{} is not a private folder", dir.display()),
        ));
    }
    Ok(dir)
}

/// Remove `scope`'s files (tests).
#[cfg(test)]
pub(super) fn forget(scope: &str) {
    if let Ok(dir) = private_dir() {
        let _ = fs::remove_file(dir.join(format!("{scope}.sock")));
        let _ = fs::remove_file(dir.join(format!("{scope}.lock")));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_folder_is_private() {
        let dir = private_dir().unwrap();
        let meta = fs::symlink_metadata(&dir).unwrap();
        assert_eq!(meta.mode() & 0o777, 0o700, "{}", dir.display());
    }

    /// A socket file left by a crashed copy (no lock held) is replaced.
    #[test]
    fn a_stale_socket_is_replaced() {
        let scope = super::super::tests::unique_scope();
        let socket = private_dir().unwrap().join(format!("{scope}.sock"));
        drop(UnixListener::bind(&socket).unwrap());
        assert!(socket.exists(), "the stale socket file stays");
        let connect = UnixStream::connect(&socket).unwrap_err();
        assert!(not_there(&connect), "{connect:?}");
        assert!(matches!(elect(&scope), Ok(Elected::Serve(_))));
        forget(&scope);
    }
}
