use std::io::{Read, Write};
use std::path::Path;

use super::read::open_package_with_limits;
use super::*;
use crate::model::Document;
use crate::package::{Assets, Package, asset_key};
use crate::{CardKind, Parent};

pub(super) fn write_zip(path: &Path, json: &str) {
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

pub(super) fn zip_with(path: &Path, doc: &Document, extra: &[(&str, &[u8])]) {
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
        other_entries: vec![("extra/x.bin".into(), vec![7; 10].into())],
        cut_on_open: false,
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
fn image_entries_are_stored_uncompressed() {
    let mut pkg = Package::new(Document::new("C"));
    let key = pkg.assets.insert("png", vec![0; 4096]);
    let root = pkg.doc.root;
    pkg.doc.boxes[&root].image = Some(crate::ImageRef {
        asset: key.clone(),
        aspect: 1.0,
    });
    pkg.other_entries
        .push(("extra.bin".into(), vec![0; 4096].into()));
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("x.rcraft");
    save_package(&pkg, &p).unwrap();
    let mut z = zip::ZipArchive::new(std::fs::File::open(&p).unwrap()).unwrap();
    let method =
        |z: &mut zip::ZipArchive<std::fs::File>, n: &str| z.by_name(n).unwrap().compression();
    assert_eq!(
        method(&mut z, &format!("images/{key}")),
        zip::CompressionMethod::Stored
    );
    assert_eq!(
        method(&mut z, "document.json"),
        zip::CompressionMethod::Deflated
    );
    assert_eq!(
        method(&mut z, "extra.bin"),
        zip::CompressionMethod::Deflated
    );
    assert_eq!(open_package(&p).unwrap(), pkg);
}

#[test]
fn image_entries_whose_bytes_do_not_match_their_key_are_dropped() {
    let real = asset_key("png", b"real");
    let d = doc_with_image(&real);
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("x.rcraft");
    let other = asset_key("png", b"other");
    zip_with(
        &p,
        &d,
        &[
            (&format!("images/{real}"), b"spoofed"),
            (&format!("images/{other}"), b"other"),
            ("images/nodot", b"x"),
        ],
    );
    let pkg = open_package(&p).unwrap();
    assert!(pkg.assets.get(&real).is_none(), "treated as missing");
    assert_eq!(&*pkg.assets.get(&other).unwrap(), b"other");
    assert_eq!(pkg.assets.len(), 1);
    let root = pkg.doc.root;
    assert!(
        pkg.doc.boxes[&root].image.is_some(),
        "the box keeps its image ref"
    );
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
