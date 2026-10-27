//! Layout directions (step 5, Task 5).

use reasoncraft_core::fixtures::{FixedMeasure, sample_iot_like};
use reasoncraft_core::layout::{Layout, LayoutParams, layout};
use reasoncraft_core::{CardKind, Document, LayoutDirection, Parent};

fn run_in(doc: &mut Document, dir: LayoutDirection) -> Layout {
    doc.layout_direction = dir;
    layout(doc, &FixedMeasure, &LayoutParams::default())
}

#[test]
fn children_follow_direction() {
    let mut doc = sample_iot_like(150);
    for dir in LayoutDirection::ALL {
        let l = run_in(&mut doc, dir);
        assert!(!l.connectors.is_empty());
        for c in &l.connectors {
            assert_eq!(c.dir, dir);
            let along = (c.from - c.to).dot(dir.toward_parent());
            assert!(
                along < 0.0,
                "{dir:?}: {:?} from {:?} to {:?}",
                c.child,
                c.from,
                c.to
            );
        }
        assert_eq!(l.main_bounds.min, emath::Pos2::ZERO, "{dir:?}");
    }
}

#[test]
fn parking_sits_after_the_main_tree_along_the_cross_axis() {
    let mut doc = sample_iot_like(60);
    for dir in LayoutDirection::ALL {
        let l = run_in(&mut doc, dir);
        let (main, parking) = (l.main_bounds, l.parking_bounds.expect("sample parks items"));
        match dir {
            LayoutDirection::LeftRight => assert!(parking.top() > main.bottom(), "{dir:?}"),
            _ => assert!(parking.left() > main.right(), "{dir:?}"),
        }
    }
}

/// Left-to-right, co-premises stack downward under the label band, which
/// stays at the frame's top in every direction.
#[test]
fn card_internals_follow_direction() {
    let p = LayoutParams::default();
    let mut doc = Document::new("Claim");
    let (card, first) = doc
        .add_card(Parent::Box(doc.root), CardKind::Reason, "One")
        .unwrap();
    let second = doc.add_premise(card, None, "Two").unwrap();
    for dir in LayoutDirection::ALL {
        let l = run_in(&mut doc, dir);
        let (frame, a, b) = (l.cards[&card], l.boxes[&first], l.boxes[&second]);
        assert_eq!(a.top(), frame.top() + p.card_label_height, "{dir:?}");
        assert!((l.card_labels[&card].y - frame.top() - p.card_label_height / 2.0).abs() < 1e-3);
        if dir == LayoutDirection::LeftRight {
            assert_eq!(a.left(), b.left());
            assert_eq!(b.top(), a.bottom() + p.premise_gap);
        } else {
            assert_eq!(a.top(), b.top());
            assert_eq!(b.left(), a.right() + p.premise_gap);
        }
    }
}

/// A chain of four cards is wider than tall left-to-right and taller than
/// wide otherwise; bottom-up mirrors top-down.
#[test]
fn chain_extends_along_the_main_axis() {
    let mut doc = Document::new("Claim");
    let mut at = doc.root;
    for i in 0..4 {
        at = doc
            .add_card(Parent::Box(at), CardKind::Reason, &format!("Reason {i}"))
            .unwrap()
            .1;
    }
    let td = run_in(&mut doc, LayoutDirection::TopDown);
    let bu = run_in(&mut doc, LayoutDirection::BottomUp);
    let lr = run_in(&mut doc, LayoutDirection::LeftRight);
    assert!(td.bounds.height() > td.bounds.width());
    assert!(lr.bounds.width() > lr.bounds.height());
    assert_eq!(bu.bounds.size(), td.bounds.size());
    let (root_td, root_bu) = (td.boxes[&doc.root], bu.boxes[&doc.root]);
    assert_eq!(root_td.top(), 0.0);
    assert_eq!(root_bu.bottom(), bu.bounds.bottom());
}

/// Two co-premises with two sources each: each pair sits under its own
/// premise, left to right as the premises are.
#[test]
fn co_premise_sources_stay_apart() {
    use reasoncraft_core::{BoxKind, SourceType};
    let mut doc = Document::new("Claim");
    let (card, a) = doc
        .add_card(Parent::Box(doc.root), CardKind::Reason, "A")
        .unwrap();
    let b = doc.add_premise(card, None, "B").unwrap();
    let source = || BoxKind::Source {
        source_type: SourceType::Data,
    };
    let mut ids = Vec::new();
    for premise in [a, b] {
        for i in 0..2 {
            ids.push(
                doc.add_box(Parent::Box(premise), source(), &format!("S{i}"))
                    .unwrap(),
            );
        }
    }
    for dir in LayoutDirection::ALL {
        let l = run_in(&mut doc, dir);
        let along = |id| match dir {
            LayoutDirection::LeftRight => l.boxes[&id].center().y,
            _ => l.boxes[&id].center().x,
        };
        assert!(
            along(ids[1]) < along(ids[2]),
            "{dir:?}: A's sources before B's"
        );
    }
}

/// A rated premise has an indicator row; its unrated co-premise reserves
/// one too, so their texts start at the same offset (side by side, at the
/// same height).
#[test]
fn co_premise_texts_align_when_one_is_rated() {
    let mut doc = Document::new("Claim");
    let (card, rated) = doc
        .add_card(Parent::Box(doc.root), CardKind::Reason, "Rated")
        .unwrap();
    let plain = doc.add_premise(card, None, "Plain").unwrap();
    doc.boxes.get_mut(&rated).unwrap().acceptance = Some(reasoncraft_core::Acceptance::Accept);
    for dir in LayoutDirection::ALL {
        let l = run_in(&mut doc, dir);
        assert!(l.indicator_row(rated) > 0.0, "{dir:?}");
        assert_eq!(l.indicator_row(plain), l.indicator_row(rated), "{dir:?}");
        let (a, b) = (l.body(rated).unwrap(), l.body(plain).unwrap());
        let (ra, rb) = (l.boxes[&rated], l.boxes[&plain]);
        assert_eq!(a.top() - ra.top(), b.top() - rb.top(), "{dir:?}");
        if dir != LayoutDirection::LeftRight {
            assert_eq!(a.top(), b.top(), "{dir:?}: first lines level");
        }
    }
}
