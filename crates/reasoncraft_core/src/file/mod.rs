//! Reading and writing `.rcraft` files: a zip holding `document.json`,
//! image assets under `images/` and any entries from newer builds.

mod atomic;
mod read;
mod sanitize;
mod write;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_compat;
#[cfg(test)]
mod tests_hostile;
#[cfg(test)]
mod tests_save;

pub use atomic::{atomic_write, create_first_new, create_first_new_with};
pub use read::{open, open_package};
pub use write::{save, save_package, save_package_private};

use zip::result::ZipError;

const DOCUMENT_ENTRY: &str = "document.json";
/// The build that writes a file, in its `"generator"` field.
pub const GENERATOR: &str = concat!("ReasonCraft ", env!("CARGO_PKG_VERSION"));
/// The field of `document.json` that names the build that wrote it.
const GENERATOR_FIELD: &str = "generator";
const IMAGES_DIR: &str = "images/";

/// Largest `document.json` that [`open`] will read (64 MiB, far beyond
/// any real map: every text is capped, see [`crate::caps`]).
const MAX_DOCUMENT_BYTES: u64 = 64 << 20;
/// Largest total size of the unknown JSON fields kept from a newer
/// build (1 MiB). Past it they are all left out: they are copied with
/// every edit, undo step and autosave.
const MAX_EXTRA_BYTES: usize = 1 << 20;
/// Most JSON values (objects, arrays, numbers, strings, ...) a
/// `document.json` may hold: parsed, each takes far more memory than its
/// bytes, so a "newer version" field holding a huge array of tiny values
/// is refused before it is parsed. A map of 10,000 boxes holds
/// about 400,000.
const MAX_JSON_VALUES: usize = 2_000_000;
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
    /// Not saved: opening keeps at most `max` characters of text
    /// ([`crate::caps::MAX_DOCUMENT_CHARS`]), and the map holds `chars`.
    #[error(
        "This map holds {chars} characters of text; a file may hold at most {max}, or it would be cut when opened."
    )]
    TooMuchText { chars: usize, max: usize },
}

fn zip_to_io(e: ZipError) -> FileError {
    match e {
        ZipError::Io(e) => FileError::Io(e),
        other => FileError::Io(std::io::Error::other(other)),
    }
}
