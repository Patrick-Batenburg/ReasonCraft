//! Hostile and oversized files: what opening one keeps, cuts or drops.

use std::sync::Arc;

use super::tests::zip_with;
use super::*;
use crate::caps::{MAX_COMMENT_CHARS, MAX_LINK_CHARS, MAX_META_CHARS};
use crate::model::{Document, MAX_BOX_CHARS};
use crate::package::asset_key;

fn chars(s: &str) -> usize {
    s.chars().count()
}

/// A document with every displayed string far over its cap.
fn oversized() -> Document {
    let mut d = Document::new("c");
    let root = d.root;
    let node = &mut d.boxes[&root];
    node.text = "é".repeat(MAX_BOX_CHARS + 10);
    node.link = Some(format!(
        "https://example.org/{}",
        "a".repeat(MAX_LINK_CHARS)
    ));
    node.comments.push(crate::model::Comment {
        id: crate::Uuid::new_v4(),
        author: "A".repeat(MAX_META_CHARS + 1),
        text: "日".repeat(MAX_COMMENT_CHARS + 1),
        verdict: None,
        created: "2".repeat(MAX_META_CHARS + 1),
    });
    let other = crate::SourceType::Other("o".repeat(MAX_META_CHARS + 1));
    d.add_box(
        crate::Parent::Box(root),
        crate::BoxKind::Source {
            source_type: other.clone(),
        },
        "s",
    )
    .unwrap();
    let mut source = crate::SnippetSource::new(other, &"c".repeat(MAX_BOX_CHARS + 1), None);
    source.link = Some("l".repeat(MAX_LINK_CHARS + 1));
    let id = d.add_snippet("text");
    d.snippets.iter_mut().find(|s| s.id == id).unwrap().source = Some(source);
    d.meta.title = "T".repeat(MAX_META_CHARS + 1);
    d.meta.author = "W".repeat(MAX_META_CHARS + 1);
    d
}

fn open_doc(d: &Document, extra: &[(&str, &[u8])]) -> crate::Package {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("x.rcraft");
    zip_with(&p, d, extra);
    open_package(&p).unwrap()
}

#[test]
fn oversized_texts_are_cut_on_open() {
    let pkg = open_doc(&oversized(), &[]);
    assert!(pkg.cut_on_open, "the open says something was cut");
    let d = &pkg.doc;
    let root = &d.boxes[&d.root];
    assert_eq!(chars(&root.text), MAX_BOX_CHARS);
    assert_eq!(root.link, None, "a cut link would lead elsewhere: dropped");
    let c = &root.comments[0];
    assert_eq!(chars(&c.text), MAX_COMMENT_CHARS);
    assert_eq!(chars(&c.author), MAX_META_CHARS);
    assert_eq!(chars(&c.created), MAX_META_CHARS);
    assert_eq!(chars(&d.meta.title), MAX_META_CHARS);
    assert_eq!(chars(&d.meta.author), MAX_META_CHARS);
    let name = |t: &crate::SourceType| match t {
        crate::SourceType::Other(n) => chars(n),
        _ => 0,
    };
    let src = d.boxes.values().find_map(|b| match &b.kind {
        crate::BoxKind::Source { source_type } => Some(source_type),
        _ => None,
    });
    assert_eq!(name(src.unwrap()), MAX_META_CHARS);
    let source = d.snippets[0].source.as_ref().unwrap();
    assert_eq!(chars(&source.citation), MAX_BOX_CHARS);
    assert_eq!(name(&source.source_type), MAX_META_CHARS);
    assert_eq!(source.link, None);
}

#[test]
fn normal_files_open_unchanged() {
    let mut d = Document::new("Een gewone kaart");
    let root = d.root;
    d.set_link(root, Some("https://example.org".into()))
        .unwrap();
    d.add_comment(root, "Ann", "Fine.", None).unwrap();
    d.boxes[&root].text = "x".repeat(MAX_BOX_CHARS);
    d.extra.insert("future".into(), "y".repeat(1000).into());
    let pkg = open_doc(&d, &[("future/x.bin", b"data")]);
    assert!(!pkg.cut_on_open);
    assert_eq!(pkg.doc, d);
    assert_eq!(pkg.other_entries.len(), 1);
}

#[test]
fn oversized_unknown_fields_are_dropped() {
    let mut d = Document::new("c");
    let root = d.root;
    let big = serde_json::Value::from("x".repeat(MAX_EXTRA_BYTES));
    d.extra.insert("future".into(), big.clone());
    d.boxes[&root].extra.insert("future".into(), 1.into());
    let pkg = open_doc(&d, &[]);
    assert!(pkg.cut_on_open);
    assert!(pkg.doc.extra.is_empty());
    assert!(pkg.doc.boxes[&root].extra.is_empty(), "all or nothing");

    // Many small fields on boxes and cards add up too.
    let mut d = Document::new("c");
    let (card, p) = d
        .add_card(crate::Parent::Box(d.root), crate::CardKind::Reason, "p")
        .unwrap();
    let half = serde_json::Value::from("x".repeat(MAX_EXTRA_BYTES / 2));
    d.boxes[&p].extra.insert("a".into(), half.clone());
    d.cards[&card].extra.insert("b".into(), half);
    let pkg = open_doc(&d, &[]);
    assert!(pkg.cut_on_open);
    assert!(pkg.doc.boxes[&p].extra.is_empty() && pkg.doc.cards[&card].extra.is_empty());
}

#[test]
fn unsafe_entry_names_are_dropped() {
    let d = Document::new("c");
    let pkg = open_doc(
        &d,
        &[
            ("../../evil.txt", b"x"),
            ("/abs.txt", b"x"),
            ("C:\\x.txt", b"x"),
            ("a\\b.txt", b"x"),
            ("ok/../../up.txt", b"x"),
            ("ok/fine.bin", b"kept"),
        ],
    );
    let names: Vec<&str> = pkg.other_entries.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["ok/fine.bin"]);
    assert!(pkg.cut_on_open);
}

#[test]
fn asset_keys_must_be_a_hash_and_an_image_extension() {
    let bytes: &[u8] = b"img";
    let good = asset_key("png", bytes);
    let sneaky = asset_key("png/../../evil", bytes);
    let odd = asset_key("exe", bytes);
    let pkg = open_doc(
        &Document::new("c"),
        &[
            (&format!("images/{sneaky}"), bytes),
            (&format!("images/{odd}"), bytes),
            (&format!("images/{good}"), bytes),
        ],
    );
    assert_eq!(pkg.assets.len(), 1);
    assert!(pkg.assets.get(&good).is_some());
    for ext in ["png", "jpg", "jpeg", "webp"] {
        let key = asset_key(ext, bytes);
        let pkg = open_doc(&Document::new("c"), &[(&format!("images/{key}"), bytes)]);
        assert!(pkg.assets.get(&key).is_some(), "{ext}");
    }
}

#[test]
fn unknown_entries_are_shared_not_copied() {
    let pkg = open_doc(&Document::new("c"), &[("future/x.bin", b"data")]);
    let copy = pkg.clone();
    assert!(Arc::ptr_eq(
        &pkg.other_entries[0].1,
        &copy.other_entries[0].1
    ));
}

#[test]
fn document_limit_is_modest() {
    let limit = MAX_DOCUMENT_BYTES;
    assert!(limit <= 64 << 20, "{limit}");
}
