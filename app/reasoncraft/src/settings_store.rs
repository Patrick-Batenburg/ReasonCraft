//! The settings file as it is on disk now, read apart from eframe's copy
//! (which eframe loads once, at the start): to tell a first run (no window
//! place kept yet) and to merge what another copy of the app saved since.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::settings::{SETTINGS_KEY, Settings};

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

/// Store `settings` in eframe's `storage` (which writes it to `file`),
/// first adding the personal dictionary's words that `file` holds now and
/// `settings` lack: another copy of the app (one that started on its own,
/// see [`crate::instance`]) added them since this one read the file, and
/// eframe writes all its keys whenever one changed (the window's place,
/// say). Words are only ever added, so a union loses none. eframe calls
/// this at most every 30 seconds and on exit; the file is small.
pub fn save(settings: &mut Settings, file: Option<&Path>, storage: &mut dyn eframe::Storage) {
    let on_disk = file
        .and_then(Stored::read)
        .and_then(|stored| eframe::get_value::<Settings>(&stored, SETTINGS_KEY));
    for word in on_disk.map(|s| s.spell_words).unwrap_or_default() {
        if !settings.spell_words.contains(&word) {
            settings.spell_words.push(word);
        }
    }
    eframe::set_value(storage, SETTINGS_KEY, settings);
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
        let settings: Settings = eframe::get_value(&stored, SETTINGS_KEY).unwrap();
        assert_eq!(settings.author, "Ann");
        std::fs::write(&path, "(not ron").unwrap();
        assert!(Stored::read(&path).is_none());
    }

    /// The settings file as eframe writes it, with `words` in the
    /// personal dictionary.
    fn write_file(path: &Path, words: &[&str]) {
        let mut stored = Stored::default();
        let settings = Settings {
            spell_words: words.iter().map(|w| (*w).to_owned()).collect(),
            ..Settings::default()
        };
        eframe::set_value(&mut stored, SETTINGS_KEY, &settings);
        let text = ron::ser::to_string(&stored.0).unwrap();
        std::fs::write(path, text).unwrap();
    }

    /// APP-1, defence in depth: a copy that writes its settings keeps the
    /// words another copy added to the personal dictionary meanwhile.
    #[test]
    fn saving_keeps_words_another_copy_added() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("app.ron");
        // Both copies read ["alpha"]; the other one then added "gamma".
        write_file(&path, &["alpha"]);
        let mut storage = Stored::read(&path).unwrap();
        let mut mine: Settings = eframe::get_value(&storage, SETTINGS_KEY).unwrap();
        write_file(&path, &["alpha", "gamma"]);
        mine.spell_words.push("beta".into());
        save(&mut mine, Some(&path), &mut storage);
        let kept: Settings = eframe::get_value(&storage, SETTINGS_KEY).unwrap();
        assert_eq!(kept.spell_words, ["alpha", "beta", "gamma"]);
        assert_eq!(mine.spell_words, kept.spell_words);
    }
}
