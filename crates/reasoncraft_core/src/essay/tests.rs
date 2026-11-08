use super::*;
use crate::edit::{EditError, RemoveMode};
use crate::history::History;
use crate::ids::{CardId, NodeId, Parent};
use crate::model::{BoxKind, CardKind, CardStrength, SourceType};

fn source() -> BoxKind {
    BoxKind::Source {
        source_type: SourceType::Publication,
    }
}

/// Five first-level cards with every strength, a reason/objection tie,
/// and a branch below the first.
fn rated() -> (Document, Vec<CardId>) {
    let mut d = Document::new("Claim");
    let root = Parent::Box(d.root);
    let mut ids = Vec::new();
    for (kind, strength, text) in [
        (
            CardKind::Objection,
            Some(CardStrength::Strong),
            "obj strong",
        ),
        (CardKind::Reason, None, "reason unrated"),
        (CardKind::Reason, Some(CardStrength::Nil), "reason nil"),
        (CardKind::Reason, Some(CardStrength::Weak), "reason weak"),
        (
            CardKind::Reason,
            Some(CardStrength::Strong),
            "reason strong",
        ),
        (CardKind::Objection, None, "obj unrated"),
    ] {
        let (c, _) = d.add_card(root, kind, text).unwrap();
        d.set_card_strength(c, strength).unwrap();
        ids.push(c);
    }
    (d, ids)
}

fn section_texts(d: &Document) -> Vec<String> {
    essay_order(d)
        .iter()
        .map(|s| s.elements[0].text.clone())
        .collect()
}

#[test]
fn essay_order_strongest_first() {
    let (d, ids) = rated();
    assert_eq!(
        section_texts(&d),
        [
            "reason strong",
            "obj strong",
            "reason unrated",
            "obj unrated",
            "reason weak",
            "reason nil",
        ]
    );
    let order = essay_order(&d);
    assert_eq!(order[0].item, Item::Card(ids[4]));
    let ranks: Vec<u8> = order.iter().map(|s| s.strength_rank).collect();
    assert_eq!(ranks, [0, 0, 1, 1, 2, 3]);
    let roles: Vec<ElementRole> = order.iter().map(|s| s.elements[0].role).collect();
    use ElementRole::{Counter, Reason};
    assert_eq!(roles, [Reason, Counter, Reason, Counter, Reason, Reason]);
}

/// A card paragraph as an old or hand-edited file may hold it.
fn put_card_text(d: &mut Document, c: CardId, text: &str) {
    let key = EssaySlot::Item(Item::Card(c)).key();
    d.essay.paragraphs.insert(key, text.to_owned());
}

#[test]
fn essay_section_elements_follow_display_order() {
    let mut d = Document::new("Claim");
    let (card, p) = d
        .add_card(Parent::Box(d.root), CardKind::Reason, "P")
        .unwrap();
    let p2 = d.add_premise(card, None, "P2").unwrap();
    let (_, o) = d
        .add_card(Parent::Box(p), CardKind::Objection, "O")
        .unwrap();
    let (_, r) = d
        .add_card(Parent::Box(o), CardKind::Objection, "R")
        .unwrap();
    let (_, sub) = d
        .add_card(Parent::Box(p2), CardKind::Reason, "Sub")
        .unwrap();
    let s = d.add_box(Parent::Box(p), source(), "S").unwrap();
    d.add_box(Parent::Box(p), BoxKind::Note, "private").unwrap();
    let (_, w) = d
        .add_card(Parent::Card(card), CardKind::Objection, "W")
        .unwrap();
    let order = essay_order(&d);
    assert_eq!(order.len(), 1);
    let got: Vec<(EssaySlot, ElementRole, &str, usize)> = order[0]
        .elements
        .iter()
        .map(|e| (e.slot, e.role, e.text.as_str(), e.depth))
        .collect();
    let slot = |b: NodeId| EssaySlot::Item(Item::Box(b));
    assert_eq!(
        got,
        [
            (slot(p), ElementRole::Reason, "P", 0),
            (slot(p2), ElementRole::Reason, "P2", 0),
            (slot(s), ElementRole::Evidence, "S", 1),
            (slot(o), ElementRole::Counter, "O", 1),
            (slot(r), ElementRole::Rebuttal, "R", 2),
            (slot(sub), ElementRole::Reason, "Sub", 1),
            (slot(w), ElementRole::Counter, "W", 1),
        ]
    );
}

