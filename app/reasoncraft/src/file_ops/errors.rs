//! The messages shown when a file cannot be opened or saved.

use std::path::Path;

use reasoncraft_core::file::FileError;

use super::{FileOp, Modal};
use crate::app::ReasonCraftApp;
use crate::i18n::{FluentArgs, t_args};

/// The i18n key that explains `err`.
pub fn error_key(op: FileOp, err: &FileError) -> &'static str {
    match (op, err) {
        (FileOp::Open, FileError::Io(_)) => "error.open.io",
        (FileOp::Open, FileError::NotAReasonCraftFile) => "error.open.not_a_file",
        (FileOp::Open, FileError::MissingDocument) => "error.open.missing_document",
        (FileOp::Open, FileError::Malformed(_)) => "error.open.malformed",
        (FileOp::Open, FileError::NewerVersion { .. }) => "error.open.newer_version",
        (FileOp::Save, FileError::Io(_)) => "error.save.io",
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
        FileError::Io(e) => args.set("detail", e.to_string()),
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
