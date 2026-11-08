//! Native file pickers behind a trait, and fakes for tests.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

use super::EXTENSION;
use crate::export::ExportFormat;
use crate::i18n::t;

/// The extensions of Argdown files.
pub const ARGDOWN_EXTENSIONS: &[&str] = &["argdown", "ad"];

/// The image files a box can show.
pub const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp"];

/// Native file pickers. `None` means the user cancelled.
pub trait FileDialogs {
    fn pick_open(&self) -> Option<PathBuf>;
    /// An Argdown file to import.
    fn pick_import(&self) -> Option<PathBuf>;
    /// `suggested` is the proposed file name, extension included.
    fn pick_save(&self, suggested: &str) -> Option<PathBuf>;
    /// An image to put in a box (PNG, JPEG or WebP).
    fn pick_image(&self) -> Option<PathBuf>;
    /// Where to write an export as `format`. `suggested` is the proposed
    /// file name, extension included. The caller adds the extension when
    /// the chosen name lacks it.
    fn pick_export(&self, format: ExportFormat, suggested: &str) -> Option<PathBuf>;
    /// Whether the last dialog could not be shown at all, so the app says
    /// what to install instead of doing nothing (see [`RfdDialogs`]);
    /// asking clears it.
    fn take_unavailable(&self) -> bool {
        false
    }
}

/// The platform's own dialogs, filtered to `.rcraft` files (or images).
///
/// On Linux and the BSDs rfd asks the desktop portal and, when that
/// fails, runs zenity; with neither it returns no file at once, as if
/// cancelled. A dialog that comes back empty sooner than anyone
/// could cancel it counts as not shown.
#[derive(Default)]
pub struct RfdDialogs {
    unavailable: Cell<bool>,
}

/// Sooner than this, an empty answer means no dialog was shown.
const NO_DIALOG_WITHIN: Duration = Duration::from_millis(200);

/// Whether a dialog that answered `empty` after `took` was never shown.
pub(crate) fn came_back_at_once(empty: bool, took: Duration) -> bool {
    empty && took < NO_DIALOG_WITHIN
}

impl RfdDialogs {
    fn dialog() -> rfd::FileDialog {
        rfd::FileDialog::new().add_filter(t("file.filter"), &[EXTENSION])
    }

    /// Runs `pick`, noting whether its dialog was never shown (only where
    /// rfd's dialogs can be missing).
    fn run(&self, pick: impl FnOnce() -> Option<PathBuf>) -> Option<PathBuf> {
        let start = Instant::now();
        let path = pick();
        let missing = cfg!(all(unix, not(target_os = "macos")))
            && came_back_at_once(path.is_none(), start.elapsed());
        self.unavailable.set(missing);
        path
    }
}

impl FileDialogs for RfdDialogs {
    fn pick_open(&self) -> Option<PathBuf> {
        self.run(|| Self::dialog().pick_file())
    }

    fn pick_save(&self, suggested: &str) -> Option<PathBuf> {
        self.run(|| Self::dialog().set_file_name(suggested).save_file())
    }

    fn pick_import(&self) -> Option<PathBuf> {
        self.run(|| {
            rfd::FileDialog::new()
                .add_filter(t("file.argdown"), ARGDOWN_EXTENSIONS)
                .pick_file()
        })
    }

    fn pick_image(&self) -> Option<PathBuf> {
        self.run(|| {
            rfd::FileDialog::new()
                .add_filter(t("file.images"), IMAGE_EXTENSIONS)
                .pick_file()
        })
    }

    fn pick_export(&self, format: ExportFormat, suggested: &str) -> Option<PathBuf> {
        self.run(|| {
            rfd::FileDialog::new()
                .add_filter(t(format.name_key()), &[format.extension()])
                .set_file_name(suggested)
                .save_file()
        })
    }

    fn take_unavailable(&self) -> bool {
        self.unavailable.take()
    }
}

/// Test dialogs: each answer is queued with `set_open` / `set_import` /
/// `set_save` / `set_image` / `set_export` and used
/// once; with nothing queued the dialog counts as cancelled. Clones share
/// their queue, so a test keeps a clone to steer the app's copy.
#[derive(Clone, Default)]
pub struct FakeDialogs {
    pub open: Rc<RefCell<Option<PathBuf>>>,
    pub import: Rc<RefCell<Option<PathBuf>>>,
    pub save: Rc<RefCell<Option<PathBuf>>>,
    pub image: Rc<RefCell<Option<PathBuf>>>,
    pub export: Rc<RefCell<Option<PathBuf>>>,
    /// The format and suggested name of the last export dialog.
    pub last_export: Rc<RefCell<Option<(ExportFormat, String)>>>,
    /// Set: the next dialog counts as not shown (no portal, no zenity).
    pub unavailable: Rc<Cell<bool>>,
}

impl FakeDialogs {
    pub fn set_open(&self, path: PathBuf) {
        self.open.replace(Some(path));
    }

    pub fn set_import(&self, path: PathBuf) {
        self.import.replace(Some(path));
    }

    pub fn set_save(&self, path: PathBuf) {
        self.save.replace(Some(path));
    }

    pub fn set_image(&self, path: PathBuf) {
        self.image.replace(Some(path));
    }

    pub fn set_export(&self, path: PathBuf) {
        self.export.replace(Some(path));
    }
}

impl FileDialogs for FakeDialogs {
    fn pick_open(&self) -> Option<PathBuf> {
        self.open.take()
    }

    fn pick_import(&self) -> Option<PathBuf> {
        self.import.take()
    }

    fn pick_save(&self, _suggested: &str) -> Option<PathBuf> {
        self.save.take()
    }

    fn pick_image(&self) -> Option<PathBuf> {
        self.image.take()
    }

    fn pick_export(&self, format: ExportFormat, suggested: &str) -> Option<PathBuf> {
        self.last_export
            .replace(Some((format, suggested.to_owned())));
        self.export.take()
    }

    fn take_unavailable(&self) -> bool {
        self.unavailable.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Nobody cancels a dialog within 200 ms: an empty answer that soon
    /// means no dialog was shown; a file, or a later empty answer, is the
    /// user's.
    #[test]
    fn an_empty_answer_at_once_means_no_dialog() {
        assert!(came_back_at_once(true, Duration::from_millis(5)));
        assert!(!came_back_at_once(true, Duration::from_millis(900)));
        assert!(!came_back_at_once(false, Duration::from_millis(5)));
    }

    /// A dialog that was never shown (Open, Import, Save As, an image)
    /// says what to install; a cancelled one says nothing.
    #[test]
    fn a_missing_dialog_says_what_to_install() {
        use crate::actions::{Action, apply};
        use crate::app::ReasonCraftApp;
        use crate::file_ops::Modal;
        let title = |app: &ReasonCraftApp| match &app.modal {
            Some(Modal::Error { title, .. }) => Some(title.clone()),
            _ => None,
        };
        let dialogs = FakeDialogs::default();
        let mut app = ReasonCraftApp::new_for_test_with(Box::new(dialogs.clone()));
        apply(&mut app, Action::Open);
        assert_eq!(title(&app), None, "cancelled");
        for action in [Action::Open, Action::ImportArgdown, Action::SaveAs] {
            dialogs.unavailable.set(true);
            apply(&mut app, action.clone());
            assert_eq!(title(&app), Some(t("error.no_dialog.title")), "{action:?}");
            app.modal = None;
        }
    }
}
