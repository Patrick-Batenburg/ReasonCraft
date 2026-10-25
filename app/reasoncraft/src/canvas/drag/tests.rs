use super::*;
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
    assert_eq!(target(Item::Box(p2), l.boxes[&p].center()), None);
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
                highlight: Some(l.boxes[&p])
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
