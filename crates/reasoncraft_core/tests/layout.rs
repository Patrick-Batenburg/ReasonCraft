//! Layout tests (spec: Task 6).

use std::time::{Duration, Instant};

use emath::Rect;
use reasoncraft_core::fixtures::{FixedMeasure, sample_iot_like};
use reasoncraft_core::layout::{ConnectorKind, Layout, LayoutParams, TargetKind, layout};
use reasoncraft_core::{BoxKind, CardKind, Document, Item, NodeId, Parent, Role};

fn run(doc: &Document) -> Layout {
    layout(doc, &FixedMeasure, &LayoutParams::default())
}

/// Strict overlap: touching edges do not count.
fn overlaps(a: &Rect, b: &Rect) -> bool {
    a.min.x < b.max.x && b.min.x < a.max.x && a.min.y < b.max.y && b.min.y < a.max.y
}

#[test]
fn boxes_never_overlap() {
    let doc = sample_iot_like(117);
    let l = run(&doc);
    assert_eq!(l.boxes.len(), doc.boxes.len(), "every box gets a rect");
    assert_eq!(l.cards.len(), doc.cards.len(), "every card gets a frame");

    // Exactly one connector per child item; none for the root, parking
    // roots or premises.
    let premises: usize = doc.cards.values().map(|c| c.premises.len()).sum();
    let items = doc.cards.len() + doc.boxes.len() - premises;
    assert_eq!(l.connectors.len(), items - 1 - doc.parking.len());
    for c in &l.connectors {
        let expected = match c.child {
            Item::Card(id) => match doc.card_role(id) {
                Role::Supports => ConnectorKind::Supports,
                Role::Opposes => ConnectorKind::Opposes,
                Role::Rebuts => ConnectorKind::Rebuts,
            },
            Item::Box(id) => match doc.boxes[&id].kind {
                BoxKind::Category => ConnectorKind::Neutral,
                _ => ConnectorKind::Attachment,
            },
        };
        assert_eq!(c.kind, expected);
    }

    // (rect, Some(card) if this is a card frame, Some(card) if a premise of that card)
    let mut rects: Vec<(Rect, Option<Item>, Option<Item>)> = Vec::new();
    for (id, r) in &l.boxes {
        let owner_card = doc
            .cards
            .values()
            .find(|c| c.premises.contains(id))
            .map(|c| Item::Card(c.id));
        rects.push((*r, None, owner_card));
    }
    for (id, r) in &l.cards {
        rects.push((*r, Some(Item::Card(*id)), None));
    }
    for i in 0..rects.len() {
        for j in (i + 1)..rects.len() {
            let (a, a_frame, a_in) = &rects[i];
            let (b, b_frame, b_in) = &rects[j];
            let premise_in_own_frame =
                (a_frame.is_some() && a_frame == b_in) || (b_frame.is_some() && b_frame == a_in);
            if premise_in_own_frame {
                assert!(
                    a.contains_rect(*b) || b.contains_rect(*a),
                    "premise must sit inside its own card frame"
                );
                continue;
            }
            assert!(!overlaps(a, b), "overlap between {a:?} and {b:?}");
        }
    }
}

#[test]
fn long_unbroken_word_grows_height() {
    let mut doc = Document::new("Claim");
    let word = "x".repeat(80);
    let (_, p) = doc
        .add_card(Parent::Box(doc.root), CardKind::Reason, &word)
        .unwrap();
    let params = LayoutParams::default();
    let l = layout(&doc, &FixedMeasure, &params);
    let r = l.boxes[&p];
    assert!(r.height() > params.min_box_height, "height {}", r.height());
    assert_eq!(r.width(), params.box_width);
}

#[test]
fn whole_card_target_attaches_at_frame_bottom() {
    let mut doc = Document::new("Claim");
    let (card, _) = doc
        .add_card(Parent::Box(doc.root), CardKind::Reason, "Reason")
        .unwrap();
    doc.add_premise(card, None, "Co-premise").unwrap();
    let (attacker, _) = doc
        .add_card(Parent::Card(card), CardKind::Objection, "Inference fails")
        .unwrap();
    let l = run(&doc);
    let whole: Vec<_> = l
        .connectors
        .iter()
        .filter(|c| c.target == TargetKind::WholeCard)
        .collect();
    assert_eq!(whole.len(), 1);
    let c = whole[0];
    assert_eq!(c.child, Item::Card(attacker));
    let frame = l.cards[&card];
    assert_eq!(c.to.y, frame.bottom());
    assert!(c.to.x >= frame.left() && c.to.x <= frame.right());

    // The sample map has at least three whole-card attacks, all at frame bottoms.
    let doc = sample_iot_like(117);
    let l = run(&doc);
    let mut n = 0;
    for c in l
        .connectors
        .iter()
        .filter(|c| c.target == TargetKind::WholeCard)
    {
        let Item::Card(child) = c.child else {
            panic!("whole-card child must be a card")
        };
        let target = doc
            .cards
            .values()
            .find(|k| k.children.contains(&child))
            .unwrap();
        let frame = l.cards[&target.id];
        assert_eq!(c.to.y, frame.bottom());
        assert!(c.to.x >= frame.left() && c.to.x <= frame.right());
        n += 1;
    }
    assert!(n >= 3, "sample has {n} whole-card attacks");
}

