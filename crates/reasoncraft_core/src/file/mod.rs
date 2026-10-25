//! Reading and writing `.rcraft` files: a zip holding `document.json`,
//! image assets under `images/` and any entries from newer builds.

mod read;
mod write;

#[cfg(test)]
mod tests;

pub use read::{open, open_package};
pub use write::{save, save_package};

use zip::result::ZipError;

const DOCUMENT_ENTRY: &str = "document.json";
const IMAGES_DIR: &str = "images/";

/// Largest `document.json` that [`open`] will read (256 MiB).
const MAX_DOCUMENT_BYTES: u64 = 256 << 20;
/// Largest single image asset (32 MiB).
const MAX_ASSET_BYTES: u64 = 32 << 20;
/// Largest total of uncompressed bytes read from one file (512 MiB).
const MAX_TOTAL_BYTES: u64 = 512 << 20;

/// Why a file could not be read or written.
#[derive(Debug, thiserror::Error)]
pub enum FileError {
    #[error("The file could not be read or written: {0}")]
    Io(#[from] std::io::Error),
    #[error("This is not a ReasonCraft file.")]
    NotAReasonCraftFile,
    #[error("This ReasonCraft file has no document inside.")]
    MissingDocument,
    #[error("This ReasonCraft file is damaged: {0}")]
    Malformed(String),
    #[error(
        "This file was made with a newer version of ReasonCraft (file version {found}, this version supports {supported})."
    )]
    NewerVersion { found: u32, supported: u32 },
}

fn zip_to_io(e: ZipError) -> FileError {
    match e {
        ZipError::Io(e) => FileError::Io(e),
        other => FileError::Io(std::io::Error::other(other)),
    }
}
