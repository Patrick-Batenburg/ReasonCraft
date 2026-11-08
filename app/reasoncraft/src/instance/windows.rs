//! The channel on Windows: a named pipe, local to this computer, whose
//! name holds the user's SID, the login session and the scope, so another
//! user or session never meets it. Only this user may connect to it or
//! add an instance of it (its DACL); the first copy creates it with
//! `FILE_FLAG_FIRST_PIPE_INSTANCE`, so a second copy cannot serve beside
//! it, and the pipe goes when its process does (no stale state after a
//! crash). A launch connects only to a pipe served by a process of its
//! own user (one squatting the name is refused), and lets that process
//! take the foreground.

use std::ffi::OsStr;
use std::fs::File;
use std::io;
use std::os::windows::ffi::OsStrExt as _;
use std::os::windows::fs::OpenOptionsExt as _;
use std::os::windows::io::{AsRawHandle as _, FromRawHandle as _, OwnedHandle};
use std::ptr::null_mut;
use std::time::Instant;

use windows_sys::Win32::Foundation::{
    ERROR_ACCESS_DENIED, ERROR_PIPE_BUSY, ERROR_PIPE_CONNECTED, HANDLE, INVALID_HANDLE_VALUE,
    LocalFree,
};
use windows_sys::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows_sys::Win32::Security::{
    GetTokenInformation, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER,
    TokenUser,
};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_ACCESS_DUPLEX, SECURITY_IDENTIFICATION,
};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, GetNamedPipeServerProcessId, PIPE_READMODE_BYTE,
    PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
    WaitNamedPipeW,
};
use windows_sys::Win32::System::RemoteDesktop::ProcessIdToSessionId;
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::WindowsAndMessaging::AllowSetForegroundWindow;

use super::Elected;

/// A connection: the client's or the server's end of the pipe.
pub(super) type Conn = File;

/// The first copy's pipe, with one instance always waiting for the next
/// launch (so the name never lapses between two of them).
pub(super) struct Server {
    name: Vec<u16>,
    access: Access,
    waiting: OwnedHandle,
}

/// Become the copy that serves `scope`, unless another one does.
pub(super) fn elect(scope: &str) -> io::Result<Elected<Server>> {
    let user = user_sid(current_process())?;
    let name = wide(&pipe_name(&user, scope)?);
    let access = Access::only(&user)?;
    match create_instance(&name, &access, true) {
        Ok(waiting) => Ok(Elected::Serve(Server {
            name,
            access,
            waiting,
        })),
        Err(e) if e.raw_os_error() == Some(ERROR_ACCESS_DENIED as i32) => Ok(Elected::Taken),
        Err(e) => Err(e),
    }
}

impl Server {
    /// Wait for the next launch to connect.
    pub(super) fn accept(&mut self) -> io::Result<Conn> {
        // SAFETY: a valid pipe handle; no overlapped I/O.
        let connected = match unsafe { ConnectNamedPipe(self.waiting.as_raw_handle(), null_mut()) }
        {
            0 => match io::Error::last_os_error() {
                // The launch connected before the wait began.
                e if e.raw_os_error() == Some(ERROR_PIPE_CONNECTED as i32) => Ok(()),
                e => Err(e),
            },
            _ => Ok(()),
        };
        // The next instance first, so the name stays taken.
        let next = create_instance(&self.name, &self.access, false)?;
        let current = std::mem::replace(&mut self.waiting, next);
        connected.map(|()| File::from(current))
    }
}

/// Connect to the copy serving `scope`, waiting while it is busy with
/// another launch (until `deadline`). A pipe not served by this user is
/// refused (`PermissionDenied`); a missing one is `NotFound`.
pub(super) fn connect(scope: &str, deadline: Instant) -> io::Result<Conn> {
    let user = user_sid(current_process())?;
    let name = pipe_name(&user, scope)?;
    loop {
        let opened = File::options()
            .read(true)
            .write(true)
            // The server may not act as this user.
            .security_qos_flags(SECURITY_IDENTIFICATION)
            .open(&name);
        match opened {
            Ok(pipe) => {
                let server = server_process(&pipe)?;
                if user_sid(server.as_raw_handle())? != user {
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "the pipe is served by another user",
                    ));
                }
                // The launch is in the foreground; the window it hands
                // over to may come to the front. Best effort.
                // SAFETY: takes a process id only.
                unsafe { AllowSetForegroundWindow(server_pid(&pipe)?) };
                return Ok(pipe);
            }
            Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY as i32) => {
                let left = deadline.saturating_duration_since(Instant::now());
                if left.is_zero() {
                    return Err(io::ErrorKind::TimedOut.into());
                }
                let ms = u32::try_from(left.as_millis()).unwrap_or(u32::MAX).max(1);
                // SAFETY: a NUL-terminated name that outlives the call.
                unsafe { WaitNamedPipeW(wide(&name).as_ptr(), ms) };
            }
            Err(e) => return Err(e),
        }
    }
}

/// Nothing to set: reads and writes are bounded by the waiting thread.
pub(super) fn set_timeouts(_: &Conn, _: std::time::Duration) {}

/// Whether `e` means that no copy serves the scope (any more): try to
/// become it.
pub(super) fn not_there(e: &io::Error) -> bool {
    e.kind() == io::ErrorKind::NotFound
}

