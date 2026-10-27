//! Building the app for tests: no storage, no real dialogs, links,
//! clipboard or OS locale.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use super::{ReasonCraftApp, Settings};
use crate::actions::FakeOpener;
use crate::canvas::input::Os;
use crate::export::FakeClipboard;
use crate::file_ops::{FakeDialogs, FileDialogs};

impl ReasonCraftApp {
    /// Build the app with default settings, no storage, no OS locale (so
    /// the interface is English), file dialogs that always cancel, a link
    /// opener and clipboard that only record, and the Windows keys.
    pub fn new_for_test() -> Self {
        Self::new_for_test_with(Box::new(FakeDialogs::default()))
    }

    /// Like [`Self::new_for_test`], with the given file dialogs.
    pub fn new_for_test_with(dialogs: Box<dyn FileDialogs>) -> Self {
        Self::build_for_test(Settings::default(), || None, dialogs)
    }

    /// Like [`Self::new_for_test`], on a computer whose OS locale
    /// `os_locale` reads, starting from `settings` (as if restored). The
    /// UI language follows the settings, so a test that calls this may
    /// switch it (see [`crate::i18n`]).
    pub fn new_for_test_with_locale(settings: Settings, os_locale: fn() -> Option<String>) -> Self {
        Self::build_for_test(settings, os_locale, Box::new(FakeDialogs::default()))
    }

    /// Like [`Self::new_for_test`], with autosave to the recovery folder
    /// `dir` (a temp folder: never the user's).
    pub fn new_for_test_with_recovery(dir: PathBuf) -> Self {
        let mut app = Self::new_for_test();
        app.start_recovery(Some(dir));
        app
    }

    fn build_for_test(
        settings: Settings,
        os_locale: fn() -> Option<String>,
        dialogs: Box<dyn FileDialogs>,
    ) -> Self {
        let mut app = Self::with_settings(
            settings,
            os_locale,
            dialogs,
            Box::new(FakeOpener::default()),
            Arc::new(Mutex::new(FakeClipboard::default())),
        );
        // The same keys whatever the test runs on.
        app.os = Os::Windows;
        app
    }
}
