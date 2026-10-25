//! Reading and writing `.rcraft` files (a zip holding `document.json`).

use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use serde::Deserialize;
use zip::result::ZipError;
use zip::write::SimpleFileOptions;

use crate::model::{Document, SCHEMA_VERSION};

const DOCUMENT_ENTRY: &str = "document.json";

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

/// Reads only the schema version, so newer files are recognised even when
/// the rest of their structure is unknown.
#[derive(Deserialize)]
struct Probe {
    schema_version: u32,
}

fn zip_to_io(e: ZipError) -> FileError {
    match e {
        ZipError::Io(e) => FileError::Io(e),
        other => FileError::Io(std::io::Error::other(other)),
    }
}

fn write_archive(doc: &Document, tmp: &Path) -> Result<(), FileError> {
    let json = serde_json::to_vec_pretty(doc).map_err(|e| FileError::Malformed(e.to_string()))?;
    let file = File::create(tmp)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zip.start_file(DOCUMENT_ENTRY, options).map_err(zip_to_io)?;
    zip.write_all(&json)?;
    zip.add_directory("images/", options).map_err(zip_to_io)?;
    let file = zip.finish().map_err(zip_to_io)?;
    file.sync_all()?;
    Ok(())
}

/// Save `doc` to `path`. Writes a temporary file next to it and renames it
/// over `path`, so a failed save never damages an existing file.
pub fn save(doc: &Document, path: &Path) -> Result<(), FileError> {
    let tmp = path.with_extension("rcraft.tmp");
    let result =
        write_archive(doc, &tmp).and_then(|()| std::fs::rename(&tmp, path).map_err(Into::into));
    if result.is_err() && tmp.is_file() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

/// Open a `.rcraft` file.
pub fn open(path: &Path) -> Result<Document, FileError> {
    let file = File::open(path)?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| match e {
        ZipError::Io(e) => FileError::Io(e),
        _ => FileError::NotAReasonCraftFile,
    })?;
    let mut text = String::new();
    match zip.by_name(DOCUMENT_ENTRY) {
        Ok(mut entry) => {
            entry
                .read_to_string(&mut text)
                .map_err(|e| FileError::Malformed(e.to_string()))?;
        }
        Err(ZipError::FileNotFound) => return Err(FileError::MissingDocument),
        Err(e) => return Err(zip_to_io(e)),
    }
    let probe: Probe =
        serde_json::from_str(&text).map_err(|e| FileError::Malformed(e.to_string()))?;
    if probe.schema_version > SCHEMA_VERSION {
        return Err(FileError::NewerVersion {
            found: probe.schema_version,
            supported: SCHEMA_VERSION,
        });
    }
    serde_json::from_str(&text).map_err(|e| FileError::Malformed(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CardKind, Parent};

    fn write_zip(path: &Path, json: &str) {
        let f = std::fs::File::create(path).unwrap();
        let mut z = zip::ZipWriter::new(f);
        let o = zip::write::SimpleFileOptions::default();
        z.start_file("document.json", o).unwrap();
        z.write_all(json.as_bytes()).unwrap();
        z.finish().unwrap();
    }

    fn raw_json(path: &Path) -> serde_json::Value {
        use std::io::Read as _;
        let mut z = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
        let mut s = String::new();
        z.by_name("document.json")
            .unwrap()
            .read_to_string(&mut s)
            .unwrap();
        serde_json::from_str(&s).unwrap()
    }

    #[test]
    fn roundtrip_preserves_structure_and_unicode() {
        let mut d = Document::new("Één — 日本 😀");
        let root = Parent::Box(d.root);
        let (card, _p) = d.add_card(root, CardKind::Reason, "Één — 日本 😀").unwrap();
        d.add_premise(card, None, "co-premise").unwrap();
        d.add_card(Parent::Card(card), CardKind::Objection, "whole-card attack")
            .unwrap();
        d.add_loose(crate::NewLoose::Note, "parked");
        d.add_loose(crate::NewLoose::Source, "src");
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("map.rcraft");
        save(&d, &path).unwrap();
        assert_eq!(open(&path).unwrap(), d);
    }

    #[test]
    fn unknown_fields_survive() {
        let d = Document::new("C");
        let mut v = serde_json::to_value(&d).unwrap();
        v["future_field"] = 1.into();
        v["boxes"][d.root.0.to_string()]["future_field"] = 2.into();
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.rcraft");
        let b = dir.path().join("b.rcraft");
        write_zip(&a, &v.to_string());
        let opened = open(&a).unwrap();
        save(&opened, &b).unwrap();
        let out = raw_json(&b);
        assert_eq!(out["future_field"], 1);
        assert_eq!(out["boxes"][d.root.0.to_string()]["future_field"], 2);
    }

    #[test]
    fn open_rejects_non_zip() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("x.rcraft");
        std::fs::write(&p, "just some text").unwrap();
        assert!(matches!(open(&p), Err(FileError::NotAReasonCraftFile)));
    }

    #[test]
    fn open_rejects_missing_document() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("x.rcraft");
        let mut z = zip::ZipWriter::new(std::fs::File::create(&p).unwrap());
        z.start_file("other.txt", zip::write::SimpleFileOptions::default())
            .unwrap();
        z.finish().unwrap();
        assert!(matches!(open(&p), Err(FileError::MissingDocument)));
    }

    #[test]
    fn open_rejects_bad_json() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("x.rcraft");
        write_zip(&p, "{ not json");
        assert!(matches!(open(&p), Err(FileError::Malformed(_))));
    }

    #[test]
    fn open_rejects_newer_schema() {
        let mut v = serde_json::to_value(Document::new("C")).unwrap();
        v["schema_version"] = 2.into();
        v["something_new"] = serde_json::json!({"shape": "unknown"});
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("x.rcraft");
        write_zip(&p, &v.to_string());
        assert!(matches!(
            open(&p),
            Err(FileError::NewerVersion {
                found: 2,
                supported: 1
            })
        ));
    }

    #[test]
    fn save_is_atomic_on_failure() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("map.rcraft");
        let original = Document::new("original");
        save(&original, &path).unwrap();
        // Block the temp target with a directory of the same name.
        std::fs::create_dir(dir.path().join("map.rcraft.tmp")).unwrap();
        let changed = Document::new("changed");
        assert!(save(&changed, &path).is_err());
        assert_eq!(open(&path).unwrap(), original);
        assert!(dir.path().join("map.rcraft.tmp").is_dir());
    }

    #[test]
    fn save_leaves_no_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("map.rcraft");
        save(&Document::new("c"), &path).unwrap();
        save(&Document::new("d"), &path).unwrap();
        assert!(!dir.path().join("map.rcraft.tmp").exists());
    }
}
