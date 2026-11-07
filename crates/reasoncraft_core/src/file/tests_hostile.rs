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
        extra: Default::default(),
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

    // And fields kept in nested objects: the meta and a comment.
    let mut d = Document::new("c");
    let root = d.root;
    d.add_comment(root, "a", "t", None).unwrap();
    let half = serde_json::Value::from("x".repeat(MAX_EXTRA_BYTES / 2));
    d.meta.extra.insert("a".into(), half.clone());
    d.boxes[&root].comments[0].extra.insert("b".into(), half);
    let pkg = open_doc(&d, &[]);
    assert!(pkg.cut_on_open);
    assert!(pkg.doc.meta.extra.is_empty() && pkg.doc.boxes[&root].comments[0].extra.is_empty());
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

/// All of a document's texts together keep at most
/// [`crate::caps::MAX_DOCUMENT_CHARS`] characters: a file of many full
/// boxes is cut on open (in document order), so laying it out and
/// painting it stay bounded (SEC-R1).
#[test]
fn a_documents_total_text_is_cut_on_open() {
    use crate::caps::MAX_DOCUMENT_CHARS;
    use crate::ids::Parent;
    use crate::model::CardKind;
    let mut d = Document::new("c");
    let full = "x".repeat(MAX_BOX_CHARS);
    let n = MAX_DOCUMENT_CHARS / MAX_BOX_CHARS + 10;
    let mut premises = Vec::new();
    for _ in 0..n {
        premises.push(
            d.add_card(Parent::Box(d.root), CardKind::Reason, &full)
                .unwrap()
                .1,
        );
    }
    d.add_snippet(&"s".repeat(1000));
    let pkg = open_doc(&d, &[]);
    assert!(pkg.cut_on_open);
    let d = &pkg.doc;
    let total: usize = (d.boxes.values().map(|b| chars(&b.text)))
        .chain(d.snippets.iter().map(|s| chars(&s.text)))
        .sum();
    assert_eq!(total, MAX_DOCUMENT_CHARS);
    assert_eq!(
        chars(&d.boxes[&premises[0]].text),
        MAX_BOX_CHARS,
        "kept in order"
    );
    assert_eq!(d.boxes[&premises[n - 1]].text, "", "the last ones are cut");
    // A document under the cap opens untouched.
    let small = open_doc(&Document::new("c"), &[]);
    assert!(!small.cut_on_open);
}

/// A document of millions of tiny JSON values (an unknown field from a
/// "newer version" holding a huge array) is refused before it is parsed,
/// as each value would take far more memory than its bytes (SEC-R3).
#[test]
fn a_flood_of_json_values_is_refused() {
    let mut json = serde_json::to_value(Document::new("c")).unwrap();
    let flood = vec![0u8; MAX_JSON_VALUES + 10];
    json["future"] = serde_json::to_value(flood).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("x.rcraft");
    let mut z = zip::ZipWriter::new(std::fs::File::create(&p).unwrap());
    z.start_file("document.json", zip::write::SimpleFileOptions::default())
        .unwrap();
    std::io::Write::write_all(&mut z, json.to_string().as_bytes()).unwrap();
    z.finish().unwrap();
    assert!(
        matches!(open_package(&p), Err(FileError::Malformed(_))),
        "refused"
    );
    // Strings full of commas and brackets are not values.
    let mut d = Document::new("c");
    let root = d.root;
    d.boxes[&root].text = "[,]{:}".repeat(MAX_BOX_CHARS / 6);
    assert!(!open_doc(&d, &[]).cut_on_open);
}

#[test]
fn branches_nested_too_deep_are_parked_on_open() {
    let (d, _) = crate::fixtures::deep_chain(2 * crate::MAX_DEPTH + 1);
    let pkg = open_doc(&d, &[]);
    assert!(pkg.parked_on_open, "the open says branches were moved");
    assert!(!pkg.cut_on_open, "no text was cut");
    let doc = &pkg.doc;
    doc.validate().unwrap();
    assert_eq!(doc.parking.len(), 1);
    assert_eq!(doc.boxes.len(), d.boxes.len(), "nothing is lost");
    let deepest = doc
        .boxes
        .keys()
        .filter_map(|b| doc.depth_of(crate::Item::Box(*b)));
    assert_eq!(deepest.max(), Some(crate::MAX_DEPTH));
    let (within, _) = crate::fixtures::deep_chain(crate::MAX_DEPTH);
    let pkg = open_doc(&within, &[]);
    assert!(!pkg.parked_on_open);
    assert_eq!(pkg.doc, within);
}
