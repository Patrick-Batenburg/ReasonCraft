//! Test-document builders and a fixed text measurer (feature `test-fixtures`).

use crate::edit::NewLoose;
use crate::ids::{CardId, Item, NodeId, Parent};
use crate::layout::Measure;
use crate::model::{BoxKind, BoxNode, Card, CardKind, Document};

mod compare;
pub use compare::structurally_equal;

/// Measures text as 7 px per character and 14 px per line, wrapping at word
/// boundaries and breaking words longer than the width.
#[derive(Debug, Clone, Copy, Default)]
pub struct FixedMeasure;

impl FixedMeasure {
    pub const CHAR_WIDTH: f32 = 7.0;
    pub const LINE_HEIGHT: f32 = 14.0;
}

impl Measure for FixedMeasure {
    fn text_height(&self, text: &str, width: f32, _kind: &BoxKind) -> f32 {
        let per_line = ((width / Self::CHAR_WIDTH).floor() as usize).max(1);
        let lines: usize = text.split('\n').map(|l| wrapped_lines(l, per_line)).sum();
        lines as f32 * Self::LINE_HEIGHT
    }
}

/// Lines needed for one paragraph at `per_line` characters per line.
fn wrapped_lines(paragraph: &str, per_line: usize) -> usize {
    let mut lines = 1;
    let mut used = 0; // characters on the current line
    for word in paragraph.split_whitespace() {
        let len = word.chars().count();
        let needed = if used == 0 { len } else { used + 1 + len };
        if needed <= per_line {
            used = needed;
            continue;
        }
        if used > 0 {
            lines += 1;
        }
        // Break an over-long word into full lines plus a remainder.
        lines += (len - 1) / per_line;
        used = (len - 1) % per_line + 1;
    }
    lines
}

const PHRASES: [&str; 8] = [
    "Connected sensors detect leaks early",
    "Cheap devices rarely get security updates",
    "Insurers offer discounts for monitored homes",
    "Default passwords are often left unchanged",
    "Remote alerts reach owners within seconds",
    "Cloud outages can disable locks and cameras",
    "A study of 2,000 households found fewer break-ins",
    "Data collected at home can be sold to third parties",
];

fn text(label: &str, i: usize) -> String {
    format!("{label} {i}: {}", PHRASES[i % PHRASES.len()])
}

/// A map whose claim has a chain of `levels` reasons, each under the
/// premise of the one before, built directly (past any edit check, so it
/// may nest deeper than [`crate::MAX_DEPTH`]); and its premises from the
/// top down.
pub fn deep_chain(levels: usize) -> (Document, Vec<NodeId>) {
    let mut d = Document::new("C");
    let mut premises = Vec::with_capacity(levels);
    let mut owner = d.root;
    for i in 0..levels {
        let (card, premise) = (CardId::new(), NodeId::new());
        d.boxes.insert(
            premise,
            BoxNode::new(premise, BoxKind::Premise, &format!("level {i}")),
        );
        d.cards
            .insert(card, Card::new(card, CardKind::Reason, vec![premise]));
        d.boxes[&owner].children.push(Item::Card(card));
        premises.push(premise);
        owner = premise;
    }
    (d, premises)
}

/// A map whose claim has a reason under which a chain of `levels`
/// whole-card objections hangs, each against the card before it, built
/// directly (past any edit check, so it may nest deeper than
/// [`crate::MAX_DEPTH`]).
pub fn deep_objection_chain(levels: usize) -> Document {
    let mut d = Document::new("C");
    let mut owner: Option<CardId> = None;
    for i in 0..=levels {
        let (card, premise) = (CardId::new(), NodeId::new());
        d.boxes.insert(
            premise,
            BoxNode::new(premise, BoxKind::Premise, &format!("level {i}")),
        );
        let kind = match owner {
            Some(_) => CardKind::Objection,
            None => CardKind::Reason,
        };
        d.cards.insert(card, Card::new(card, kind, vec![premise]));
        match owner {
            Some(o) => d.cards[&o].children.push(card),
            None => {
                let root = d.root;
                d.boxes[&root].children.push(Item::Card(card));
            }
        }
        owner = Some(card);
    }
    d
}