/// On a single-premise card the premise centre and the frame centre share
/// an x, so the two attack kinds must be spread apart.
#[test]
fn premise_and_whole_card_ports_do_not_coincide() {
    use reasoncraft_core::connector::{STUB_PX, Segment, curved_path};
    let p = LayoutParams::default();
    for premises in [1, 3] {
        let mut doc = Document::new("Claim");
        let (card, first) = doc
            .add_card(Parent::Box(doc.root), CardKind::Reason, "Reason")
            .unwrap();
        let mut middle = first;
        for i in 1..premises {
            let id = doc.add_premise(card, None, "Co-premise").unwrap();
            if i == premises / 2 {
                middle = id;
            }
        }
        let (on_premise, _) = doc
            .add_card(Parent::Box(middle), CardKind::Objection, "Untrue")
            .unwrap();
        let (on_card, _) = doc
            .add_card(Parent::Card(card), CardKind::Objection, "Does not follow")
            .unwrap();
        let l = run(&doc);
        let find = |id| {
            l.connectors
                .iter()
                .find(|c| c.child == Item::Card(id))
                .unwrap()
        };
        let (a, w) = (find(on_premise), find(on_card));
        assert_eq!(a.target, TargetKind::Box);
        assert_eq!(a.to.y, l.boxes[&middle].bottom());
        assert_eq!(w.target, TargetKind::WholeCard);
        assert_eq!(w.to.y, l.cards[&card].bottom());
        assert!(
            (a.to.x - w.to.x).abs() >= p.port_spacing - 1e-3,
            "{premises} premises: ports {} and {} too close",
            a.to.x,
            w.to.x
        );
        // The premise connector's stub must not run through the whole-card end.
        let path = curved_path(a.from, a.to, STUB_PX);
        let Some(Segment::Line([s0, s1])) = path.last().copied() else {
            panic!("path ends in a stub");
        };
        let on_stub =
            (w.to.x - s0.x).abs() < 1.0 && w.to.y >= s0.y.min(s1.y) && w.to.y <= s0.y.max(s1.y);
        assert!(
            !on_stub,
            "{premises} premises: stub crosses the whole-card end"
        );
    }
}

#[test]
fn shared_ports_are_spread_and_ordered() {
    let mut doc = Document::new("Claim");
    let mut cards = Vec::new();
    for i in 0..5 {
        let (c, _) = doc
            .add_card(Parent::Box(doc.root), CardKind::Reason, &format!("R{i}"))
            .unwrap();
        cards.push(c);
    }
    let l = run(&doc);
    assert_eq!(l.connectors.len(), 5);
    let mut cs: Vec<_> = l.connectors.iter().collect();
    cs.sort_by(|a, b| a.from.x.total_cmp(&b.from.x));
    for w in cs.windows(2) {
        assert!(w[1].to.x > w[0].to.x, "ports increase with child x");
        assert!((w[1].to.x - w[0].to.x - 16.0).abs() < 1e-3, "16 px apart");
    }
    let root = l.boxes[&doc.root];
    for c in &cs {
        assert_eq!(c.to.y, root.bottom());
        assert_eq!(c.target, TargetKind::Box);
    }
    let mid = (cs[0].to.x + cs[4].to.x) / 2.0;
    assert!(
        (mid - root.center().x).abs() < 1e-3,
        "ports centred on the box"
    );
}

#[test]
fn label_anchor_inside_frame() {
    let doc = sample_iot_like(117);
    let params = LayoutParams::default();
    let l = layout(&doc, &FixedMeasure, &params);
    assert_eq!(l.card_labels.len(), l.cards.len());
    for (id, frame) in &l.cards {
        let a = l.card_labels[id];
        assert!(frame.contains(a), "anchor {a:?} outside {frame:?}");
        assert!(a.y <= frame.top() + params.card_label_height);
    }
}

#[test]
fn parking_is_right_of_main() {
    let doc = sample_iot_like(117);
    assert!(!doc.parking.is_empty());
    let l = run(&doc);
    let parking = l.parking_bounds.expect("parking bounds");
    assert!(parking.left() >= l.main_bounds.right() + 120.0);
    assert!(l.bounds.contains_rect(parking));
    assert!(l.bounds.contains_rect(l.main_bounds));

    let empty = run(&Document::new("Claim"));
    assert!(empty.parking_bounds.is_none());
}

#[test]
fn deep_chain_no_overlap() {
    let mut doc = Document::new("Claim");
    let mut parent = Parent::Box(doc.root);
    for i in 0..200 {
        let (_, p): (_, NodeId) = doc
            .add_card(parent, CardKind::Reason, &format!("Reason {i}"))
            .unwrap();
        parent = Parent::Box(p);
    }
    let l = run(&doc);
    assert_eq!(l.cards.len(), 200);
    for (card, frame) in &l.cards {
        let Some(Parent::Box(owner)) = doc.owner_of(Item::Card(*card)) else {
            panic!("chain card must hang under a box")
        };
        assert!(frame.top() > l.boxes[&owner].bottom());
    }
    for c in &l.connectors {
        assert!(c.from.y > c.to.y, "child top below parent bottom");
    }
}

#[test]
#[ignore = "performance; run with --release --ignored"]
fn layout_500_boxes_fast() {
    let doc = sample_iot_like(500);
    assert!(
        doc.boxes.len() >= 490,
        "sample has {} boxes",
        doc.boxes.len()
    );
    let _ = run(&doc); // warm up
    let start = Instant::now();
    let l = run(&doc);
    let elapsed = start.elapsed();
    println!("layout of {} boxes took {elapsed:?}", doc.boxes.len());
    assert_eq!(l.boxes.len(), doc.boxes.len());
    assert!(elapsed < Duration::from_millis(50), "took {elapsed:?}");
}
