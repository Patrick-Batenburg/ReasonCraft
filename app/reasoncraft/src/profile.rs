//! An optional separate profile folder, for a portable install or for
//! testing without touching your own settings.
//!
//! When the environment variable `REASONCRAFT_PROFILE_DIR` holds an
//! absolute path, the settings file (`app.ron`) and the recovery folder
//! live under it instead of in the per-user locations. A variable that is
//! missing, empty or relative counts as unset.

use std::ffi::OsString;
use std::path::PathBuf;

/// The environment variable that names the profile folder.
pub const PROFILE_DIR_VAR: &str = "REASONCRAFT_PROFILE_DIR";

/// Where the settings and the recovery folder live.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    /// The settings file for eframe's storage; `None` keeps eframe's
    /// per-user default.
    pub settings_file: Option<PathBuf>,
    /// The recovery folder; `None` when there is nowhere to keep one.
    pub recovery_dir: Option<PathBuf>,
}

impl Profile {
    /// The profile from the process environment.
    pub fn from_env() -> Self {
        Self::resolve(
            std::env::var_os(PROFILE_DIR_VAR),
            crate::recovery::RecoveryStore::default_dir(),
        )
    }

    /// The profile for the variable's `value`, falling back to
    /// `default_recovery` when the variable is unset.
    pub fn resolve(value: Option<OsString>, default_recovery: Option<PathBuf>) -> Self {
        match value.map(PathBuf::from).filter(|p| p.has_root()) {
            Some(dir) => Self {
                settings_file: Some(dir.join("app.ron")),
                recovery_dir: Some(dir.join("recovery")),
            },
            None => Self {
                settings_file: None,
                recovery_dir: default_recovery,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_recovery() -> Option<PathBuf> {
        Some(PathBuf::from("/home/ann/recovery"))
    }

    #[test]
    fn an_absolute_folder_holds_settings_and_recovery() {
        let profile = Profile::resolve(Some("/tmp/rc-profile".into()), default_recovery());
        let dir = PathBuf::from("/tmp/rc-profile");
        assert_eq!(profile.settings_file, Some(dir.join("app.ron")));
        assert_eq!(profile.recovery_dir, Some(dir.join("recovery")));
    }

    #[test]
    fn unset_empty_or_relative_keeps_the_defaults() {
        let defaults = Profile {
            settings_file: None,
            recovery_dir: default_recovery(),
        };
        for value in [None, Some(""), Some("profile")] {
            let profile = Profile::resolve(value.map(OsString::from), default_recovery());
            assert_eq!(profile, defaults, "{value:?}");
        }
    }
}
