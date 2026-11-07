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
