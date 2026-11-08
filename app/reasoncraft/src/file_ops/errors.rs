//! The messages shown when a file cannot be opened or saved.

use std::path::Path;

use reasoncraft_core::file::FileError;

use super::{FileOp, Modal};
use crate::app::ReasonCraftApp;
use crate::i18n::{FluentArgs, t, t_args};

/// Why reading or writing a file failed, as far as a message tells apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum IoReason {
    /// No permission, a read-only file or disk, or (Windows) a file
    /// another program holds open.
    Denied,
    NotFound,
    DiskFull,
    Other,
}

fn io_reason(e: &std::io::Error) -> IoReason {
    use std::io::ErrorKind as K;
    // ERROR_SHARING_VIOLATION, ERROR_LOCK_VIOLATION.
    if cfg!(windows) && matches!(e.raw_os_error(), Some(32 | 33)) {
        return IoReason::Denied;
    }
    match e.kind() {
        K::PermissionDenied | K::ReadOnlyFilesystem => IoReason::Denied,
        K::NotFound => IoReason::NotFound,
        K::StorageFull | K::QuotaExceeded => IoReason::DiskFull,
        _ => IoReason::Other,
    }
}

/// The OS's text for `e`, without its "(os error N)" tail and closing
/// full stop (the message goes on after it).
pub fn io_detail(e: &std::io::Error) -> String {
    let text = e.to_string();
    let text = match e.raw_os_error() {
        Some(code) => text
            .strip_suffix(&format!("(os error {code})"))
            .map_or(text.as_str(), str::trim_end),
        None => text.as_str(),
    };
    text.trim_end_matches('.').to_owned()
}

/// The i18n key that explains `err`.
pub fn error_key(op: FileOp, err: &FileError) -> &'static str {
    match (op, err) {
        (FileOp::Open, FileError::Io(e)) => match io_reason(e) {
            IoReason::Denied => "error.open.denied",
            IoReason::NotFound => "error.open.not_found",
            IoReason::DiskFull | IoReason::Other => "error.open.io",
        },
        (FileOp::Save, FileError::Io(e)) => match io_reason(e) {
            IoReason::Denied => "error.save.read_only",
            IoReason::DiskFull => "error.save.disk_full",
            IoReason::NotFound | IoReason::Other => "error.save.io",
        },
        (FileOp::Open, FileError::NotAReasonCraftFile) => "error.open.not_a_file",
        (FileOp::Open, FileError::MissingDocument) => "error.open.missing_document",
        (FileOp::Open, FileError::Malformed(_)) => "error.open.malformed",
        (FileOp::Open, FileError::NewerVersion { .. }) => "error.open.newer_version",
        (_, FileError::TooMuchText { .. }) => "error.save.too_much_text",
        (FileOp::Save, _) => "error.save.other",
    }
}

/// The user-facing explanation of `err`.
pub fn error_text(op: FileOp, err: &FileError) -> String {
    let mut args = FluentArgs::new();
    match err {
        FileError::NewerVersion { found, supported } => {
            args.set("found", *found);
            args.set("supported", *supported);
        }
        FileError::TooMuchText { chars, max } => {
            args.set("chars", *chars);
            args.set("max", *max);
        }
        FileError::Io(e) => {
            args.set("detail", io_detail(e));
            args.set("save_as", crate::i18n::t("action.save_as"));
        }
        FileError::Malformed(detail) => args.set("detail", detail.clone()),
        FileError::NotAReasonCraftFile | FileError::MissingDocument => {
            args.set("detail", err.to_string());
        }
    }
    t_args(error_key(op, err), &args)
}

pub(crate) fn show_error(app: &mut ReasonCraftApp, op: FileOp, path: &Path, err: &FileError) {
    let mut args = FluentArgs::new();
    let file = path.file_name().unwrap_or(path.as_os_str());
    args.set("file", file.to_string_lossy().into_owned());
    let title_key = match op {
        FileOp::Open => "error.open.title",
        FileOp::Save => "error.save.title",
    };
    app.modal = Some(Modal::Error {
        title: t_args(title_key, &args),
        message: error_text(op, err),
    });
}

/// When the last file dialog could not be shown at all (Linux without
/// a desktop portal backend or zenity, see [`super::RfdDialogs`]), say
/// what to install, unless another message is up already.
pub(crate) fn explain_missing_dialog(app: &mut ReasonCraftApp) {
    if app.dialogs.take_unavailable() && app.modal.is_none() {
        app.modal = Some(Modal::Error {
            title: t("error.no_dialog.title"),
            message: t("error.no_dialog"),
        });
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::*;
    use FileOp::{Open, Save};

    fn io_err(kind: io::ErrorKind) -> FileError {
        FileError::Io(io::Error::new(kind, "detail"))
    }

    /// The usual reasons get a message of their own, in the UI's
    /// language, with what to do.
    #[test]
    fn common_io_errors_are_explained() {
        let denied = io_err(io::ErrorKind::PermissionDenied);
        assert_eq!(error_key(Save, &denied), "error.save.read_only");
        assert_eq!(error_key(Open, &denied), "error.open.denied");
        let read_only_disk = io_err(io::ErrorKind::ReadOnlyFilesystem);
        assert_eq!(error_key(Save, &read_only_disk), "error.save.read_only");
        let gone = io_err(io::ErrorKind::NotFound);
        assert_eq!(error_key(Open, &gone), "error.open.not_found");
        let full = io_err(io::ErrorKind::StorageFull);
        assert_eq!(error_key(Save, &full), "error.save.disk_full");
        let other = io_err(io::ErrorKind::Other);
        assert_eq!(error_key(Save, &other), "error.save.io");
        assert_eq!(error_key(Open, &other), "error.open.io");
    }

    /// A file another program holds open (Windows: sharing or lock
    /// violation) is "in use", like a read-only one.
    #[cfg(windows)]
    #[test]
    fn a_file_in_use_is_explained_on_windows() {
        for code in [32, 33] {
            let err = FileError::Io(io::Error::from_raw_os_error(code));
            assert_eq!(error_key(Save, &err), "error.save.read_only", "{code}");
        }
    }

    /// The OS's own text loses its "(os error 5)" tail.
    #[test]
    fn no_os_error_numbers_in_messages() {
        for code in [2, 5, 13, 28] {
            for op in [Open, Save] {
                let text = error_text(op, &FileError::Io(io::Error::from_raw_os_error(code)));
                assert!(!text.contains("os error"), "{text}");
            }
        }
        let detail = io_detail(&io::Error::from_raw_os_error(2));
        assert!(!detail.is_empty() && !detail.ends_with('.'), "{detail:?}");
    }

    #[test]
    fn the_read_only_message_points_to_save_as() {
        let text = error_text(Save, &io_err(io::ErrorKind::PermissionDenied));
        assert!(text.contains(&crate::i18n::t("action.save_as")), "{text}");
    }
}
