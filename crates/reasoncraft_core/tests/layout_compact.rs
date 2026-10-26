//! Compact contour-packed layout (step 5, Task 4).

use emath::Rect;
use reasoncraft_core::fixtures::{FixedMeasure, sample_iot_like};
use reasoncraft_core::layout::{Layout, LayoutParams, layout};
use reasoncraft_core::{CardId, CardKind, Document, Item, NewLoose, Parent, owner_index};

fn run(doc: &Document) -> Layout {
    layout(doc, &FixedMeasure, &LayoutParams::default())
}

/// True when `a` and `b` overlap by more than half a pixel on both axes.
fn intersects(a: &Rect, b: &Rect) -> bool {
    let w = a.max.x.min(b.max.x) - a.min.x.max(b.min.x);
    let h = a.max.y.min(b.max.y) - a.min.y.max(b.min.y);
    w > 0.5 && h > 0.5
}

/// True when the connector span `line` (possibly zero-width) crosses into
/// `r` by more than half a pixel across the cross axis.
fn crosses(line: &Rect, r: &Rect) -> bool {
    line.min.x < r.max.x - 0.5
        && line.max.x > r.min.x + 0.5
        && line.min.y < r.max.y
        && line.max.y > r.min.y
}

/// The rects a connector from `child` may touch: the child's own and its
/// parent's, with a card's premises counted as part of the card.
fn connector_ends(doc: &Document, child: Item) -> Vec<Item> {
    let card_and_premises = |c: CardId| {
        let premises = doc.cards[&c].premises.iter().map(|&p| Item::Box(p));
        std::iter::once(Item::Card(c)).chain(premises)
    };
    let mut ends = match child {
        Item::Card(c) => card_and_premises(c).collect(),
        Item::Box(_) => vec![child],
    };
    match owner_index(doc).get(&child) {
        Some(&Parent::Card(c)) => ends.extend(card_and_premises(c)),
        Some(&Parent::Box(b)) => match doc.cards.values().find(|c| c.premises.contains(&b)) {
            Some(card) => ends.extend(card_and_premises(card.id)),
            None => ends.push(Item::Box(b)),
        },
        _ => {}
    }
    ends
}

/// No two boxes intersect, every card contains its premises, no box outside
/// a card intersects it, no two card frames intersect and no connector span
/// runs through a box or frame other than its own ends.
fn assert_no_overlaps(doc: &Document, l: &Layout) {
    assert_eq!(l.boxes.len(), doc.boxes.len(), "every box gets a rect");
    assert_eq!(l.cards.len(), doc.cards.len(), "every card gets a frame");
    let boxes: Vec<_> = l.boxes.iter().collect();
    for (i, (a_id, a)) in boxes.iter().enumerate() {
        for (b_id, b) in &boxes[i + 1..] {
            assert!(!intersects(a, b), "boxes {a_id:?} {a:?} and {b_id:?} {b:?}");
        }
    }
    for (id, frame) in &l.cards {
        let premises = &doc.cards[id].premises;
        for pid in premises {
            assert!(frame.contains_rect(l.boxes[pid]), "premise outside card");
        }
        for (bid, r) in &l.boxes {
            if !premises.contains(bid) {
                assert!(!intersects(frame, r), "box {r:?} in card {frame:?}");
            }
        }
    }
    let cards: Vec<_> = l.cards.values().collect();
    for (i, a) in cards.iter().enumerate() {
        for b in &cards[i + 1..] {
            assert!(!intersects(a, b), "cards {a:?} and {b:?}");
        }
    }
    let rects: Vec<(Item, Rect)> = (l.boxes.iter().map(|(&id, &r)| (Item::Box(id), r)))
        .chain(l.cards.iter().map(|(&id, &r)| (Item::Card(id), r)))
        .collect();
    for c in &l.connectors {
        let ends = connector_ends(doc, c.child);
        // Between a premise port and its frame's edge the line runs inside
        // the parent's own frame, so the span starts at the parent's far edge.
        let parent_edge = (rects.iter().filter(|(item, _)| ends.contains(item)))
            .map(|(_, r)| r.bottom())
            .filter(|&b| b <= c.from.y)
            .fold(c.to.y, f32::max);
        let line = Rect::from_two_pos(emath::pos2(c.to.x, parent_edge), c.from)
            .shrink2(emath::vec2(0.0, 0.5));
        for (item, r) in &rects {
            if !ends.contains(item) {
                assert!(
                    !crosses(&line, r),
                    "connector of {:?} through {item:?} {r:?}",
                    c.child
                );
            }
        }
    }
}

