use super::*;
use crate::ids::CardId;
use crate::model::{MapKind, SourceType};

pub(super) fn publication() -> BoxKind {
    BoxKind::Source {
        source_type: SourceType::Publication,
    }
}

#[test]
fn root_cannot_mix_cards_and_categories() {
    let mut d = Document::new("C");
    d.add_card(Parent::Box(d.root), CardKind::Reason, "R")
        .unwrap();
    assert_eq!(
        d.add_box(Parent::Box(d.root), BoxKind::Category, "X"),
        Err(EditError::NotAllowed)
    );
    assert!(d.add_box(Parent::Box(d.root), BoxKind::Note, "n").is_ok());
    assert_eq!(d.map_kind(), MapKind::Reasoning);
}

#[test]
fn card_targets_premise_or_whole_card() {
    let mut d = Document::new("C");
    let (card, p) = d
        .add_card(Parent::Box(d.root), CardKind::Reason, "P1")
        .unwrap();
    d.add_premise(card, None, "P2").unwrap();
    let (on_premise, _) = d
        .add_card(Parent::Box(p), CardKind::Objection, "against P1")
        .unwrap();
    let (on_card, _) = d
        .add_card(Parent::Card(card), CardKind::Objection, "link fails")
        .unwrap();
    assert_eq!(d.owner_of(Item::Card(on_premise)), Some(Parent::Box(p)));
    assert_eq!(d.owner_of(Item::Card(on_card)), Some(Parent::Card(card)));
    assert_eq!(d.cards[&card].premises.len(), 2);
    assert!(d.add_box(Parent::Card(card), publication(), "s").is_err());
}

#[test]
fn add_premise_respects_index_and_missing_card() {
    let mut d = Document::new("C");
    let (card, p1) = d
        .add_card(Parent::Box(d.root), CardKind::Reason, "P1")
        .unwrap();
    let p0 = d.add_premise(card, Some(0), "P0").unwrap();
    assert_eq!(d.cards[&card].premises, vec![p0, p1]);
    assert_eq!(
        d.add_premise(CardId::new(), None, "x"),
        Err(EditError::NotFound)
    );
}

#[test]
fn add_box_rejects_claim_and_premise_and_missing_parent() {
    let mut d = Document::new("C");
    assert_eq!(
        d.add_box(Parent::Box(d.root), BoxKind::Claim, "x"),
        Err(EditError::NotAllowed)
    );
    assert_eq!(
        d.add_box(Parent::Box(d.root), BoxKind::Premise, "x"),
        Err(EditError::NotAllowed)
    );
    assert_eq!(
        d.add_box(Parent::Box(NodeId::new()), BoxKind::Note, "x"),
        Err(EditError::NotFound)
    );
}

#[test]
fn parking_accepts_and_set_text_updates() {
    let mut d = Document::new("C");
    let (_, p) = d
        .add_card(Parent::Parking, CardKind::Objection, "loose")
        .unwrap();
    assert_eq!(d.parking.len(), 1);
    d.meta.modified = "old".into();
    d.set_text(p, "changed").unwrap();
    assert_eq!(d.boxes[&p].text, "changed");
    assert_ne!(d.meta.modified, "old");
    assert_eq!(d.set_text(NodeId::new(), "x"), Err(EditError::NotFound));
}

pub(super) fn card_under_root(d: &mut Document, text: &str) -> (CardId, NodeId) {
    d.add_card(Parent::Box(d.root), CardKind::Reason, text)
        .unwrap()
}

#[test]
fn remove_last_premise_removes_card() {
    let mut d = Document::new("C");
    let (card, p) = card_under_root(&mut d, "P");
    let (sub, _) = d
        .add_card(Parent::Box(p), CardKind::Objection, "x")
        .unwrap();
    d.remove_item(Item::Box(p), RemoveMode::KeepChildren)
        .unwrap();
    assert!(!d.cards.contains_key(&card));
    assert!(!d.boxes.contains_key(&p));
    assert!(d.boxes[&d.root].children.is_empty());
    assert_eq!(d.parking, vec![Item::Card(sub)]);
}

