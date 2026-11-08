//! Files from other builds: unknown fields at every level survive a
//! roundtrip, damaged entries other than the document only lose
//! themselves, and every file says which build wrote it.

use serde_json::{Value, json};

use super::tests::write_zip;
use super::*;
use crate::model::{Document, ImageRef};

/// `v` written as a file, opened, saved again and read back as JSON.
fn roundtrip(v: &Value) -> Value {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = (dir.path().join("a.rcraft"), dir.path().join("b.rcraft"));
    write_zip(&a, &v.to_string());
    save_package(&open_package(&a).unwrap(), &b).unwrap();
    serde_json::to_value(open(&b).unwrap()).unwrap()
}

#[test]
fn unknown_fields_in_nested_objects_survive() {
    let mut d = Document::new("C");
    let root = d.root;
    d.add_comment(root, "me", "hi", None).unwrap();
    d.set_image(root, Some(ImageRef::new("a.png".into(), 1.0)))
        .unwrap();
    let mut v = serde_json::to_value(&d).unwrap();
    let rid = root.0.to_string();
    v["meta"]["subject"] = "future".into();
    v["boxes"][&rid]["comments"][0]["resolved"] = true.into();
    v["boxes"][&rid]["image"]["alt"] = "a picture".into();
    v["ignored_hints"] = json!([{"kind": "k", "item": {"Box": rid}, "until": 3}]);
    v["essay"] = json!({"unplaced": [{"label": "L", "text": "T", "moved": "today"}]});

    let w = roundtrip(&v);
    assert_eq!(w["meta"]["subject"], "future");
    assert_eq!(w["boxes"][&rid]["comments"][0]["resolved"], true);
    assert_eq!(w["boxes"][&rid]["image"]["alt"], "a picture");
    assert_eq!(w["ignored_hints"][0]["until"], 3);
    assert_eq!(
        w["essay"]["unplaced"][0],
        json!({"label": "L", "text": "T", "moved": "today"})
    );
}

#[test]
fn unplaced_notes_from_before_0_1_0_still_read() {
    let mut v = serde_json::to_value(Document::new("C")).unwrap();
    v["essay"] = json!({"unplaced": [["Gone", "Its paragraph"]]});
    let w = roundtrip(&v);
    assert_eq!(
        w["essay"]["unplaced"],
        json!([{"label": "Gone", "text": "Its paragraph"}])
    );
}

/// The top-level JSON of the file at `path`.
fn raw(path: &std::path::Path) -> Value {
    use std::io::Read as _;
    let mut z = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
    let mut s = String::new();
    let mut entry = z.by_name("document.json").unwrap();
    entry.read_to_string(&mut s).unwrap();
    serde_json::from_str(&s).unwrap()
}

#[test]
fn files_name_the_build_that_wrote_them() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = (dir.path().join("a.rcraft"), dir.path().join("b.rcraft"));
    let d = Document::new("C");
    save(&d, &a).unwrap();
    let ours = format!("ReasonCraft {}", env!("CARGO_PKG_VERSION"));
    assert_eq!(raw(&a)["generator"], ours.as_str());
    assert_eq!(open(&a).unwrap(), d, "not part of the document");

    // A file from another build keeps saying so until it is saved here.
    let mut v = raw(&a);
    v["generator"] = "ReasonCraft 9.9.0".into();
    write_zip(&a, &v.to_string());
    let pkg = open_package(&a).unwrap();
    assert_eq!(pkg.generator.as_deref(), Some("ReasonCraft 9.9.0"));
    assert!(pkg.doc.extra.is_empty());
    save_package(&pkg, &b).unwrap();
    assert_eq!(raw(&b)["generator"], ours.as_str());
}

/// A file holding `doc`, an image and an unknown entry, all stored
/// uncompressed, with one byte flipped in each entry named in `damage`.
fn file_with_damage(path: &std::path::Path, doc: &Document, image: &[u8], damage: &[&str]) {
    use std::io::Write as _;
    let stored =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    let json = serde_json::to_vec(doc).unwrap();
    let key = crate::package::asset_key("png", image);
    let entries: [(String, &[u8]); 3] = [
        ("document.json".into(), &json),
        (format!("images/{key}"), image),
        ("future/a.bin".into(), &[0xCD; 500]),
    ];
    let mut z = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, bytes) in &entries {
        z.start_file(name.as_str(), stored).unwrap();
        z.write_all(bytes).unwrap();
    }
    let mut bytes = z.finish().unwrap().into_inner();
    for (name, entry) in &entries {
        if damage.iter().any(|d| name.starts_with(d)) {
            let at = bytes
                .windows(entry.len())
                .position(|w| w == *entry)
                .unwrap();
            bytes[at + entry.len() / 2] ^= 1;
        }
    }
    std::fs::write(path, bytes).unwrap();
}

#[test]
fn a_damaged_image_or_unknown_entry_only_loses_itself() {
    let mut d = Document::new("Claim");
    let root = d.root;
    let image = vec![0xAB; 3000];
    let key = crate::package::asset_key("png", &image);
    d.boxes[&root].image = Some(ImageRef::new(key.clone(), 1.0));
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.rcraft");

    file_with_damage(&path, &d, &image, &[]);
    let pkg = open_package(&path).unwrap();
    assert!(!pkg.cut_on_open && pkg.assets.get(&key).is_some());
    assert_eq!(pkg.other_entries.len(), 1);

    file_with_damage(&path, &d, &image, &["images/", "future/"]);
    let pkg = open_package(&path).expect("the map itself is whole");
    assert!(pkg.cut_on_open, "the user is told");
    assert_eq!(pkg.doc, d, "the box still names its image (shown missing)");
    assert!(pkg.assets.get(&key).is_none());
    assert!(pkg.other_entries.is_empty());

    // A damaged document is still refused.
    file_with_damage(&path, &d, &image, &["document.json"]);
    assert!(matches!(open_package(&path), Err(FileError::Malformed(_))));
}
