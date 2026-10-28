use super::*;
use crate::fixtures::deep_chain as chain;
use crate::ids::{CardId, NodeId};
use crate::model::{BoxKind, CardKind};

fn deepest(d: &Document) -> usize {
    d.boxes
        .keys()
        .filter_map(|b| d.depth_of(Item::Box(*b)))
        .max()
        .unwrap_or(0)
}

#[test]
fn levels_count_like_the_outline() {
    let (d, premises) = chain(3);
    assert_eq!(d.depth_of(Item::Box(d.root)), Some(0));
    assert_eq!(d.depth_of(Item::Box(premises[0])), Some(1));
    assert_eq!(d.depth_of(Item::Box(premises[2])), Some(3));
    assert_eq!(d.height(Item::Box(d.root)), 3);
    let outline = crate::outline::build_outline(&d, &Default::default());
    assert_eq!(outline.last().map(|l| l.depth), Some(3));
}

#[test]
fn adding_past_the_deepest_level_is_refused() {
    let (mut d, premises) = chain(MAX_DEPTH);
    let last = *premises.last().unwrap();
    assert_eq!(
        d.add_card(Parent::Box(last), CardKind::Objection, "x"),
        Err(EditError::TooDeep)
    );
    assert_eq!(
        d.add_box(Parent::Box(last), BoxKind::Note, "x"),
        Err(EditError::TooDeep)
    );
    // One level up still takes a card, and a premise shares its card's level.
    let up = premises[MAX_DEPTH - 2];
    let (card, _) = d.add_card(Parent::Box(up), CardKind::Reason, "x").unwrap();
    assert!(d.add_premise(card, None, "co").is_ok());
    assert_eq!(deepest(&d), MAX_DEPTH);
}

#[test]
fn moves_and_pastes_that_reach_too_deep_are_refused() {
    let (mut d, premises) = chain(MAX_DEPTH);
    let last = *premises.last().unwrap();
    // A loose two-level branch fits nowhere below the last premise.
    let loose = d.add_loose(crate::edit::NewLoose::Card(CardKind::Reason), "a");
    let Item::Card(loose_card) = loose else {
        unreachable!()
    };
    let loose_premise = d.cards[&loose_card].premises[0];
    d.add_card(Parent::Box(loose_premise), CardKind::Reason, "b")
        .unwrap();
    let before = d.clone();
    assert_eq!(
        d.move_item(loose, Parent::Box(last), None),
        Err(EditError::TooDeep)
    );
    assert_eq!(d, before, "a refused move changes nothing");
    let fragment = d.copy_fragment(loose).unwrap();
    assert_eq!(fragment.height(), 1);
    assert_eq!(
        d.paste_fragment(&fragment, Parent::Box(last)),
        Err(EditError::TooDeep)
    );
    // A premise moved into a card keeps what hangs under it.
    assert_eq!(
        d.move_item(
            Item::Box(loose_premise),
            Parent::Card(card_of(&d, last)),
            None
        ),
        Err(EditError::TooDeep)
    );
    assert_eq!(d, before);
    // One level up both fit, and nothing goes past the deepest level.
    let up = premises[MAX_DEPTH - 3];
    d.paste_fragment(&fragment, Parent::Box(up)).unwrap();
    d.move_item(loose, Parent::Box(up), None).unwrap();
    assert_eq!(deepest(&d), MAX_DEPTH);
}

fn card_of(d: &Document, premise: NodeId) -> CardId {
    d.cards
        .values()
        .find(|c| c.premises.contains(&premise))
        .unwrap()
        .id
}

#[test]
fn parking_too_deep_branches_splits_a_long_chain() {
    let levels = 3 * MAX_DEPTH + 7;
    let (mut d, premises) = chain(levels);
    assert!(d.park_too_deep());
    d.validate().unwrap();
    assert_eq!(deepest(&d), MAX_DEPTH);
    assert_eq!(d.parking.len(), 3, "split every {MAX_DEPTH} levels");
    // Every box is still there, in its order.
    assert_eq!(d.boxes.len(), levels + 1);
    let parked = d.parking[0];
    assert_eq!(d.depth_of(parked), Some(0));
    assert_eq!(
        d.cards[&card_of(&d, premises[MAX_DEPTH])].id,
        match parked {
            Item::Card(c) => c,
            Item::Box(_) => unreachable!(),
        }
    );
    assert!(!d.park_too_deep(), "nothing left to move");
}

#[test]
fn parking_leaves_a_map_within_the_limit_alone() {
    let (mut d, _) = chain(MAX_DEPTH);
    let before = d.clone();
    assert!(!d.park_too_deep());
    assert_eq!(d, before);
}