#[test]
fn slot_keys_roundtrip() {
    let b = NodeId::new();
    let c = CardId::new();
    for slot in [
        EssaySlot::Intro,
        EssaySlot::Conclusion,
        EssaySlot::Item(Item::Box(b)),
        EssaySlot::Item(Item::Card(c)),
    ] {
        assert_eq!(EssaySlot::from_key(&slot.key()), Some(slot));
        let json = serde_json::to_string(&slot).unwrap();
        assert_eq!(json, format!("\"{}\"", slot.key()));
        assert_eq!(serde_json::from_str::<EssaySlot>(&json).unwrap(), slot);
    }
    assert_eq!(EssaySlot::Item(Item::Box(b)).key(), format!("box:{}", b.0));
    assert_eq!(
        EssaySlot::Item(Item::Card(c)).key(),
        format!("card:{}", c.0)
    );
    for bad in ["", "box:", "box:nope", "item:x", "Intro"] {
        assert_eq!(EssaySlot::from_key(bad), None, "{bad}");
    }
}

#[test]
fn set_essay_text_caps_removes_and_skips_no_ops() {
    let mut d = Document::new("C");
    d.meta.modified = "then".into();
    d.set_essay_text(EssaySlot::Intro, "").unwrap();
    assert_eq!(d.meta.modified, "then", "removing nothing is a no-op");
    d.set_essay_text(EssaySlot::Intro, "hello").unwrap();
    assert_eq!(d.essay.paragraphs["intro"], "hello");
    d.meta.modified = "then".into();
    d.set_essay_text(EssaySlot::Intro, "hello").unwrap();
    assert_eq!(d.meta.modified, "then", "same text is a no-op");
    d.set_essay_text(EssaySlot::Intro, "").unwrap();
    assert!(d.essay.paragraphs.is_empty());
    let long: String = "é日a".chars().cycle().take(60_000).collect();
    d.set_essay_text(EssaySlot::Conclusion, &long).unwrap();
    assert_eq!(
        d.essay.paragraphs["conclusion"].chars().count(),
        MAX_ESSAY_CHARS
    );
    let missing = EssaySlot::Item(Item::Box(NodeId::new()));
    assert_eq!(d.set_essay_text(missing, "x"), Err(EditError::NotFound));
}

#[test]
fn card_slots_are_not_writable() {
    let mut d = Document::new("C");
    let (card, _) = d
        .add_card(Parent::Box(d.root), CardKind::Reason, "P")
        .unwrap();
    let slot = EssaySlot::Item(Item::Card(card));
    assert_eq!(d.set_essay_text(slot, "x"), Err(EditError::NotAllowed));
    assert_eq!(d.set_essay_text(slot, ""), Err(EditError::NotAllowed));
    assert!(d.essay.paragraphs.is_empty());
}

#[test]
fn empty_item_text_gives_ellipsis_label() {
    let mut d = Document::new("C");
    let (card, p) = d
        .add_card(Parent::Box(d.root), CardKind::Reason, " \n ")
        .unwrap();
    put_card_text(&mut d, card, "para card");
    d.set_essay_text(EssaySlot::Item(Item::Box(p)), "para P")
        .unwrap();
    d.remove_item(Item::Card(card), RemoveMode::Branch).unwrap();
    assert!(d.essay.unplaced.iter().all(|u| u.label == "\u{2026}"));
    assert_eq!(d.essay.unplaced.len(), 2);
}

