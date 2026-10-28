use super::*;
use crate::model::{Comment, SourceType};

/// root ← card A [a1, a2]; a1 ← card C [c1] and a note; a2 ← a source.
fn sample() -> (Document, CardId, NodeId, NodeId) {
    let mut d = Document::new("Claim");
    let root = Parent::Box(d.root);
    let (ca, a1) = d.add_card(root, CardKind::Reason, "a1").unwrap();
    let a2 = d.add_premise(ca, None, "a2").unwrap();
    d.add_card(Parent::Box(a1), CardKind::Objection, "c1")
        .unwrap();
    d.add_box(Parent::Box(a1), BoxKind::Note, "note").unwrap();
    let source = BoxKind::Source {
        source_type: SourceType::Web,
    };
    d.add_box(Parent::Box(a2), source, "src").unwrap();
    (d, ca, a1, a2)
}

#[test]
fn copy_takes_everything_under_the_item() {
    let (d, ca, ..) = sample();
    let f = d.copy_fragment(Item::Card(ca)).unwrap();
    assert_eq!(f.cards.len(), 2);
    assert_eq!(f.boxes.len(), 5, "a1, a2, c1, note, src");
    assert_eq!(f.premise_of, None);
    assert_eq!(f.plain_text(), "a1\n  note\n  c1\na2\n  src\n");
    assert_eq!(d.copy_fragment(Item::Box(d.root)), None, "not the root");
}

#[test]
fn paste_gives_fresh_ids_and_keeps_the_original() {
    let (mut d, ca, ..) = sample();
    let f = d.copy_fragment(Item::Card(ca)).unwrap();
    let (boxes, cards) = (d.boxes.len(), d.cards.len());
    let root = Parent::Box(d.root);
    let new = d.paste_fragment(&f, root).unwrap();
    let again = d.paste_fragment(&f, root).unwrap();
    assert_ne!(new, again);
    assert_eq!(d.boxes.len(), boxes + 10);
    assert_eq!(d.cards.len(), cards + 4);
    assert_eq!(d.boxes[&d.root].children.last(), Some(&again));
    assert_eq!(d.copy_fragment(new).unwrap().plain_text(), f.plain_text());
    assert_eq!(d.validate(), Ok(()));
}

#[test]
fn a_premise_pastes_as_the_premise_of_a_new_card_of_its_kind() {
    let (mut d, _, a1, _) = sample();
    let f = d.copy_fragment(Item::Box(a1)).unwrap();
    assert_eq!(f.premise_of, Some(CardKind::Reason));
    let Item::Card(c) = d.paste_fragment(&f, Parent::Parking).unwrap() else {
        panic!("a card")
    };
    let card = &d.cards[&c];
    assert_eq!(card.kind, CardKind::Reason);
    assert_eq!(d.boxes[&card.premises[0]].text, "a1");
    assert_eq!(d.boxes[&card.premises[0]].children.len(), 2);
    assert_eq!(d.validate(), Ok(()));
}

#[test]
fn paste_follows_the_structure_rules() {
    let (mut d, ca, a1, _) = sample();
    let note = d.boxes[&a1].children[1];
    let card = d.copy_fragment(Item::Card(ca)).unwrap();
    let Item::Box(note) = note else {
        panic!("a box")
    };
    assert_eq!(
        d.paste_fragment(&card, Parent::Box(note)),
        Err(EditError::NotAllowed),
        "a note takes nothing"
    );
    let note = d.copy_fragment(Item::Box(note)).unwrap();
    assert_eq!(
        d.paste_fragment(&note, Parent::Card(ca)),
        Err(EditError::NotAllowed),
        "a card's whole-card children are cards"
    );
}

#[test]
fn comments_get_new_ids() {
    let (mut d, _, a1, _) = sample();
    d.add_comment(a1, "Ann", "good", None).unwrap();
    let f = d.copy_fragment(Item::Box(a1)).unwrap();
    let Item::Card(c) = d.paste_fragment(&f, Parent::Parking).unwrap() else {
        panic!("a card")
    };
    let pasted = d.cards[&c].premises[0];
    let old: &Comment = &d.boxes[&a1].comments[0];
    let new = &d.boxes[&pasted].comments[0];
    assert_eq!(new.text, old.text);
    assert_ne!(new.id, old.id);
}

/// A chain `depth` reasons deep under the root: each premise has one
/// reason under it (built directly: an edit checks the whole document in
/// debug builds). Returns the top card.
fn chain(depth: usize) -> (Document, CardId) {
    let mut d = Document::new("Claim");
    let mut holder = d.root;
    let mut top = None;
    for i in 0..depth {
        let (card, premise) = (CardId::new(), NodeId::new());
        let text = format!("level {i}");
        (d.boxes).insert(premise, BoxNode::new(premise, BoxKind::Premise, &text));
        (d.cards).insert(card, Card::new(card, CardKind::Reason, vec![premise]));
        d.boxes[&holder].children.push(Item::Card(card));
        holder = premise;
        top.get_or_insert(card);
    }
    d.validate().unwrap();
    (d, top.unwrap())
}

/// Copy, its clipboard text and paste walk the tree without recursion,
/// so a deep chain (from a crafted file or Argdown) fits the main
/// thread's 1 MiB stack; the indent stops growing at [`MAX_INDENT`]
/// levels, so the text grows with the box count, not its square.
#[test]
fn a_deep_chain_copies_and_pastes_on_a_small_stack() {
    const DEPTH: usize = 50_000;
    let (mut d, top) = chain(DEPTH);
    let run = std::thread::Builder::new()
        .stack_size(1 << 20)
        .spawn(move || {
            let f = d.copy_fragment(Item::Card(top)).unwrap();
            let text = f.plain_text();
            d.paste_fragment(&f, Parent::Parking).unwrap();
            (text, d.boxes.len())
        })
        .unwrap();
    let (text, boxes) = run.join().expect("no stack overflow");
    assert_eq!(boxes, 1 + 2 * DEPTH);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), DEPTH);
    assert_eq!(lines[1], "  level 1");
    let deepest = format!("{}level {}", "  ".repeat(MAX_INDENT), DEPTH - 1);
    assert_eq!(lines[DEPTH - 1], deepest);
    assert!(text.len() < DEPTH * (2 * MAX_INDENT + 16));
}
