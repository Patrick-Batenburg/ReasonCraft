use super::*;
use crate::actions::Insertable;
use eframe::egui::vec2;
use reasoncraft_core::fixtures::FixedMeasure;
use reasoncraft_core::{CardKind, LayoutParams, NewLoose, SourceType, layout};

#[test]
fn drop_targets_connect_reattach_and_park() {
    let mut d = Document::new("Claim");
    let root = d.root;
    let (a, p) = d
        .add_card(Parent::Box(root), CardKind::Reason, "p")
        .unwrap();
    let p2 = d.add_premise(a, None, "p2").unwrap();
    let Item::Card(loose) = d.add_loose(NewLoose::Card(CardKind::Objection), "q") else {
        unreachable!()
    };
    let q = d.cards[&loose].premises[0];
    let l = layout(&d, &FixedMeasure, &LayoutParams::default());
    let target = |item, world| drop_target(&d, &l, item, world, 1.0).map(|t| t.to);

    // The root is fixed; a sole premise drags its card; a co-premise drags itself.
    assert_eq!(drag_item(&d, Hit::Box(root)), None);
    assert_eq!(drag_item(&d, Hit::Box(q)), Some(Item::Card(loose)));
    assert_eq!(drag_item(&d, Hit::Box(p)), Some(Item::Box(p)));

    let lc = Item::Card(loose);
    assert_eq!(target(lc, l.boxes[&root].center()), Some(Parent::Box(root)));
    assert_eq!(target(lc, l.boxes[&p].center()), Some(Parent::Box(p)));
    assert_eq!(
        target(lc, l.cards[&a].center_bottom()),
        Some(Parent::Card(a))
    );
    let empty = l.bounds.right_bottom() + vec2(500.0, 500.0);
    assert_eq!(target(lc, empty), None, "already parked");
    assert_eq!(target(lc, l.boxes[&q].center()), None, "onto itself");

    let ac = Item::Card(a);
    assert_eq!(target(ac, empty), Some(Parent::Parking));
    assert_eq!(
        target(ac, l.boxes[&p2].center()),
        None,
        "onto its own premise"
    );
    // A co-premise dropped on another card's premise joins that card.
    assert_eq!(
        target(Item::Box(p2), l.boxes[&q].center()),
        Some(Parent::Card(loose))
    );
    // Onto itself, or right of its only co-premise (where it is): no change.
    assert_eq!(target(Item::Box(p2), l.boxes[&p2].center()), None);
    let right_of_p = l.boxes[&p].center() + vec2(1.0, 0.0);
    assert_eq!(target(Item::Box(p2), right_of_p), None);
}