/// A deterministic map of about `n_boxes` boxes (root, premises and sources
/// in the tree and the parking area). It has reasons and objections at
/// several depths, rebuttals, 2- and 3-premise cards, whole-card attacks,
/// sources under premises and two parking items. Built only through the
/// public edit operations, so it is always valid.
pub fn sample_iot_like(n_boxes: usize) -> Document {
    let mut d = Document::new("Smart home devices make homes safer");
    let root = Parent::Box(d.root);
    let source = || BoxKind::Source {
        source_type: crate::model::SourceType::Publication,
    };

    // A fixed prologue that guarantees every feature.
    let (a, a1) = d
        .add_card(root, CardKind::Reason, &text("Reason", 0))
        .unwrap();
    d.add_premise(a, None, &text("Co-premise", 1)).unwrap();
    let (b, b1) = d
        .add_card(root, CardKind::Objection, &text("Objection", 2))
        .unwrap();
    d.add_premise(b, None, &text("Co-premise", 3)).unwrap();
    d.add_premise(b, None, &text("Co-premise", 4)).unwrap();
    let (c, c1) = d
        .add_card(Parent::Box(a1), CardKind::Objection, &text("Objection", 5))
        .unwrap();
    let (_, r1) = d
        .add_card(Parent::Box(c1), CardKind::Objection, &text("Rebuttal", 6))
        .unwrap();
    d.add_card(Parent::Card(a), CardKind::Objection, &text("Undercut", 7))
        .unwrap();
    d.add_card(Parent::Card(b), CardKind::Objection, &text("Undercut", 8))
        .unwrap();
    d.add_card(Parent::Card(c), CardKind::Reason, &text("Backing", 9))
        .unwrap();
    d.add_box(Parent::Box(a1), source(), &text("Source", 10))
        .unwrap();
    d.add_box(Parent::Box(b1), source(), &text("Source", 11))
        .unwrap();
    d.add_loose(NewLoose::Card(CardKind::Reason), &text("Loose reason", 12));
    d.add_loose(NewLoose::Note, &text("Loose note", 13));

    // Then grow with a counter-based pattern until big enough.
    let mut premises: Vec<NodeId> = vec![a1, b1, c1, r1];
    let mut cards: Vec<CardId> = vec![a, b, c];
    let mut i = 14;
    while d.boxes.len() < n_boxes {
        let target = Parent::Box(premises[(i * 7) % premises.len()]);
        match i % 7 {
            0 | 1 | 4 => {
                let (card, p) = d
                    .add_card(target, CardKind::Reason, &text("Reason", i))
                    .unwrap();
                cards.push(card);
                premises.push(p);
            }
            2 | 5 => {
                let (card, p) = d
                    .add_card(target, CardKind::Objection, &text("Objection", i))
                    .unwrap();
                cards.push(card);
                premises.push(p);
            }
            3 => {
                d.add_box(target, source(), &text("Source", i)).unwrap();
            }
            _ => {
                let card = cards[(i * 5) % cards.len()];
                if i % 3 == 0 {
                    let (k, p) = d
                        .add_card(
                            Parent::Card(card),
                            CardKind::Objection,
                            &text("Undercut", i),
                        )
                        .unwrap();
                    cards.push(k);
                    premises.push(p);
                } else if d.cards[&card].premises.len() < 3 {
                    premises.push(d.add_premise(card, None, &text("Co-premise", i)).unwrap());
                } else {
                    d.add_box(target, BoxKind::Note, &text("Note", i)).unwrap();
                }
            }
        }
        i += 1;
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_measure_wraps_and_breaks_words() {
        let m = FixedMeasure;
        let k = BoxKind::Premise;
        // 70 px = 10 chars per line.
        assert_eq!(m.text_height("", 70.0, &k), 14.0);
        assert_eq!(m.text_height("aaaa bbbbb", 70.0, &k), 14.0);
        assert_eq!(m.text_height("aaaa bbbbbb", 70.0, &k), 28.0);
        assert_eq!(m.text_height(&"x".repeat(10), 70.0, &k), 14.0);
        assert_eq!(m.text_height(&"x".repeat(25), 70.0, &k), 42.0);
        assert_eq!(m.text_height("ab\ncd", 70.0, &k), 28.0);
    }

    #[test]
    fn sample_is_deterministic_and_sized() {
        let a = sample_iot_like(117);
        let b = sample_iot_like(117);
        assert_eq!(a.boxes.len(), 117);
        assert_eq!(a.cards.len(), b.cards.len());
        let texts = |d: &Document| d.boxes.values().map(|n| n.text.clone()).collect::<Vec<_>>();
        assert_eq!(texts(&a), texts(&b));
        assert_eq!(a.parking.len(), 2);
        assert!(a.cards.values().any(|c| c.premises.len() == 2));
        assert!(a.cards.values().any(|c| c.premises.len() == 3));
        assert!(a.cards.values().filter(|c| !c.children.is_empty()).count() >= 3);
        assert!(
            a.cards
                .keys()
                .any(|&c| a.card_role(c) == crate::model::Role::Rebuts)
        );
    }

    #[test]
    fn note_order_counts() {
        let mut a = Document::new("c");
        let root = a.root;
        a.add_box(Parent::Box(root), BoxKind::Note, "first")
            .unwrap();
        a.add_box(Parent::Box(root), BoxKind::Note, "second")
            .unwrap();
        let mut b = a.clone();
        b.boxes[&root].children.reverse();
        assert!(structurally_equal(&a, &a.clone()).is_ok());
        let err = structurally_equal(&a, &b).unwrap_err();
        assert!(err.contains("notes"), "{err}");
    }
}