/// Union of a card's frame and every rect below it.
fn subtree_bounds(doc: &Document, l: &Layout, card: CardId) -> Rect {
    let mut bounds = Rect::NOTHING;
    let mut stack = vec![Item::Card(card)];
    while let Some(item) = stack.pop() {
        match item {
            Item::Card(id) => {
                bounds = bounds.union(l.cards[&id]);
                let c = &doc.cards[&id];
                stack.extend(c.children.iter().map(|&k| Item::Card(k)));
                stack.extend(c.premises.iter().map(|&p| Item::Box(p)));
            }
            Item::Box(id) => {
                bounds = bounds.union(l.boxes[&id]);
                stack.extend(doc.boxes[&id].children.iter().copied());
            }
        }
    }
    bounds
}

#[test]
fn compact_layout_tucks_shallow_sibling() {
    let mut doc = Document::new("Claim");
    let root = Parent::Box(doc.root);
    let reason = |doc: &mut Document, parent, text: &str| {
        doc.add_card(parent, CardKind::Reason, text).unwrap()
    };
    // A: one premise, then one reason, then four reasons under that.
    let (a, a_premise) = reason(&mut doc, root, "A");
    let (_, a_mid) = reason(&mut doc, Parent::Box(a_premise), "A1");
    for i in 0..4 {
        reason(&mut doc, Parent::Box(a_mid), &format!("A2 {i}"));
    }
    // B: a single premise with no children.
    let (b, _) = reason(&mut doc, root, "B");
    // C: one premise with four reasons under it.
    let (c, c_premise) = reason(&mut doc, root, "C");
    for i in 0..4 {
        reason(&mut doc, Parent::Box(c_premise), &format!("C1 {i}"));
    }

    let p = LayoutParams::default();
    let l = run(&doc);
    assert_no_overlaps(&doc, &l);
    let (left_a, left_b, left_c) = (l.cards[&a].left(), l.cards[&b].left(), l.cards[&c].left());
    assert!(
        left_a < left_b && left_b < left_c,
        "A, B, C keep their order"
    );
    let (span_a, span_c) = (subtree_bounds(&doc, &l, a), subtree_bounds(&doc, &l, c));
    assert!(
        span_c.left() < span_a.right(),
        "C tucks into A's cross range"
    );
    let widths: f32 = [a, b, c]
        .iter()
        .map(|&k| subtree_bounds(&doc, &l, k).width())
        .sum();
    assert!(
        l.main_bounds.width() < widths + 2.0 * p.h_gap,
        "main width {} not less than {}",
        l.main_bounds.width(),
        widths + 2.0 * p.h_gap
    );
}

/// Three parked fragments with children of their own, beside a small map.
fn parked_fragments() -> Document {
    let mut doc = Document::new("Claim");
    let (_, p) = doc
        .add_card(Parent::Box(doc.root), CardKind::Reason, "Reason")
        .unwrap();
    doc.add_card(Parent::Box(p), CardKind::Objection, "Objection")
        .unwrap();
    for i in 0..3 {
        let Item::Card(card) = doc.add_loose(NewLoose::Card(CardKind::Reason), &format!("F{i}"))
        else {
            panic!("a loose card is a card");
        };
        let premise = doc.cards[&card].premises[0];
        for j in 0..=i {
            doc.add_card(Parent::Box(premise), CardKind::Reason, &format!("F{i} {j}"))
                .unwrap();
        }
    }
    doc
}

/// A short card with four children, then a card whose tall premise reaches
/// down into the band the first card's connectors cross.
fn tall_neighbour() -> Document {
    let mut doc = Document::new("Claim");
    let (_, short) = doc
        .add_card(Parent::Box(doc.root), CardKind::Reason, "Short")
        .unwrap();
    for i in 0..4 {
        doc.add_card(Parent::Box(short), CardKind::Reason, &format!("Child {i}"))
            .unwrap();
    }
    let tall = ["Tall"; 6].join(
        "
",
    );
    doc.add_card(Parent::Box(doc.root), CardKind::Reason, &tall)
        .unwrap();
    doc
}

#[test]
fn no_overlaps_in_any_direction() {
    for doc in [sample_iot_like(300), parked_fragments(), tall_neighbour()] {
        let l = run(&doc);
        assert_no_overlaps(&doc, &l);
    }
}
