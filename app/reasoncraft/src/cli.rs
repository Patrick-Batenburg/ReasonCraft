//! The command line: `reasoncraft [FILE]...` opens the files, and
//! `reasoncraft --version` prints the version and exits.

use std::ffi::OsString;

/// Whether the arguments (without the program name) ask only for the
/// version: `--version` and nothing else. Any other argument is a path.
pub fn wants_version(args: &[OsString]) -> bool {
    matches!(args, [only] if only == "--version")
}

/// Print the version line (`--version`). A release build on Windows is a
/// window app with no console of its own: it borrows the console it was
/// started from, if any, when its output goes nowhere else (a pipe or a
/// file keeps it). Nowhere to print (a closed pipe) is not an error.
pub fn print_version() {
    use std::io::Write as _;
    #[cfg(windows)]
    attach_parent_console();
    let _ = writeln!(
        std::io::stdout(),
        "ReasonCraft {}",
        env!("CARGO_PKG_VERSION")
    );
}

/// Writes to stdout reach the console of the process that started this
/// one (a command prompt), when stdout is not set (a window app's isn't,
/// unless redirected). The prompt does not wait for a window app, so the
/// line may show after it.
#[cfg(windows)]
fn attach_parent_console() {
    use std::os::windows::io::IntoRawHandle as _;
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows_sys::Win32::System::Console::{
        ATTACH_PARENT_PROCESS, AttachConsole, GetStdHandle, STD_OUTPUT_HANDLE, SetStdHandle,
    };
    let unset =
        |h: windows_sys::Win32::Foundation::HANDLE| h.is_null() || h == INVALID_HANDLE_VALUE;
    // SAFETY: these take and return plain values; no pointer is passed.
    if !unset(unsafe { GetStdHandle(STD_OUTPUT_HANDLE) })
        || unsafe { AttachConsole(ATTACH_PARENT_PROCESS) } == 0
    {
        return;
    }
    // SAFETY: as above.
    if unset(unsafe { GetStdHandle(STD_OUTPUT_HANDLE) })
        && let Ok(console) = std::fs::OpenOptions::new().write(true).open("CONOUT$")
    {
        // The handle stays open for the rest of the process, as stdout.
        // SAFETY: a valid handle that is not closed elsewhere.
        unsafe { SetStdHandle(STD_OUTPUT_HANDLE, console.into_raw_handle()) };
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::wants_version;

    fn args(list: &[&str]) -> Vec<OsString> {
        list.iter().map(OsString::from).collect()
    }

    #[test]
    fn only_a_lone_version_flag_counts() {
        assert!(wants_version(&args(&["--version"])));
        assert!(!wants_version(&args(&[])));
        assert!(!wants_version(&args(&["--version", "map.rcraft"])));
        assert!(!wants_version(&args(&["map.rcraft"])));
        // A document may be called that; with a path it is a path.
        assert!(!wants_version(&args(&["./--version"])));
        assert!(!wants_version(&args(&["-V"])));
    }
}
