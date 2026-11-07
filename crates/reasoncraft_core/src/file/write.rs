//! Writing `.rcraft` files.

use std::fs::File;
use std::io::Write;
use std::path::Path;

use serde::Serialize;
use zip::write::SimpleFileOptions;

use super::atomic::atomic_write;
use super::{DOCUMENT_ENTRY, FileError, GENERATOR, IMAGES_DIR, zip_to_io};
use crate::model::Document;
use crate::package::Package;

/// `document.json`: the document, led by the build that wrote it.
#[derive(Serialize)]
struct Saved<'a> {
    generator: &'static str,
    #[serde(flatten)]
    doc: &'a Document,
}

fn write_archive(pkg: &Package, file: &mut File) -> Result<(), FileError> {
    let saved = Saved {
        generator: GENERATOR,
        doc: &pkg.doc,
    };
    let json =
        serde_json::to_vec_pretty(&saved).map_err(|e| FileError::Malformed(e.to_string()))?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zip.start_file(DOCUMENT_ENTRY, options).map_err(zip_to_io)?;
    zip.write_all(&json)?;
    zip.add_directory(IMAGES_DIR, options).map_err(zip_to_io)?;
    // Images are already compressed (PNG): stored as they are, so a save
    // never deflates them again.
    let stored = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (key, bytes) in pkg.assets.referenced(&pkg.doc) {
        zip.start_file(format!("{IMAGES_DIR}{key}"), stored)
            .map_err(zip_to_io)?;
        zip.write_all(bytes)?;
    }
    for (name, bytes) in &pkg.other_entries {
        if name.ends_with('/') {
            zip.add_directory(name, options).map_err(zip_to_io)?;
        } else {
            zip.start_file(name, options).map_err(zip_to_io)?;
            zip.write_all(bytes)?;
        }
    }
    zip.finish().map_err(zip_to_io)?;
    Ok(())
}

/// Save `pkg` to `path`. Writes a new temporary file next to it and
/// renames it over `path`, so a failed save never damages an existing
/// file (see [`atomic_write`]); a replaced file keeps its permissions.
/// Assets the document does not refer to are not written.
pub fn save_package(pkg: &Package, path: &Path) -> Result<(), FileError> {
    atomic_write(path, false, |file| write_archive(pkg, file))
}

/// [`save_package`] for a private copy: on Unix only the user may read it
/// (mode 0600). Used for recovery copies.
pub fn save_package_private(pkg: &Package, path: &Path) -> Result<(), FileError> {
    atomic_write(path, true, |file| write_archive(pkg, file))
}

/// Save `doc` alone (no assets) to `path`.
pub fn save(doc: &Document, path: &Path) -> Result<(), FileError> {
    save_package(&Package::new(doc.clone()), path)
}