#[test]
fn remove_non_last_premise_keeps_card() {
    let mut d = Document::new("C");
    let (card, p1) = card_under_root(&mut d, "P1");
    let p2 = d.add_premise(card, None, "P2").unwrap();
    d.remove_item(Item::Box(p1), RemoveMode::Branch).unwrap();
    assert_eq!(d.cards[&card].premises, vec![p2]);
    assert!(!d.boxes.contains_key(&p1));
}

#[test]
fn remove_branch_leaves_no_orphans() {
    let mut d = Document::new("C");
    let (card, p) = card_under_root(&mut d, "P");
    d.add_box(Parent::Box(p), publication(), "s").unwrap();
    d.add_card(Parent::Card(card), CardKind::Objection, "x")
        .unwrap();
    d.remove_item(Item::Card(card), RemoveMode::Branch).unwrap();
    assert_eq!(d.boxes.len(), 1);
    assert!(d.cards.is_empty());
    assert!(d.boxes[&d.root].children.is_empty());
}

#[test]
fn remove_keep_children_moves_to_parking() {
    let mut d = Document::new("C");
    let (card, p) = card_under_root(&mut d, "P");
    let (sub, _) = d.add_card(Parent::Box(p), CardKind::Reason, "Q").unwrap();
    let (attacker, _) = d
        .add_card(Parent::Card(card), CardKind::Objection, "A")
        .unwrap();
    d.remove_item(Item::Card(card), RemoveMode::KeepChildren)
        .unwrap();
    assert_eq!(d.parking, vec![Item::Card(sub), Item::Card(attacker)]);
    assert!(!d.cards.contains_key(&card));
    assert!(!d.boxes.contains_key(&p));
}

#[test]
fn remove_root_or_missing_is_refused() {
    let mut d = Document::new("C");
    assert_eq!(
        d.remove_item(Item::Box(d.root), RemoveMode::Branch),
        Err(EditError::NotAllowed)
    );
    assert_eq!(
        d.remove_item(Item::Card(CardId::new()), RemoveMode::Branch),
        Err(EditError::NotFound)
    );
}

#[test]
fn connect_parking_card_to_premise() {
    let mut d = Document::new("C");
    let (_, p) = card_under_root(&mut d, "P");
    let loose = d.add_loose(NewLoose::Card(CardKind::Objection), "loose");
    assert_eq!(d.parking, vec![loose]);
    d.move_item(loose, Parent::Box(p), None).unwrap();
    assert!(d.parking.is_empty());
    assert_eq!(d.boxes[&p].children, vec![loose]);
    assert_eq!(d.owner_of(loose), Some(Parent::Box(p)));
}

#[test]
fn move_checks_structure_rules() {
    let mut d = Document::new("C");
    let cat = d
        .add_box(Parent::Box(d.root), BoxKind::Category, "X")
        .unwrap();
    let note = d.add_loose(NewLoose::Note, "n");
    let Item::Box(note_id) = note else {
        unreachable!()
    };
    let other = d.add_loose(NewLoose::Note, "m");
    assert_eq!(
        d.move_item(other, Parent::Box(note_id), None),
        Err(EditError::NotAllowed)
    );
    let card = d.add_loose(NewLoose::Card(CardKind::Reason), "r");
    assert_eq!(
        d.move_item(card, Parent::Box(cat), None),
        Err(EditError::NotAllowed)
    );
    assert_eq!(
        d.move_item(card, Parent::Box(d.root), None),
        Err(EditError::NotAllowed)
    );
    assert_eq!(
        d.move_item(Item::Box(d.root), Parent::Parking, None),
        Err(EditError::NotAllowed)
    );
}

