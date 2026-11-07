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
