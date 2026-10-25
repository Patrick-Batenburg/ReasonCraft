//! Snippets in the file format: unknown fields roundtrip, oversized text
//! is capped on open.

use std::io::{Read, Write};
use std::path::Path;

use reasoncraft_core::Document;
use reasoncraft_core::file::{open, save};
use reasoncraft_core::model::SourceType;
use reasoncraft_core::snippets::{MAX_SNIPPET_CHARS, SnippetSource};

fn write_zip(path: &Path, json: &str) {
    let f = std::fs::File::create(path).unwrap();
    let mut z = zip::ZipWriter::new(f);
    z.start_file("document.json", zip::write::SimpleFileOptions::default())
        .unwrap();
    z.write_all(json.as_bytes()).unwrap();
    z.finish().unwrap();
}

fn raw_json(path: &Path) -> serde_json::Value {
    let mut z = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
    let mut s = String::new();
    z.by_name("document.json")
        .unwrap()
        .read_to_string(&mut s)
        .unwrap();
    serde_json::from_str(&s).unwrap()
}

#[test]
fn unknown_snippet_fields_survive_open_and_save() {
    let mut d = Document::new("C");
    let id = d.add_snippet("text");
    let source = SnippetSource::new(SourceType::Web, "Doe 2020", Some("https://x.org"));
    d.set_snippet_source(id, Some(source)).unwrap();
    let mut v = serde_json::to_value(&d).unwrap();
    v["snippets"][0]["highlight"] = "yellow".into();
    v["snippets"][0]["source"]["page"] = 12.into();
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = (dir.path().join("a.rcraft"), dir.path().join("b.rcraft"));
    write_zip(&a, &v.to_string());

    let opened = open(&a).unwrap();
    assert_eq!(opened.snippets[0].extra["highlight"], "yellow");
    let src = opened.snippets[0].source.as_ref().unwrap();
    assert_eq!(src.extra["page"], 12);
    assert_eq!(src.citation, "Doe 2020");
    save(&opened, &b).unwrap();
    let out = raw_json(&b);
    assert_eq!(out["snippets"][0]["highlight"], "yellow");
    assert_eq!(out["snippets"][0]["source"]["page"], 12);

    // Known fields never land in `extra`, and an empty `extra` writes
    // nothing.
    let plain = serde_json::to_value(&d).unwrap();
    let snippet = plain["snippets"][0].as_object().unwrap();
    let mut keys: Vec<&str> = snippet.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, ["id", "source", "text"]);
    assert!(d.snippets[0].extra.is_empty());
}

#[test]
fn oversized_snippet_text_is_capped_on_open() {
    let mut d = Document::new("C");
    d.add_snippet("short");
    d.add_snippet("x");
    let mut v = serde_json::to_value(&d).unwrap();
    // A multi-byte text well over the cap, as a hand-edited or foreign
    // file could hold.
    let huge: String = "é日a".chars().cycle().take(3 * MAX_SNIPPET_CHARS).collect();
    v["snippets"][1]["text"] = huge.clone().into();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("big.rcraft");
    write_zip(&path, &v.to_string());

    let opened = open(&path).unwrap();
    assert_eq!(opened.snippets[0].text, "short");
    let text = &opened.snippets[1].text;
    assert_eq!(text.chars().count(), MAX_SNIPPET_CHARS);
    assert!(huge.starts_with(text.as_str()));
    // Capping on open is not an edit.
    assert_eq!(opened.meta.modified, d.meta.modified);
}