#[test]
fn move_into_descendant_is_cycle() {
    let mut d = Document::new("C");
    let (a, pa) = card_under_root(&mut d, "A");
    let (_, pb) = d.add_card(Parent::Box(pa), CardKind::Reason, "B").unwrap();
    assert_eq!(
        d.move_item(Item::Card(a), Parent::Box(pb), None),
        Err(EditError::Cycle)
    );
    assert_eq!(
        d.move_item(Item::Card(a), Parent::Card(a), None),
        Err(EditError::Cycle)
    );
}

#[test]
fn detach_to_parking_and_reorder() {
    let mut d = Document::new("C");
    let (a, _) = card_under_root(&mut d, "A");
    let (b, _) = card_under_root(&mut d, "B");
    let (c, _) = card_under_root(&mut d, "C");
    d.move_item(Item::Card(c), Parent::Box(d.root), Some(0))
        .unwrap();
    assert_eq!(
        d.boxes[&d.root].children,
        vec![Item::Card(c), Item::Card(a), Item::Card(b)]
    );
    d.move_item(Item::Card(a), Parent::Parking, None).unwrap();
    assert_eq!(d.parking, vec![Item::Card(a)]);
    assert_eq!(
        d.boxes[&d.root].children,
        vec![Item::Card(c), Item::Card(b)]
    );
}

#[test]
fn move_premise_between_cards() {
    let mut d = Document::new("C");
    let (a, pa) = card_under_root(&mut d, "A");
    let (b, _) = card_under_root(&mut d, "B");
    let pa2 = d.add_premise(a, None, "A2").unwrap();
    d.move_item(Item::Box(pa2), Parent::Card(b), Some(0))
        .unwrap();
    assert_eq!(d.cards[&a].premises, vec![pa]);
    assert_eq!(d.cards[&b].premises[0], pa2);
    let (sub, _) = d
        .add_card(Parent::Box(pa), CardKind::Objection, "s")
        .unwrap();
    // A premise cannot move under its own subtree.
    assert_eq!(
        d.move_item(Item::Box(pa), Parent::Card(sub), None),
        Err(EditError::Cycle)
    );
    // A premise only moves between cards.
    assert_eq!(
        d.move_item(Item::Box(pa), Parent::Parking, None),
        Err(EditError::NotAllowed)
    );
    // Moving the last premise away removes the emptied card.
    d.move_item(Item::Box(pa), Parent::Card(b), None).unwrap();
    assert!(!d.cards.contains_key(&a));
    assert!(!d.boxes[&d.root].children.contains(&Item::Card(a)));
    assert_eq!(d.cards[&b].premises.len(), 3);
    assert_eq!(d.owner_of(Item::Card(sub)), Some(Parent::Box(pa)));
}

#[test]
fn descendants_is_iterative_and_complete() {
    let mut d = Document::new("C");
    let (card, p) = card_under_root(&mut d, "P");
    let (att, _) = d
        .add_card(Parent::Card(card), CardKind::Objection, "A")
        .unwrap();
    let mut under = p;
    let mut expected = 3; // premise, attacker, attacker's premise
    for _ in 0..500 {
        let (_, np) = d
            .add_card(Parent::Box(under), CardKind::Reason, "n")
            .unwrap();
        under = np;
        expected += 2;
    }
    let all = d.descendants(Item::Card(card));
    assert_eq!(all.len(), expected);
    assert!(all.contains(&Item::Card(att)));
    assert!(!all.contains(&Item::Card(card)));
    d.remove_item(Item::Card(card), RemoveMode::Branch).unwrap();
    assert_eq!(d.boxes.len(), 1);
}

// `check_invariants` exists in debug builds only.
#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "invariant")]
fn invariants_catch_dangling_ids() {
    let mut d = Document::new("C");
    let root = d.root;
    d.boxes
        .get_mut(&root)
        .unwrap()
        .children
        .push(Item::Card(CardId::new()));
    d.check_invariants();
}