#[test]
fn deleting_branch_moves_its_paragraph_to_unplaced() {
    let mut d = Document::new("C");
    let (card, p) = d
        .add_card(
            Parent::Box(d.root),
            CardKind::Reason,
            "Premise text\nsecond line",
        )
        .unwrap();
    let (_, o) = d
        .add_card(Parent::Box(p), CardKind::Objection, &"long ".repeat(30))
        .unwrap();
    d.set_essay_text(EssaySlot::Item(Item::Box(p)), "para P")
        .unwrap();
    d.set_essay_text(EssaySlot::Item(Item::Box(o)), "para O")
        .unwrap();
    put_card_text(&mut d, card, "para card");
    d.set_essay_text(EssaySlot::Intro, "intro").unwrap();
    d.remove_item(Item::Card(card), RemoveMode::Branch).unwrap();
    assert_eq!(d.essay.paragraphs.len(), 1, "only the intro stays");
    let mut got: Vec<(String, String)> = d
        .essay
        .unplaced
        .iter()
        .map(|u| (u.label.clone(), u.text.clone()))
        .collect();
    got.sort();
    let o_label: String = "long ".repeat(30).chars().take(60).collect();
    assert_eq!(o_label.chars().count(), 60);
    let mut want = vec![
        (
            "Premise text second line".to_owned(),
            "para card".to_owned(),
        ),
        ("Premise text second line".to_owned(), "para P".to_owned()),
        (o_label.trim_end().to_owned(), "para O".to_owned()),
    ];
    want.sort();
    assert_eq!(got, want);
    d.remove_unplaced(0).unwrap();
    assert_eq!(d.essay.unplaced.len(), 2);
    assert_eq!(d.remove_unplaced(5), Err(EditError::NotFound));
}

#[test]
fn keep_children_and_moves_salvage_only_removed_items() {
    let mut d = Document::new("C");
    let (card, p) = d
        .add_card(Parent::Box(d.root), CardKind::Reason, "P")
        .unwrap();
    let (_, o) = d
        .add_card(Parent::Box(p), CardKind::Objection, "O")
        .unwrap();
    d.set_essay_text(EssaySlot::Item(Item::Box(p)), "para P")
        .unwrap();
    d.set_essay_text(EssaySlot::Item(Item::Box(o)), "para O")
        .unwrap();
    d.remove_item(Item::Card(card), RemoveMode::KeepChildren)
        .unwrap();
    assert_eq!(
        d.essay.unplaced,
        [UnplacedNote::new("P".into(), "para P".into())]
    );
    assert!(
        d.essay
            .paragraphs
            .contains_key(&EssaySlot::Item(Item::Box(o)).key())
    );

    // Moving a card's only premise away removes the card: its slot is
    // salvaged under the moved premise's text; the premise keeps its own.
    let (a, pa) = d
        .add_card(Parent::Box(d.root), CardKind::Reason, "A")
        .unwrap();
    let (b, _) = d
        .add_card(Parent::Box(d.root), CardKind::Reason, "B")
        .unwrap();
    put_card_text(&mut d, a, "para A");
    d.set_essay_text(EssaySlot::Item(Item::Box(pa)), "para pa")
        .unwrap();
    d.move_item(Item::Box(pa), Parent::Card(b), None).unwrap();
    assert!(!d.cards.contains_key(&a));
    assert_eq!(
        d.essay.unplaced[1],
        UnplacedNote::new("A".into(), "para A".into())
    );
    assert_eq!(
        d.essay.paragraphs[&EssaySlot::Item(Item::Box(pa)).key()],
        "para pa"
    );
}

#[test]
fn undo_restores_paragraph_slot() {
    let mut d = Document::new("C");
    let (card, p) = d
        .add_card(Parent::Box(d.root), CardKind::Reason, "P")
        .unwrap();
    d.set_essay_text(EssaySlot::Item(Item::Box(p)), "para")
        .unwrap();
    let before = d.clone();
    let mut h = History::new(10);
    h.record(d.clone());
    d.remove_item(Item::Card(card), RemoveMode::Branch).unwrap();
    assert_eq!(d.essay.unplaced.len(), 1);
    assert!(h.undo(&mut d));
    assert_eq!(d, before);
    assert_eq!(
        d.essay.paragraphs[&EssaySlot::Item(Item::Box(p)).key()],
        "para"
    );
    assert!(d.essay.unplaced.is_empty());
}
