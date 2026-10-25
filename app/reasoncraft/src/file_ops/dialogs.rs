//! Native file pickers behind a trait, and fakes for tests.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use super::EXTENSION;
use crate::export::ExportFormat;
use crate::i18n::t;

/// The image files a box can show.
pub const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp"];

/// Native file pickers. `None` means the user cancelled.
pub trait FileDialogs {
    fn pick_open(&self) -> Option<PathBuf>;
    /// `suggested` is the proposed file name, extension included.
    fn pick_save(&self, suggested: &str) -> Option<PathBuf>;
    /// An image to put in a box (PNG, JPEG or WebP).
    fn pick_image(&self) -> Option<PathBuf>;
    /// Where to write an export as `format`. `suggested` is the proposed
    /// file name, extension included. The caller adds the extension when
    /// the chosen name lacks it.
    fn pick_export(&self, format: ExportFormat, suggested: &str) -> Option<PathBuf>;
}

/// The platform's own dialogs, filtered to `.rcraft` files (or images).
pub struct RfdDialogs;

impl RfdDialogs {
    fn dialog() -> rfd::FileDialog {
        rfd::FileDialog::new().add_filter(t("file.filter"), &[EXTENSION])
    }
}

impl FileDialogs for RfdDialogs {
    fn pick_open(&self) -> Option<PathBuf> {
        Self::dialog().pick_file()
    }

    fn pick_save(&self, suggested: &str) -> Option<PathBuf> {
        Self::dialog().set_file_name(suggested).save_file()
    }

    fn pick_image(&self) -> Option<PathBuf> {
        rfd::FileDialog::new()
            .add_filter(t("file.images"), IMAGE_EXTENSIONS)
            .pick_file()
    }

    fn pick_export(&self, format: ExportFormat, suggested: &str) -> Option<PathBuf> {
        rfd::FileDialog::new()
            .add_filter(t(format.name_key()), &[format.extension()])
            .set_file_name(suggested)
            .save_file()
    }
}

/// Test dialogs: each answer is queued with `set_open` / `set_save` /
/// `set_image` / `set_export` and used
/// once; with nothing queued the dialog counts as cancelled. Clones share
/// their queue, so a test keeps a clone to steer the app's copy.
#[derive(Clone, Default)]
pub struct FakeDialogs {
    pub open: Rc<RefCell<Option<PathBuf>>>,
    pub save: Rc<RefCell<Option<PathBuf>>>,
    pub image: Rc<RefCell<Option<PathBuf>>>,
    pub export: Rc<RefCell<Option<PathBuf>>>,
    /// The format and suggested name of the last export dialog.
    pub last_export: Rc<RefCell<Option<(ExportFormat, String)>>>,
}

impl FakeDialogs {
    pub fn set_open(&self, path: PathBuf) {
        self.open.replace(Some(path));
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
}
