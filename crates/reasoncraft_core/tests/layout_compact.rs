//! Compact contour-packed layout (step 5, Task 4).

use emath::Rect;
use reasoncraft_core::fixtures::{FixedMeasure, sample_iot_like};
use reasoncraft_core::layout::{Layout, LayoutParams, layout};
use reasoncraft_core::{CardId, CardKind, Document, Item, NewLoose, Parent};

fn run(doc: &Document) -> Layout {
    layout(doc, &FixedMeasure, &LayoutParams::default())
}

/// True when `a` and `b` overlap by more than half a pixel on both axes.
fn intersects(a: &Rect, b: &Rect) -> bool {
    let w = a.max.x.min(b.max.x) - a.min.x.max(b.min.x);
    let h = a.max.y.min(b.max.y) - a.min.y.max(b.min.y);
    w > 0.5 && h > 0.5
}

/// No two boxes intersect, every card contains its premises, no box outside
/// a card intersects it and no two card frames intersect.
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

#[test]
fn no_overlaps_in_any_direction() {
    for doc in [sample_iot_like(300), parked_fragments()] {
        let l = run(&doc);
        assert_no_overlaps(&doc, &l);
    }
}