/// Make sure the other end read everything before the pipe closes.
pub(super) fn finish(conn: &Conn) {
    let _ = conn.sync_all();
}

/// `\\.\pipe\<app>-<user SID>-<session>-<scope>`.
fn pipe_name(user: &str, scope: &str) -> io::Result<String> {
    let mut session = 0;
    // SAFETY: writes one u32 through a valid pointer.
    if unsafe { ProcessIdToSessionId(std::process::id(), &raw mut session) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let app = crate::settings_store::APP_ID;
    Ok(format!(r"\\.\pipe\{app}-{user}-{session}-{scope}"))
}

fn wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain([0]).collect()
}

fn current_process() -> HANDLE {
    // SAFETY: a pseudo handle; nothing to close.
    unsafe { GetCurrentProcess() }
}

/// The process serving `pipe`.
fn server_process(pipe: &File) -> io::Result<OwnedHandle> {
    let pid = server_pid(pipe)?;
    // SAFETY: takes plain values; the handle returned is checked.
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    owned(process)
}

fn server_pid(pipe: &File) -> io::Result<u32> {
    let mut pid = 0;
    // SAFETY: a valid pipe handle; writes one u32.
    if unsafe { GetNamedPipeServerProcessId(pipe.as_raw_handle(), &raw mut pid) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(pid)
}

fn owned(handle: HANDLE) -> io::Result<OwnedHandle> {
    if handle.is_null() || handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: a valid handle nothing else owns.
    Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
}

/// The SID of the user `process` runs as, as text (`S-1-5-21-…`).
fn user_sid(process: HANDLE) -> io::Result<String> {
    let mut token = null_mut();
    // SAFETY: a valid process handle; writes one handle.
    if unsafe { OpenProcessToken(process, TOKEN_QUERY, &raw mut token) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let token = owned(token)?;
    let mut len = 0u32;
    // SAFETY: asks for the size only (no buffer).
    unsafe {
        GetTokenInformation(
            token.as_raw_handle(),
            TokenUser,
            null_mut(),
            0,
            &raw mut len,
        )
    };
    // u64s keep the TOKEN_USER at the start aligned.
    let mut buf = vec![0u64; (len as usize).div_ceil(8).max(1)];
    // SAFETY: the buffer holds `len` bytes.
    let ok = unsafe {
        GetTokenInformation(
            token.as_raw_handle(),
            TokenUser,
            buf.as_mut_ptr().cast(),
            len,
            &raw mut len,
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the call filled the buffer with a TOKEN_USER (aligned, see above).
    let sid = unsafe { (*buf.as_ptr().cast::<TOKEN_USER>()).User.Sid };
    let mut text = null_mut();
    // SAFETY: a valid SID (inside `buf`); writes one pointer, freed below.
    if unsafe { ConvertSidToStringSidW(sid, &raw mut text) } == 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: a NUL-terminated string from the call above.
    let len = (0..).take_while(|&i| unsafe { *text.add(i) } != 0).count();
    // SAFETY: `len` units before the NUL.
    let sid = String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(text, len) });
    // SAFETY: allocated by ConvertSidToStringSidW with LocalAlloc.
    unsafe { LocalFree(text.cast()) };
    Ok(sid)
}

/// A security descriptor that lets only one user at the pipe.
struct Access(PSECURITY_DESCRIPTOR);

// SAFETY: the descriptor is plain memory, only read after it is made.
unsafe impl Send for Access {}

impl Access {
    fn only(user: &str) -> io::Result<Self> {
        // Protected DACL: full access for `user`, nobody else.
        let sddl = wide(&format!("D:P(A;;GA;;;{user})"));
        let mut sd = null_mut();
        // SAFETY: a NUL-terminated string; writes one pointer, freed in Drop.
        let ok = unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                SDDL_REVISION_1,
                &raw mut sd,
                null_mut(),
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(sd))
    }
}

impl Drop for Access {
    fn drop(&mut self) {
        // SAFETY: allocated with LocalAlloc by the conversion above.
        unsafe { LocalFree(self.0) };
    }
}

/// A new instance of the pipe `name`, for this computer only; `first`:
/// fail when the pipe exists already.
fn create_instance(name: &[u16], access: &Access, first: bool) -> io::Result<OwnedHandle> {
    let attributes = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: access.0,
        bInheritHandle: 0,
    };
    let open_mode = PIPE_ACCESS_DUPLEX
        | if first {
            FILE_FLAG_FIRST_PIPE_INSTANCE
        } else {
            0
        };
    let pipe_mode = PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS;
    // SAFETY: a NUL-terminated name and attributes that outlive the call.
    let pipe = unsafe {
        CreateNamedPipeW(
            name.as_ptr(),
            open_mode,
            pipe_mode,
            PIPE_UNLIMITED_INSTANCES,
            4096,
            4096,
            0,
            &raw const attributes,
        )
    };
    owned(pipe)
}

#[cfg(test)]
pub(super) fn forget(_scope: &str) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pipe_name_holds_the_user_and_session() {
        let user = user_sid(current_process()).unwrap();
        assert!(user.starts_with("S-1-"), "{user}");
        let name = pipe_name(&user, "scope").unwrap();
        assert!(name.starts_with(r"\\.\pipe\io.github.Patrick_Batenburg.ReasonCraft-S-1-"));
        assert!(name.ends_with("-scope"), "{name}");
    }
}
