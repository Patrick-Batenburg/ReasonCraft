//! The essay in the file format: paragraphs whose item is missing.

use reasoncraft_core::file::{open, save};
use reasoncraft_core::{Document, EssaySlot, UnplacedNote};

#[test]
fn orphan_paragraphs_become_unplaced_on_open() {
    let mut d = Document::new("C");
    d.set_essay_text(EssaySlot::Intro, "Intro text").unwrap();
    // A hand-built file: paragraphs keyed by items the map does not have,
    // and a key this build does not know.
    let card = "card:00000000-0000-4000-8000-000000000001";
    let boxed = "box:00000000-0000-4000-8000-000000000002";
    let paragraphs = &mut d.essay.paragraphs;
    paragraphs.insert(card.into(), "Orphan card text".into());
    paragraphs.insert(boxed.into(), "Orphan box text".into());
    paragraphs.insert("figure:7".into(), "Kept for newer builds".into());
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = (dir.path().join("a.rcraft"), dir.path().join("b.rcraft"));
    save(&d, &a).unwrap();

    let opened = open(&a).unwrap();
    let unplaced = vec![
        UnplacedNote::new(String::new(), "Orphan card text".to_owned()),
        UnplacedNote::new(String::new(), "Orphan box text".to_owned()),
    ];
    assert_eq!(opened.essay.unplaced, unplaced);
    assert_eq!(opened.essay.text(EssaySlot::Intro), Some("Intro text"));
    assert!(!opened.essay.paragraphs.contains_key(card));
    assert!(!opened.essay.paragraphs.contains_key(boxed));
    assert!(opened.essay.paragraphs.contains_key("figure:7"));

    // A second save and open keeps them there, once.
    save(&opened, &b).unwrap();
    let again = open(&b).unwrap();
    assert_eq!(again.essay.unplaced, unplaced);
    assert_eq!(again.essay.paragraphs, opened.essay.paragraphs);
}
