//! Reading `.rcraft` files.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use serde::Deserialize;
use zip::result::ZipError;

use super::{
    DOCUMENT_ENTRY, FileError, IMAGES_DIR, MAX_ASSET_BYTES, MAX_DOCUMENT_BYTES, MAX_TOTAL_BYTES,
    zip_to_io,
};
use crate::model::{Document, SCHEMA_VERSION};
use crate::package::{Assets, Package, asset_key};

/// Reads only the schema version, so newer files are recognised even when
/// the rest of their structure is unknown.
#[derive(Deserialize)]
struct Probe {
    schema_version: u32,
}

pub fn open(path: &Path) -> Result<Document, FileError> {
    open_package(path).map(|p| p.doc)
}

/// Open a `.rcraft` file. A damaged file whose JSON parses but whose
/// structure is inconsistent is refused as [`FileError::Malformed`].
pub fn open_package(path: &Path) -> Result<Package, FileError> {
    open_package_with_limits(path, MAX_DOCUMENT_BYTES, MAX_ASSET_BYTES, MAX_TOTAL_BYTES)
}

/// Reads all of `entry`, failing with `Malformed` if it exceeds `limit`
/// bytes or would push the running `total` past `total_limit`.
fn read_capped(
    entry: impl Read,
    what: &str,
    limit: u64,
    total: &mut u64,
    total_limit: u64,
) -> Result<Vec<u8>, FileError> {
    let mut buf = Vec::new();
    // Never read past what the total still allows, nor one byte past the
    // entry limit (that byte tells "exactly at" from "over").
    let allowed = limit.min(total_limit.saturating_sub(*total));
    let read = entry
        .take(allowed.saturating_add(1))
        .read_to_end(&mut buf)
        .map_err(|e| FileError::Malformed(e.to_string()))? as u64;
    if read > limit {
        return Err(FileError::Malformed(format!(
            "{what} is larger than {limit} bytes"
        )));
    }
    if read > allowed {
        return Err(FileError::Malformed(format!(
            "the file holds more than {total_limit} bytes"
        )));
    }
    *total += read;
    Ok(buf)
}

/// Whether `key` is `"<sha256 of bytes>.<ext>"`.
fn key_matches(key: &str, bytes: &[u8]) -> bool {
    key.split_once('.')
        .is_some_and(|(_, ext)| asset_key(ext, bytes) == key)
}

/// [`open_package`] with explicit caps on the uncompressed document, each
/// image asset and the total.
pub(super) fn open_package_with_limits(
    path: &Path,
    doc_limit: u64,
    asset_limit: u64,
    total_limit: u64,
) -> Result<Package, FileError> {
    let file = File::open(path)?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| match e {
        ZipError::Io(e) => FileError::Io(e),
        _ => FileError::NotAReasonCraftFile,
    })?;
    let mut total = 0;
    let bytes = match zip.by_name(DOCUMENT_ENTRY) {
        Ok(entry) => read_capped(entry, "the document", doc_limit, &mut total, total_limit)?,
        Err(ZipError::FileNotFound) => return Err(FileError::MissingDocument),
        Err(e) => return Err(zip_to_io(e)),
    };
    let text = String::from_utf8(bytes).map_err(|e| FileError::Malformed(e.to_string()))?;
    let probe: Probe =
        serde_json::from_str(&text).map_err(|e| FileError::Malformed(e.to_string()))?;
    if probe.schema_version > SCHEMA_VERSION {
        return Err(FileError::NewerVersion {
            found: probe.schema_version,
            supported: SCHEMA_VERSION,
        });
    }
    let mut doc: Document =
        serde_json::from_str(&text).map_err(|e| FileError::Malformed(e.to_string()))?;
    doc.validate().map_err(FileError::Malformed)?;
    doc.cap_snippet_texts();
    doc.cap_essay_texts();

    let mut assets = Assets::default();
    let mut other_entries = Vec::new();
    for i in 0..zip.len() {
        let entry = zip.by_index(i).map_err(zip_to_io)?;
        let name = entry
            .name()
            .map_err(|e| FileError::Malformed(e.to_string()))?
            .into_owned();
        if name == DOCUMENT_ENTRY || name == IMAGES_DIR {
            continue;
        }
        if let Some(key) = name.strip_prefix(IMAGES_DIR) {
            if entry.is_dir() {
                continue;
            }
            let bytes = read_capped(entry, &name, asset_limit, &mut total, total_limit)?;
            // Keys are content hashes and the image cache is shared by all
            // tabs: an entry whose bytes do not match its key is dropped
            // (shown as missing) so it cannot stand in for another image.
            if key_matches(key, &bytes) {
                assets.insert_raw(key.to_string(), bytes);
            }
        } else if entry.is_dir() {
            other_entries.push((name, Vec::new()));
        } else {
            let bytes = read_capped(entry, &name, total_limit, &mut total, total_limit)?;
            other_entries.push((name, bytes));
        }
    }
    Ok(Package {
        doc,
        assets,
        other_entries,
    })
}
