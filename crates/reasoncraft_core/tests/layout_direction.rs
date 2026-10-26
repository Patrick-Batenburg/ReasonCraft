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
