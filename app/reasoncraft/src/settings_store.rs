//! The settings file as it is on disk now, read apart from eframe's copy
//! (which eframe loads once, at the start): to tell a first run (no window
//! place kept yet) and to merge what another copy of the app saved since.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The app ID (the window's on Linux; also eframe's settings folder name).
pub const APP_ID: &str = "io.github.Patrick_Batenburg.ReasonCraft";

/// eframe's key for the window's place and size.
const WINDOW_KEY: &str = "window";

/// The settings file eframe uses: `profile_file` when a profile folder
/// gives one, else eframe's per-user default for [`APP_ID`].
pub fn settings_file(profile_file: Option<PathBuf>) -> Option<PathBuf> {
    profile_file.or_else(|| eframe::storage_dir(APP_ID).map(|dir| dir.join("app.ron")))
}

/// The settings file's entries (eframe's format: RON text values by key).
#[derive(Default)]
pub struct Stored(HashMap<String, String>);

impl Stored {
    /// What `path` holds now; `None` when it is missing or cannot be read
    /// (also while another copy of the app is writing it).
    pub fn read(path: &Path) -> Option<Self> {
        let text = std::fs::read_to_string(path).ok()?;
        ron::from_str(&text).ok().map(Self)
    }

    /// Whether the window's place and size were kept (not the first run).
    pub fn has_window(&self) -> bool {
        self.0.contains_key(WINDOW_KEY)
    }
}

impl eframe::Storage for Stored {
    fn get_string(&self, key: &str) -> Option<String> {
        self.0.get(key).cloned()
    }

    fn set_string(&mut self, key: &str, value: String) {
        self.0.insert(key.to_owned(), value);
    }

    fn remove_string(&mut self, key: &str) {
        self.0.remove(key);
    }

    fn flush(&mut self) {}
}

/// Whether eframe's storage `storage` has the window's place and size.
pub fn storage_has_window(storage: Option<&dyn eframe::Storage>) -> bool {
    storage.is_some_and(|s| s.get_string(WINDOW_KEY).is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_what_eframe_writes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("app.ron");
        assert!(Stored::read(&path).is_none());
        let mut kv = HashMap::new();
        kv.insert("window".to_owned(), "(inner_size: None)".to_owned());
        kv.insert("settings".to_owned(), "(author: \"Ann\")".to_owned());
        let text = ron::ser::to_string_pretty(&kv, ron::ser::PrettyConfig::default()).unwrap();
        std::fs::write(&path, text).unwrap();
        let stored = Stored::read(&path).unwrap();
        assert!(stored.has_window());
        let settings: crate::settings::Settings =
            eframe::get_value(&stored, crate::settings::SETTINGS_KEY).unwrap();
        assert_eq!(settings.author, "Ann");
        std::fs::write(&path, "(not ron").unwrap();
        assert!(Stored::read(&path).is_none());
    }
}
