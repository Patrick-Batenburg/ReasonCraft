//! Test-document builders and a fixed text measurer (feature `test-fixtures`).

use crate::edit::NewLoose;
use crate::ids::{CardId, Item, NodeId, Parent};
use crate::layout::Measure;
use crate::model::{BoxKind, BoxNode, Card, CardKind, Document};

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

/// Whether `a` and `b` hold the same map as far as Argdown carries it
/// (`docs/argdown.md`): title, author, language, the tree and the parking
/// area with every box's kind, text, acceptance, rating, link and
/// comments (author, text, verdict), and every card's kind, strength,
/// premises and children, in order.
///
/// Ignored, because Argdown does not keep them: ids, images, widths,
/// comment dates, the links and comments of notes under a box, where notes
/// sit among their other siblings (the notes' own order counts), and the
/// spaces around a box's text (line breaks in it count as spaces).
pub fn structurally_equal(a: &Document, b: &Document) -> Result<(), String> {
    for (what, x, y) in [
        ("title", &a.meta.title, &b.meta.title),
        ("author", &a.meta.author, &b.meta.author),
        ("language", &a.meta.language, &b.meta.language),
    ] {
        if x != y {
            return Err(format!("{what}: {x:?} != {y:?}"));
        }
    }
    let mut cmp = Compare {
        a,
        b,
        todo: vec![(
            Item::Box(a.root),
            Item::Box(b.root),
            "root".to_owned(),
            true,
        )],
    };
    if a.parking.len() != b.parking.len() {
        return Err(format!(
            "parking: {} items != {}",
            a.parking.len(),
            b.parking.len()
        ));
    }
    for (k, (&x, &y)) in a.parking.iter().zip(&b.parking).enumerate() {
        cmp.todo.push((x, y, format!("parking[{k}]"), true));
    }
    // A work list, not recursion: maps can be thousands of levels deep.
    while let Some((x, y, at, whole)) = cmp.todo.pop() {
        cmp.item(x, y, &at, whole)?;
    }
    Ok(())
}

struct Compare<'a> {
    a: &'a Document,
    b: &'a Document,
    /// Pairs still to compare: items, where they are, and `whole`.
    todo: Vec<(Item, Item, String, bool)>,
}

impl Compare<'_> {
    /// Compare two items and all under them. `whole`: a note compares its
    /// link and comments too (a parked note keeps them).
    fn item(&mut self, x: Item, y: Item, at: &str, whole: bool) -> Result<(), String> {
        match (x, y) {
            (Item::Box(x), Item::Box(y)) => self.node(x, y, at, whole),
            (Item::Card(x), Item::Card(y)) => self.card(x, y, at),
            _ => Err(format!("{at}: a box and a card")),
        }
    }

    fn node(&mut self, x: NodeId, y: NodeId, at: &str, whole: bool) -> Result<(), String> {
        let (Some(p), Some(q)) = (self.a.boxes.get(&x), self.b.boxes.get(&y)) else {
            return Err(format!("{at}: missing box"));
        };
        let at = format!("{at} {:?}", flat(&p.text));
        if p.kind != q.kind {
            return Err(format!("{at}: kind {:?} != {:?}", p.kind, q.kind));
        }
        if flat(&p.text) != flat(&q.text) {
            return Err(format!("{at}: text != {:?}", flat(&q.text)));
        }
        let comments = |n: &crate::model::BoxNode| {
            let c = n.comments.iter();
            c.map(|c| (c.author.clone(), c.text.clone(), c.verdict))
                .collect::<Vec<_>>()
        };
        let note = p.kind == BoxKind::Note;
        let mut fields = vec![
            (
                "acceptance",
                format!("{:?}", p.acceptance),
                format!("{:?}", q.acceptance),
            ),
            (
                "rating",
                format!("{:?}", p.source_rating),
                format!("{:?}", q.source_rating),
            ),
        ];
        if !note || whole {
            fields.push(("link", format!("{:?}", p.link), format!("{:?}", q.link)));
            fields.push((
                "comments",
                format!("{:?}", comments(p)),
                format!("{:?}", comments(q)),
            ));
        }
        for (what, u, v) in fields {
            if u != v {
                return Err(format!("{at}: {what} {u} != {v}"));
            }
        }
        let split = |d: &Document, items: &[Item]| {
            let is_note = |i: &Item| matches!(i, Item::Box(b) if d.boxes.get(b).is_some_and(|n| n.kind == BoxKind::Note));
            let (notes, rest): (Vec<Item>, Vec<Item>) = items.iter().partition(|i| is_note(i));
            let texts: Vec<String> = notes
                .iter()
                .filter_map(|i| match i {
                    Item::Box(b) => d.boxes.get(b).map(|n| flat(&n.text)),
                    Item::Card(_) => None,
                })
                .collect();
            (texts, rest)
        };
        let (notes_p, rest_p) = split(self.a, &p.children);
        let (notes_q, rest_q) = split(self.b, &q.children);
        if notes_p != notes_q {
            return Err(format!("{at}: notes {notes_p:?} != {notes_q:?}"));
        }
        self.list(&rest_p, &rest_q, &at)
    }

    fn card(&mut self, x: CardId, y: CardId, at: &str) -> Result<(), String> {
        let (Some(c), Some(d)) = (self.a.cards.get(&x), self.b.cards.get(&y)) else {
            return Err(format!("{at}: missing card"));
        };
        let at = format!("{at} card");
        if c.kind != d.kind || c.strength != d.strength {
            let (u, v) = ((&c.kind, c.strength), (&d.kind, d.strength));
            return Err(format!("{at}: kind and strength {u:?} != {v:?}"));
        }
        let premises = |c: &crate::model::Card| c.premises.iter().map(|&p| Item::Box(p)).collect();
        let (pc, pd): (Vec<Item>, Vec<Item>) = (premises(c), premises(d));
        self.list(&pc, &pd, &format!("{at} premise"))?;
        let children = |c: &crate::model::Card| c.children.iter().map(|&k| Item::Card(k)).collect();
        let (kc, kd): (Vec<Item>, Vec<Item>) = (children(c), children(d));
        self.list(&kc, &kd, &format!("{at} whole-card child"))
    }

    fn list(&mut self, xs: &[Item], ys: &[Item], at: &str) -> Result<(), String> {
        if xs.len() != ys.len() {
            return Err(format!("{at}: {} children != {}", xs.len(), ys.len()));
        }
        for (k, (&x, &y)) in xs.iter().zip(ys).enumerate().rev() {
            self.todo.push((x, y, format!("{at}/{k}"), false));
        }
        Ok(())
    }
}

/// Text as Argdown keeps it: on one line, without surrounding spaces.
fn flat(text: &str) -> String {
    let lines: Vec<&str> = text
        .split(['\n', '\r'])
        .filter(|l| !l.trim().is_empty())
        .collect();
    lines.join(" ").trim().to_owned()
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
