//! The essay as Markdown and plain text, its file round trip and the
//! caps on texts read from files.

use super::*;
use crate::ids::Parent;
use crate::model::{BoxKind, CardKind, CardStrength, SourceType};

fn source() -> BoxKind {
    BoxKind::Source {
        source_type: SourceType::Publication,
    }
}

fn essay_doc(lang: &str) -> Document {
    let mut d = Document::new("Robots *help*");
    d.meta.language = lang.into();
    let root = Parent::Box(d.root);
    let (weak, _) = d.add_card(root, CardKind::Reason, "Weak one").unwrap();
    d.set_card_strength(weak, Some(CardStrength::Weak)).unwrap();
    let (strong, p) = d.add_card(root, CardKind::Reason, "Strong one").unwrap();
    d.set_card_strength(strong, Some(CardStrength::Strong))
        .unwrap();
    let s = d.add_box(Parent::Box(p), source(), "A study").unwrap();
    d.set_essay_text(EssaySlot::Intro, "\n  \nFirst | line\n\n- second & ~x~\n\n")
        .unwrap();
    d.set_essay_text(EssaySlot::Item(Item::Box(s)), "Evidence para.")
        .unwrap();
    d
}

#[test]
fn essay_markdown_nl_headings() {
    let d = essay_doc("nl");
    let md = essay_markdown(&d, "nl");
    let want = "# Robots \\*help\\*\n\n\
## Inleiding\n\n\
First \\| line\n\n\
\\- second \\& \\~x\\~\n\n\
## Deel 1: Strong one\n\n\
*Strong one*\n\n\
Evidence para.\n\n\
## Deel 2: Weak one\n\n\
*Weak one*\n\n\
## Conclusie\n\n\
*Robots \\*help\\**";
    assert_eq!(md, want);
    let en = essay_markdown(&d, "en");
    assert!(en.contains("## Introduction\n"));
    assert!(en.contains("## Section 1: Strong one\n"));
    assert!(en.ends_with("## Conclusion\n\n*Robots \\*help\\**"));
}

#[test]
fn essay_plain_text_has_headings_and_hints() {
    let mut d = essay_doc("en");
    d.meta.title = "My essay".into();
    let txt = essay_plain_text(&d, "en");
    let want = "My essay\n\n\
Introduction\n\n\
First | line\n\n\
- second & ~x~\n\n\
Section 1: Strong one\n\n\
[Strong one]\n\n\
Evidence para.\n\n\
Section 2: Weak one\n\n\
[Weak one]\n\n\
Conclusion\n\n\
[Robots *help*]";
    assert_eq!(txt, want);
}

#[test]
fn essay_roundtrip_and_old_files() {
    let mut d = essay_doc("en");
    let json = serde_json::to_string(&Document::new("C")).unwrap();
    assert!(!json.contains("essay"), "an empty essay is not written");
    let old: Document = serde_json::from_str(&json).unwrap();
    assert!(old.essay.is_empty());

    d.essay
        .unplaced
        .push(UnplacedNote::new("label".into(), "text".into()));
    d.essay
        .extra
        .insert("future".into(), serde_json::json!({"x": 1}));
    // A stale key (the item is gone) stays in the file and is ignored.
    let stale = EssaySlot::Item(Item::Box(NodeId::new())).key();
    d.essay.paragraphs.insert(stale.clone(), "stale".into());
    d.essay.paragraphs.insert("bogus".into(), "kept".into());
    let json = serde_json::to_string(&d).unwrap();
    let back: Document = serde_json::from_str(&json).unwrap();
    assert_eq!(back, d);
    assert!(back.validate().is_ok());
    assert_eq!(back.essay.paragraphs[&stale], "stale");
    assert!(!essay_markdown(&back, "en").contains("stale"));
}

#[test]
fn oversized_texts_from_files_are_capped() {
    let mut d = Document::new("C");
    let long: String = "é日a".chars().cycle().take(60_000).collect();
    d.essay.paragraphs.insert("intro".into(), long.clone());
    d.essay
        .unplaced
        .push(UnplacedNote::new("é".repeat(80), long));
    d.cap_essay_texts();
    assert_eq!(d.essay.unplaced[0].label, "é".repeat(LABEL_CHARS));
    assert_eq!(d.essay.paragraphs["intro"].chars().count(), MAX_ESSAY_CHARS);
    assert_eq!(d.essay.unplaced[0].text.chars().count(), MAX_ESSAY_CHARS);
}