#[test]
fn panel_drop_target_rules() {
    let mut d = Document::new("Claim");
    let root = d.root;
    let cat = d
        .add_box(Parent::Box(root), BoxKind::Category, "cat")
        .unwrap();
    // The root holds a category, so the reason card is parked.
    let Item::Card(c) = d.add_loose(NewLoose::Card(CardKind::Reason), "p") else {
        unreachable!()
    };
    let p = d.cards[&c].premises[0];
    let l = layout(&d, &FixedMeasure, &LayoutParams::default());
    let at = |what: Insertable, world| panel_drop_target(&d, &l, &what, world, 1.0).unwrap();
    let empty = l.bounds.right_bottom() + vec2(500.0, 500.0);
    let source = Insertable::Source(SourceType::Web);

    // A source over a category: refused, red over the category.
    let (t, ok) = at(source.clone(), l.boxes[&cat].center());
    assert_eq!((t.to, ok), (Parent::Box(cat), false));
    assert_eq!(t.highlight, Some(l.boxes[&cat]));
    assert_eq!(
        at(source, l.boxes[&p].center()),
        (
            DropTarget {
                to: Parent::Box(p),
                index: None,
                highlight: Some(l.boxes[&p]),
                bar: None,
            },
            true
        )
    );

    // A reason over a card's bottom edge: a whole-card target.
    let (t, ok) = at(Insertable::Reason, l.cards[&c].center_bottom());
    assert_eq!((t.to, ok), (Parent::Card(c), true));
    let (t, ok) = at(Insertable::Note, l.cards[&c].center_bottom());
    assert_eq!((t.to, ok), (Parent::Card(c), false), "cards only");

    // Empty canvas: parking for boxes, never for a comment.
    assert_eq!(at(Insertable::Note, empty).0.to, Parent::Parking);
    assert!(at(Insertable::Note, empty).1);
    let (t, ok) = at(Insertable::Comment(0), empty);
    assert_eq!((t.to, ok), (Parent::Parking, false));
    assert_eq!(panel_drop_action(&Insertable::Comment(0), &t), None);

    // A comment goes on any box; a co-premise on a premise joins its card.
    let (t, ok) = at(Insertable::Comment(3), l.boxes[&cat].center());
    assert!(ok);
    assert_eq!(
        panel_drop_action(&Insertable::Comment(3), &t),
        Some(Action::Insert {
            what: Insertable::Comment(3),
            to: None,
            on: Some(cat)
        })
    );
    let (t, ok) = at(Insertable::CoPremise, l.boxes[&p].center());
    assert_eq!(
        (t.to, t.highlight, ok),
        (Parent::Box(p), Some(l.cards[&c]), true)
    );
    let (t, ok) = at(Insertable::CoPremise, l.cards[&c].center_bottom());
    assert_eq!((t.to, ok), (Parent::Box(p), true));
    assert!(!at(Insertable::CoPremise, l.boxes[&cat].center()).1);
    assert!(!at(Insertable::CoPremise, empty).1);
}

/// Three sibling cards under the root, and a parked card.
fn three_cards() -> (Document, [reasoncraft_core::CardId; 4]) {
    let mut d = Document::new("Claim");
    let root = d.root;
    let mut card = |t| {
        d.add_card(Parent::Box(root), CardKind::Reason, t)
            .unwrap()
            .0
    };
    let (a, b, c) = (card("a"), card("b"), card("c"));
    let Item::Card(parked) = d.add_loose(NewLoose::Card(CardKind::Reason), "p") else {
        unreachable!()
    };
    (d, [a, b, c, parked])
}

#[test]
fn drop_index_from_pointer_x() {
    let (d, [a, b, c, parked]) = three_cards();
    let root = d.root;
    let l = layout(&d, &FixedMeasure, &LayoutParams::default());
    let (ra, rb, rc) = (l.cards[&a], l.cards[&b], l.cards[&c]);
    let between_ab = egui::pos2((ra.right() + rb.left()) / 2.0, ra.center().y);

    // Pointer between the 1st and 2nd card: index 1, bar in the gap.
    let t = drop_target(&d, &l, Item::Card(c), between_ab, 1.0).unwrap();
    assert_eq!((t.to, t.index), (Parent::Box(root), Some(1)));
    let bar = t.bar.expect("insertion bar");
    assert_eq!(bar.left(), between_ab.x);
    assert_eq!(bar.width(), 0.0);
    assert!(bar.top() <= ra.top() && bar.bottom() >= ra.bottom());
    assert_eq!(t.highlight, Some(l.boxes[&root]), "the parent lights up");
    // A parked card dropped there is inserted, not appended.
    let t = drop_target(&d, &l, Item::Card(parked), between_ab, 1.0).unwrap();
    assert_eq!((t.to, t.index), (Parent::Box(root), Some(1)));
    // Dragging the 1st card to the same gap leaves it where it is.
    assert_eq!(drop_target(&d, &l, Item::Card(a), between_ab, 1.0), None);
    // Just past either end of the row.
    let left_end = egui::pos2(ra.left() - 4.0, ra.center().y);
    let right_end = egui::pos2(rc.right() + 4.0, rc.center().y);
    let t = drop_target(&d, &l, Item::Card(c), left_end, 1.0).unwrap();
    assert_eq!(t.index, Some(0));
    let t = drop_target(&d, &l, Item::Card(a), right_end, 1.0).unwrap();
    assert_eq!(t.index, Some(2));
    // Far below the row is empty canvas: parking.
    let below = egui::pos2(between_ab.x, ra.bottom() + 40.0);
    assert_eq!(
        drop_target(&d, &l, Item::Card(c), below, 1.0).map(|t| t.to),
        Some(Parent::Parking)
    );
    // Over the parent box itself the pointer's x picks the place too.
    let over_root = l.boxes[&root].center();
    let t = drop_target(&d, &l, Item::Card(parked), over_root, 1.0).unwrap();
    assert_eq!((t.to, t.index), (Parent::Box(root), Some(1)));
}

