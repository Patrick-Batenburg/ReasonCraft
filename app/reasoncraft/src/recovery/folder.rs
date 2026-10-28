//! Where the per-user recovery folder lives, worked out from the
//! environment with std alone:
//!
//! - Windows: `%LOCALAPPDATA%\ReasonCraft\recovery`
//! - macOS: `$HOME/Library/Application Support/ReasonCraft/recovery`
//! - other Unix: `$XDG_DATA_HOME/ReasonCraft/recovery`, else
//!   `$HOME/.local/share/ReasonCraft/recovery`
//!
//! A variable that is missing, empty or not an absolute path (on Windows
//! one without a drive, such as `\Users\x`, is not) counts as unset. The
//! path logic takes the values as arguments, so tests feed it any
//! environment.

use std::ffi::OsString;
use std::path::PathBuf;

const APP: &str = "ReasonCraft";
const RECOVERY: &str = "recovery";

/// The recovery folder for this platform, from the process environment.
pub(super) fn default_dir() -> Option<PathBuf> {
    let var = std::env::var_os;
    #[cfg(windows)]
    return windows_dir(var("LOCALAPPDATA"));
    #[cfg(target_os = "macos")]
    return macos_dir(var("HOME"));
    #[cfg(all(unix, not(target_os = "macos")))]
    return unix_dir(var("XDG_DATA_HOME"), var("HOME"));
    #[cfg(not(any(windows, unix)))]
    {
        let _ = var;
        None
    }
}

/// `value` as a usable base folder: set, not empty and absolute (on
/// Windows with a drive or UNC prefix: `\Users\x` is relative to the
/// current drive), the same test as `crate::profile` applies.
fn base(value: Option<OsString>) -> Option<PathBuf> {
    let path = PathBuf::from(value?);
    path.is_absolute().then_some(path)
}

#[cfg(any(windows, test))]
pub(super) fn windows_dir(local_app_data: Option<OsString>) -> Option<PathBuf> {
    Some(base(local_app_data)?.join(APP).join(RECOVERY))
}

#[cfg(any(target_os = "macos", test))]
pub(super) fn macos_dir(home: Option<OsString>) -> Option<PathBuf> {
    let support = base(home)?.join("Library").join("Application Support");
    Some(support.join(APP).join(RECOVERY))
}

#[cfg(any(all(unix, not(target_os = "macos")), test))]
pub(super) fn unix_dir(xdg_data_home: Option<OsString>, home: Option<OsString>) -> Option<PathBuf> {
    let data = base(xdg_data_home).or_else(|| Some(base(home)?.join(".local").join("share")))?;
    Some(data.join(APP).join(RECOVERY))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An absolute folder `name` on this platform: `C:\name` on Windows,
    /// `/name` elsewhere, so every platform runs every platform's logic.
    fn abs(name: &str) -> String {
        if cfg!(windows) {
            format!(r"C:\{name}")
        } else {
            format!("/{name}")
        }
    }

    fn s(v: &str) -> Option<OsString> {
        Some(OsString::from(v))
    }

    fn p(parts: &[&str]) -> Option<PathBuf> {
        Some(parts.iter().collect())
    }

    #[test]
    fn windows_uses_local_app_data() {
        let local = abs("Local");
        assert_eq!(
            windows_dir(s(&local)),
            p(&[&local, "ReasonCraft", "recovery"])
        );
        assert_eq!(windows_dir(None), None);
        assert_eq!(windows_dir(s("")), None);
        assert_eq!(windows_dir(s("AppData")), None, "relative");
    }

    /// Rooted but without a drive: relative to the current drive, so not
    /// used (as for `REASONCRAFT_PROFILE_DIR`, see `crate::profile`).
    #[cfg(windows)]
    #[test]
    fn windows_refuses_a_path_without_a_drive() {
        assert_eq!(windows_dir(s(r"\Users\Ann\AppData\Local")), None);
        assert_eq!(windows_dir(s("/Ann/Local")), None);
    }

    #[test]
    fn macos_uses_application_support() {
        let home = abs("ann");
        assert_eq!(
            macos_dir(s(&home)),
            p(&[
                &home,
                "Library",
                "Application Support",
                "ReasonCraft",
                "recovery"
            ])
        );
        assert_eq!(macos_dir(None), None);
        assert_eq!(macos_dir(s("")), None);
        assert_eq!(macos_dir(s("ann")), None, "relative");
    }

    #[test]
    fn unix_prefers_xdg_data_home_then_home() {
        let (data, home) = (abs("data"), abs("ann"));
        let xdg = p(&[&data, "ReasonCraft", "recovery"]);
        assert_eq!(unix_dir(s(&data), s(&home)), xdg);
        let fallback = p(&[&home, ".local", "share", "ReasonCraft", "recovery"]);
        assert_eq!(unix_dir(None, s(&home)), fallback);
        assert_eq!(unix_dir(s(""), s(&home)), fallback, "empty XDG");
        assert_eq!(unix_dir(s("data"), s(&home)), fallback, "relative XDG");
        assert_eq!(unix_dir(None, None), None);
        assert_eq!(unix_dir(s("data"), s("home")), None);
        assert_eq!(unix_dir(None, s("")), None);
    }
}
