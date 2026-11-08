use super::*;
use crate::NewLoose;
use crate::ids::Parent;
use crate::model::{BoxKind, CardKind, SourceType};

const OPTS: FindOptions = FindOptions {
    match_case: false,
    whole_word: false,
};

fn texts(doc: &Document, hits: &[FindHit]) -> Vec<String> {
    hits.iter()
        .map(|h| doc.boxes[&h.node].text[h.range.clone()].to_owned())
        .collect()
}

#[test]
fn finds_in_all_box_kinds_in_outline_order() {
    let mut d = Document::new("x claim");
    let root = d.root;
    let (_, p1) = d
        .add_card(Parent::Box(root), CardKind::Reason, "x premise one")
        .unwrap();
    let source = BoxKind::Source {
        source_type: SourceType::Publication,
    };
    d.add_box(Parent::Box(p1), source, "x source").unwrap();
    d.add_box(Parent::Box(p1), BoxKind::Note, "x note").unwrap();
    d.add_card(Parent::Box(root), CardKind::Objection, "x premise two")
        .unwrap();
    d.add_loose(NewLoose::Category, "x category");
    d.add_loose(NewLoose::Note, "parked note x");

    let hits = find_all(&d, "X", OPTS);
    let boxes: Vec<String> = hits.iter().map(|h| d.boxes[&h.node].text.clone()).collect();
    assert_eq!(
        boxes,
        [
            "x claim",
            "x premise one",
            "x source",
            "x note",
            "x premise two",
            "x category",
            "parked note x",
        ]
    );
    assert_eq!(search_order(&d).len(), d.boxes.len());
    // Within a box, text order.
    let mut d = Document::new("aXbxc");
    let hits = find_all(&d, "x", OPTS);
    assert_eq!(
        hits.iter().map(|h| h.range.clone()).collect::<Vec<_>>(),
        [1..2, 3..4]
    );
    assert!(
        find_all(&d, "", OPTS).is_empty(),
        "an empty query finds nothing"
    );
    d.set_text(d.root, "").unwrap();
    assert!(find_all(&d, "x", OPTS).is_empty());
}

#[test]
fn match_case_and_whole_word() {
    let d = Document::new("Cat cat concat cat_x CAT, cat.");
    let n = |opts| find_all(&d, "cat", opts).len();
    assert_eq!(n(OPTS), 6);
    let case = FindOptions {
        match_case: true,
        whole_word: false,
    };
    assert_eq!(n(case), 4);
    let word = FindOptions {
        match_case: false,
        whole_word: true,
    };
    let hits = find_all(&d, "cat", word);
    assert_eq!(texts(&d, &hits), ["Cat", "cat", "CAT", "cat"]);
    let both = FindOptions {
        match_case: true,
        whole_word: true,
    };
    assert_eq!(n(both), 2);
    // A failed whole-word hit does not hide a later one that overlaps it.
    let d = Document::new("aaa aa");
    assert_eq!(find_all(&d, "aa", word).len(), 1);
    assert_eq!(find_all(&d, "aa", word)[0].range, 4..6);
}

#[test]
fn case_fold_with_length_change_is_safe() {
    let d = Document::new("İstanbul STRASSE straße");
    let text = &d.boxes[&d.root].text;
    let hits = find_all(&d, "straße", OPTS);
    assert_eq!(
        texts(&d, &hits),
        ["straße"],
        "STRASSE does not fold to straße"
    );
    for query in ["i̇stanbul", "İSTANBUL", "ß", "e", "İ"] {
        for h in find_all(&d, query, OPTS) {
            assert!(text.is_char_boundary(h.range.start), "{query}");
            assert!(text.is_char_boundary(h.range.end), "{query}");
        }
    }
    assert_eq!(texts(&d, &find_all(&d, "İSTANBUL", OPTS)), ["İstanbul"]);
}

#[test]
fn replacement_containing_query_terminates() {
    let mut d = Document::new("banana");
    assert_eq!(d.replace_all("a", "aa", OPTS).count, 3);
    assert_eq!(d.boxes[&d.root].text, "baanaanaa");
}

#[test]
fn replace_all_with_empty_string() {
    let mut d = Document::new("a-b-c");
    let (_, p) = d
        .add_card(Parent::Box(d.root), CardKind::Reason, "-x-")
        .unwrap();
    assert_eq!(d.replace_all("-", "", OPTS).count, 4);
    assert_eq!(d.boxes[&d.root].text, "abc");
    assert_eq!(d.boxes[&p].text, "x");
    assert_eq!(d.replace_all("-", "", OPTS).count, 0, "nothing left");
    assert_eq!(
        d.replace_all("", "y", OPTS).count,
        0,
        "an empty query replaces nothing"
    );
}

#[test]
fn replace_range_checks_bounds_and_boundaries() {
    let mut d = Document::new("straße");
    let root = d.root;
    assert_eq!(d.replace_range(root, 4..5, "s"), Err(EditError::NotFound));
    assert_eq!(d.replace_range(root, 3..99, ""), Err(EditError::NotFound));
    #[allow(clippy::reversed_empty_ranges)]
    let reversed = 3..2;
    assert_eq!(
        d.replace_range(root, reversed, ""),
        Err(EditError::NotFound)
    );
    assert_eq!(
        d.replace_range(NodeId::new(), 0..1, ""),
        Err(EditError::NotFound)
    );
    assert_eq!(d.boxes[&root].text, "straße");
    d.replace_range(root, 4..6, "ss").unwrap();
    assert_eq!(d.boxes[&root].text, "strasse");
}

#[test]
fn replacements_never_leave_a_box_past_its_cap() {
    use crate::MAX_BOX_CHARS;
    let chars = |d: &Document| d.boxes[&d.root].text.chars().count();
    let long = "é".repeat(1_000);
    let mut d = Document::new(&"a ".repeat(100));
    assert_eq!(
        d.replace_all("a", &long, OPTS),
        Replaced {
            count: 100,
            cut: true
        }
    );
    assert_eq!(chars(&d), MAX_BOX_CHARS);
    assert!(d.boxes[&d.root].text.starts_with(&format!("{long} {long}")));

    let mut d = Document::new("ab");
    assert_eq!(d.replace_range(d.root, 0..1, &long), Ok(false));
    let huge = "x".repeat(MAX_BOX_CHARS);
    assert_eq!(d.replace_range(d.root, 0..2, &huge), Ok(true), "one é");
    assert_eq!(chars(&d), MAX_BOX_CHARS);

    // The review's case: 50,000 hits of a 100,000-character replacement
    // took about 5 GB, replaced right to left; now one pass, stopped early.
    let mut d = Document::new(&"a".repeat(MAX_BOX_CHARS));
    let start = std::time::Instant::now();
    let done = d.replace_all("a", &"x".repeat(100_000), OPTS);
    assert_eq!(
        done,
        Replaced {
            count: MAX_BOX_CHARS,
            cut: true
        }
    );
    assert_eq!(d.boxes[&d.root].text, "x".repeat(MAX_BOX_CHARS));
    assert!(start.elapsed().as_secs() < 2, "{:?}", start.elapsed());
}
