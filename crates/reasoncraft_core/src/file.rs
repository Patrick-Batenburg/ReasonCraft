//! Reading and writing `.rcraft` files: a zip holding `document.json`,
//! image assets under `images/` and any entries from newer builds.

use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use serde::Deserialize;
use zip::result::ZipError;
use zip::write::SimpleFileOptions;

use crate::model::{Document, SCHEMA_VERSION};
use crate::package::{Assets, Package};

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

fn write_archive(pkg: &Package, tmp: &Path) -> Result<(), FileError> {
    let json =
        serde_json::to_vec_pretty(&pkg.doc).map_err(|e| FileError::Malformed(e.to_string()))?;
    let file = File::create(tmp)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zip.start_file(DOCUMENT_ENTRY, options).map_err(zip_to_io)?;
    zip.write_all(&json)?;
    zip.add_directory(IMAGES_DIR, options).map_err(zip_to_io)?;
    for (key, bytes) in pkg.assets.referenced(&pkg.doc) {
        zip.start_file(format!("{IMAGES_DIR}{key}"), options)
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
    let file = zip.finish().map_err(zip_to_io)?;
    file.sync_all()?;
    Ok(())
}

/// Save `pkg` to `path`. Writes a temporary file next to it and renames it
/// over `path`, so a failed save never damages an existing file. Assets the
/// document does not refer to are not written.
pub fn save_package(pkg: &Package, path: &Path) -> Result<(), FileError> {
    let tmp = path.with_extension("rcraft.tmp");
    let result =
        write_archive(pkg, &tmp).and_then(|()| std::fs::rename(&tmp, path).map_err(Into::into));
    if result.is_err() && tmp.is_file() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

/// Save `doc` alone (no assets) to `path`.
pub fn save(doc: &Document, path: &Path) -> Result<(), FileError> {
    save_package(&Package::new(doc.clone()), path)
}

/// Open a `.rcraft` file, dropping its assets and extra entries.
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

/// [`open_package`] with explicit caps on the uncompressed document, each
/// image asset and the total.
fn open_package_with_limits(
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
    let doc: Document =
        serde_json::from_str(&text).map_err(|e| FileError::Malformed(e.to_string()))?;
    doc.validate().map_err(FileError::Malformed)?;

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
            assets.insert_raw(key.to_string(), bytes);
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
        d.add_loose(
            crate::NewLoose::Source(crate::model::SourceType::Source),
            "src",
        );
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

    /// Save `d` unchecked, as a damaged file would look on disk.
    fn write_doc(path: &Path, d: &Document) {
        write_zip(path, &serde_json::to_string(d).unwrap());
    }

    #[test]
    fn open_rejects_dangling_ids() {
        let mut d = Document::new("C");
        let root = d.root;
        d.boxes[&root]
            .children
            .push(crate::Item::Card(crate::CardId::new()));
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("x.rcraft");
        write_doc(&p, &d);
        assert!(matches!(open(&p), Err(FileError::Malformed(m)) if m.contains("missing")));
    }

    #[test]
    fn open_rejects_cycles() {
        let mut d = Document::new("C");
        let (a, p) = d
            .add_card(Parent::Box(d.root), CardKind::Reason, "P")
            .unwrap();
        // Move card `a` under its own premise.
        let root = d.root;
        d.boxes[&root].children.clear();
        d.boxes[&p].children.push(crate::Item::Card(a));
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("x.rcraft");
        write_doc(&path, &d);
        assert!(matches!(open(&path), Err(FileError::Malformed(m)) if m.contains("unreachable")));
    }

    #[test]
    fn open_rejects_oversize_document() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("x.rcraft");
        save(&Document::new("C"), &p).unwrap();
        let size = raw_json(&p).to_string().len() as u64;
        // The pretty-printed entry is larger than its compact form.
        assert!(matches!(
            open_package_with_limits(&p, size, MAX_ASSET_BYTES, MAX_TOTAL_BYTES),
            Err(FileError::Malformed(m)) if m.contains("larger")
        ));
        assert!(open_package_with_limits(&p, size * 4, MAX_ASSET_BYTES, MAX_TOTAL_BYTES).is_ok());
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

    fn zip_with(path: &Path, doc: &Document, extra: &[(&str, &[u8])]) {
        let mut z = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        let o = zip::write::SimpleFileOptions::default();
        z.start_file("document.json", o).unwrap();
        z.write_all(serde_json::to_string(doc).unwrap().as_bytes())
            .unwrap();
        for (n, b) in extra {
            z.start_file(*n, o).unwrap();
            z.write_all(b).unwrap();
        }
        z.finish().unwrap();
    }

    fn entry_names(path: &Path) -> Vec<String> {
        let z = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
        z.file_names().map(|n| n.unwrap().into_owned()).collect()
    }

    fn doc_with_image(key: &str) -> Document {
        let mut d = Document::new("C");
        let root = d.root;
        d.boxes[&root].image = Some(crate::ImageRef {
            asset: key.into(),
            aspect: 0.5,
        });
        d
    }

    #[test]
    fn package_roundtrip_keeps_ratings_comments_links() {
        let mut d = Document::new("C");
        let (_card, premise) = d
            .add_card(Parent::Box(d.root), CardKind::Reason, "R")
            .unwrap();
        let root = d.root;
        d.boxes[&root].acceptance = Some(crate::Acceptance::Accept);
        d.boxes[&premise].link = Some("https://example.org".into());
        d.boxes[&premise].comments.push(crate::Comment {
            id: uuid::Uuid::new_v4(),
            author: "A".into(),
            text: "T".into(),
            verdict: Some(crate::Verdict::Problem),
            created: "2026-01-01T00:00:00Z".into(),
        });
        let mut assets = Assets::default();
        let key = assets.insert("png", vec![1, 2, 3, 4]);
        d.boxes[&root].image = Some(crate::ImageRef {
            asset: key,
            aspect: 0.75,
        });
        let pkg = Package {
            doc: d,
            assets,
            other_entries: vec![("extra/x.bin".into(), vec![7; 10])],
        };
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("m.rcraft");
        save_package(&pkg, &path).unwrap();
        assert_eq!(open_package(&path).unwrap(), pkg);
    }

    #[test]
    fn unknown_zip_entries_survive_save() {
        let d = Document::new("C");
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.rcraft");
        let b = dir.path().join("b.rcraft");
        let thumb: &[u8] = &[0x89, b'P', 0, 255, 1];
        let bin: &[u8] = &[0, 1, 2, 3, 250];
        zip_with(
            &a,
            &d,
            &[("thumbnail.png", thumb), ("future/data.bin", bin)],
        );
        let pkg = open_package(&a).unwrap();
        assert_eq!(pkg.other_entries.len(), 2);
        save_package(&pkg, &b).unwrap();
        let mut z = zip::ZipArchive::new(std::fs::File::open(&b).unwrap()).unwrap();
        for (n, want) in [("thumbnail.png", thumb), ("future/data.bin", bin)] {
            let mut got = Vec::new();
            z.by_name(n).unwrap().read_to_end(&mut got).unwrap();
            assert_eq!(got, want);
        }
        let names = entry_names(&b);
        let pos = |n: &str| names.iter().position(|x| x == n).unwrap();
        assert!(pos("thumbnail.png") < pos("future/data.bin"));
    }

    #[test]
    fn open_with_missing_asset_keeps_image_ref() {
        let d = doc_with_image("abc.png");
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("x.rcraft");
        zip_with(&p, &d, &[]);
        let pkg = open_package(&p).unwrap();
        assert!(pkg.assets.get("abc.png").is_none());
        let root = pkg.doc.root;
        assert!(pkg.doc.boxes[&root].image.is_some());
    }

    #[test]
    fn unreferenced_assets_not_written() {
        let mut pkg = Package::new(Document::new("C"));
        let key = pkg.assets.insert("png", vec![5, 5, 5]);
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("x.rcraft");
        save_package(&pkg, &p).unwrap();
        assert!(!entry_names(&p).contains(&format!("images/{key}")));
        assert_eq!(open_package(&p).unwrap().assets.len(), 0);
        // Referenced ones are written.
        let root = pkg.doc.root;
        pkg.doc.boxes[&root].image = Some(crate::ImageRef {
            asset: key.clone(),
            aspect: 1.0,
        });
        save_package(&pkg, &p).unwrap();
        assert!(entry_names(&p).contains(&format!("images/{key}")));
    }

    #[test]
    fn oversized_asset_rejected() {
        let d = doc_with_image("a.png");
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("x.rcraft");
        zip_with(&p, &d, &[("images/a.png", &[0u8; 100])]);
        let big = 1 << 30;
        assert!(open_package_with_limits(&p, big, 100, big).is_ok());
        assert!(matches!(
            open_package_with_limits(&p, big, 99, big),
            Err(FileError::Malformed(_))
        ));
    }

    #[test]
    fn total_cap_rejected() {
        let d = Document::new("C");
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("x.rcraft");
        zip_with(&p, &d, &[("a.bin", &[0u8; 100]), ("b.bin", &[0u8; 100])]);
        let doc_len = serde_json::to_string(&d).unwrap().len() as u64;
        let big = 1 << 30;
        assert!(open_package_with_limits(&p, big, big, doc_len + 200).is_ok());
        assert!(matches!(
            open_package_with_limits(&p, big, big, doc_len + 199),
            Err(FileError::Malformed(_))
        ));
    }
}