#[test]
fn co_premise_reorder_within_card() {
    let mut d = Document::new("Claim");
    let root = d.root;
    let (card, p1) = d
        .add_card(Parent::Box(root), CardKind::Reason, "1")
        .unwrap();
    let p2 = d.add_premise(card, None, "2").unwrap();
    let p3 = d.add_premise(card, None, "3").unwrap();
    let l = layout(&d, &FixedMeasure, &LayoutParams::default());
    // p3 over the left part of p1: before p1.
    let left_of_p1 = l.boxes[&p1].center() - vec2(10.0, 0.0);
    let t = drop_target(&d, &l, Item::Box(p3), left_of_p1, 1.0).unwrap();
    assert_eq!((t.to, t.index), (Parent::Card(card), Some(0)));
    assert!(t.bar.is_some());
    // p1 in the gap between p2 and p3 (inside the frame).
    let (r2, r3) = (l.boxes[&p2], l.boxes[&p3]);
    let gap = egui::pos2((r2.right() + r3.left()) / 2.0, r2.center().y);
    let t = drop_target(&d, &l, Item::Box(p1), gap, 1.0).unwrap();
    assert_eq!((t.to, t.index), (Parent::Card(card), Some(1)));
    let mut moved = d.clone();
    moved.move_item(Item::Box(p1), t.to, t.index).unwrap();
    assert_eq!(moved.cards[&card].premises, vec![p2, p1, p3]);
}

/// Children [cardA, source, cardB] show as [source, cardA, cardB]. Dragging
/// cardA into the gap next to itself changes only hidden document order:
/// no target, so no bar and no move.
#[test]
fn drop_that_only_changes_hidden_order_is_no_change() {
    let mut d = Document::new("Claim");
    let root = d.root;
    let a = d
        .add_card(Parent::Box(root), CardKind::Reason, "a")
        .unwrap()
        .0;
    d.add_box(
        Parent::Box(root),
        BoxKind::Source {
            source_type: SourceType::Web,
        },
        "s",
    )
    .unwrap();
    let b = d
        .add_card(Parent::Box(root), CardKind::Reason, "b")
        .unwrap()
        .0;
    let l = layout(&d, &FixedMeasure, &LayoutParams::default());
    let (ra, rb) = (l.cards[&a], l.cards[&b]);
    assert!(ra.left() < rb.left(), "cardA shows before cardB");
    let between = egui::pos2((ra.right() + rb.left()) / 2.0, ra.center().y);
    assert_eq!(drop_target(&d, &l, Item::Card(a), between, 1.0), None);
    let left_of_a = egui::pos2(ra.left() - 4.0, ra.center().y);
    assert_eq!(drop_target(&d, &l, Item::Card(a), left_of_a, 1.0), None);
    // Past cardB it really moves.
    let right_end = egui::pos2(rb.right() + 4.0, rb.center().y);
    let t = drop_target(&d, &l, Item::Card(a), right_end, 1.0).unwrap();
    assert_eq!(t.index, Some(2));
}
