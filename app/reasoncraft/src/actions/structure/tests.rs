use reasoncraft_core::{BoxKind, CardId, CardKind, Document, Item, NewLoose, Parent, SourceType};

use crate::actions::{Action, Dir, apply, is_enabled};
use crate::app::ReasonCraftApp;
use crate::doc_tab::DocTab;

fn app_with(doc: Document) -> ReasonCraftApp {
    let mut a = ReasonCraftApp::new_for_test();
    a.tabs[0] = DocTab::from_document(doc);
    a
}

fn tab(a: &ReasonCraftApp) -> &DocTab {
    &a.tabs[a.active]
}

/// The root with three reason cards a, b, c.
fn three_cards() -> (Document, [CardId; 3]) {
    let mut d = Document::new("Claim");
    let root = d.root;
    let mut card = |t| {
        d.add_card(Parent::Box(root), CardKind::Reason, t)
            .unwrap()
            .0
    };
    let ids = [card("a"), card("b"), card("c")];
    (d, ids)
}

fn root_cards(a: &ReasonCraftApp) -> Vec<Item> {
    let t = tab(a);
    t.doc.boxes[&t.doc.root].children.clone()
}

#[test]
fn reorder_with_alt_arrows() {
    let (d, [ca, cb, cc]) = three_cards();
    let mut a = app_with(d);
    let card = Item::Card;
    a.tabs[0].selection = Some(card(ca));
    apply(&mut a, Action::Reorder(Dir::Right));
    assert_eq!(root_cards(&a), vec![card(cb), card(ca), card(cc)]);
    assert_eq!(tab(&a).selection, Some(card(ca)), "keeps the selection");
    assert_eq!(tab(&a).history.undo.len(), 1);
    apply(&mut a, Action::Reorder(Dir::Right));
    assert_eq!(root_cards(&a), vec![card(cb), card(cc), card(ca)]);
    // At the last place: no change, no history, and the command is greyed out.
    assert!(!is_enabled(tab(&a), &Action::Reorder(Dir::Right)));
    apply(&mut a, Action::Reorder(Dir::Right));
    assert_eq!(root_cards(&a), vec![card(cb), card(cc), card(ca)]);
    assert_eq!(tab(&a).history.undo.len(), 2);
    apply(&mut a, Action::Reorder(Dir::Left));
    assert_eq!(root_cards(&a), vec![card(cb), card(ca), card(cc)]);
    // Up and Down are not reorders; the root never moves.
    assert!(!is_enabled(tab(&a), &Action::Reorder(Dir::Up)));
    a.tabs[0].selection = Some(Item::Box(tab(&a).doc.root));
    assert!(!is_enabled(tab(&a), &Action::Reorder(Dir::Left)));
}

/// A card's only premise moves its card; a co-premise moves within its card.
#[test]
fn reorder_premises_and_sole_premise_cards() {
    let (mut d, [ca, cb, _]) = three_cards();
    let a1 = d.cards[&ca].premises[0];
    let a2 = d.add_premise(ca, None, "a2").unwrap();
    let b1 = d.cards[&cb].premises[0];
    let mut a = app_with(d);
    a.tabs[0].selection = Some(Item::Box(a1));
    apply(&mut a, Action::Reorder(Dir::Right));
    assert_eq!(tab(&a).doc.cards[&ca].premises, vec![a2, a1]);
    assert_eq!(root_cards(&a)[0], Item::Card(ca), "the card stays put");
    assert!(!is_enabled(tab(&a), &Action::Reorder(Dir::Right)));
    a.tabs[0].selection = Some(Item::Box(b1));
    apply(&mut a, Action::Reorder(Dir::Left));
    assert_eq!(root_cards(&a)[0], Item::Card(cb));
    assert_eq!(tab(&a).selection, Some(Item::Box(b1)));
}

/// Under a box the layout shows sources and notes first, so a card steps
/// over them to its next card sibling, and a source only among attachments.
#[test]
fn reorder_steps_within_display_group() {
    let mut d = Document::new("Claim");
    let root = d.root;
    let (ca, _) = d
        .add_card(Parent::Box(root), CardKind::Reason, "a")
        .unwrap();
    let src = d
        .add_box(
            Parent::Box(root),
            BoxKind::Source {
                source_type: SourceType::Web,
            },
            "s",
        )
        .unwrap();
    let (cb, _) = d
        .add_card(Parent::Box(root), CardKind::Reason, "b")
        .unwrap();
    let note = d.add_box(Parent::Box(root), BoxKind::Note, "n").unwrap();
    let mut a = app_with(d);
    a.tabs[0].selection = Some(Item::Card(ca));
    apply(&mut a, Action::Reorder(Dir::Right));
    let order = root_cards(&a);
    let pos = |i| order.iter().position(|&x| x == i).unwrap();
    assert!(pos(Item::Card(cb)) < pos(Item::Card(ca)));
    a.tabs[0].selection = Some(Item::Box(src));
    apply(&mut a, Action::Reorder(Dir::Right));
    let order = root_cards(&a);
    let pos = |i| order.iter().position(|&x| x == i).unwrap();
    assert!(pos(Item::Box(note)) < pos(Item::Box(src)));
    assert!(!is_enabled(tab(&a), &Action::Reorder(Dir::Right)));
}

#[test]
fn move_with_index_reorders_and_same_place_is_disabled() {
    let (d, [ca, cb, cc]) = three_cards();
    let root = d.root;
    let mut a = app_with(d);
    let mv = |item, index| Action::Move {
        item: Item::Card(item),
        to: Parent::Box(root),
        index,
    };
    assert!(!is_enabled(tab(&a), &mv(cb, Some(1))), "same place");
    assert!(!is_enabled(tab(&a), &mv(cb, None)), "same parent, no index");
    assert!(is_enabled(tab(&a), &mv(cc, Some(0))));
    apply(&mut a, mv(cc, Some(0)));
    assert_eq!(
        root_cards(&a),
        vec![Item::Card(cc), Item::Card(ca), Item::Card(cb)]
    );
}

#[test]
fn connect_mode_starts_for_parked_items_and_ends_on_move() {
    let (mut d, [ca, ..]) = three_cards();
    let root = d.root;
    let Item::Card(parked) = d.add_loose(NewLoose::Card(CardKind::Objection), "q") else {
        unreachable!()
    };
    let q = d.cards[&parked].premises[0];
    let (_, deep) = d
        .add_card(Parent::Box(q), CardKind::Reason, "deep")
        .unwrap();
    let mut a = app_with(d);
    // Only top-level parking items (or the premise of a parked card).
    for (sel, ok) in [
        (Item::Box(root), false),
        (Item::Card(ca), false),
        (Item::Box(deep), false),
        (Item::Card(parked), true),
        (Item::Box(q), true),
    ] {
        a.tabs[0].selection = Some(sel);
        assert_eq!(is_enabled(tab(&a), &Action::StartConnect), ok, "{sel:?}");
    }
    apply(&mut a, Action::StartConnect);
    assert_eq!(tab(&a).connect_mode, Some(Item::Card(parked)));
    assert_eq!(tab(&a).selection, Some(Item::Box(root)), "picks in the map");
    // Navigating keeps the mode; a refused target changes nothing.
    apply(&mut a, Action::Navigate(Dir::Down));
    assert_eq!(tab(&a).connect_mode, Some(Item::Card(parked)));
    let ca1 = tab(&a).doc.cards[&ca].premises[0];
    let to_premise = Action::Move {
        item: Item::Card(parked),
        to: Parent::Box(ca1),
        index: None,
    };
    apply(&mut a, to_premise);
    assert_eq!(tab(&a).connect_mode, None);
    assert!(tab(&a).doc.parking.is_empty());
    assert_eq!(tab(&a).doc.boxes[&ca1].children, vec![Item::Card(parked)]);
    assert_eq!(tab(&a).history.undo.len(), 1);
}

#[test]
fn connect_mode_cancel_and_other_commands_end_it() {
    let mut d = Document::new("Claim");
    let note = d.add_loose(NewLoose::Note, "n");
    let mut a = app_with(d);
    for end in [Action::CancelConnect, Action::Delete, Action::Undo] {
        a.tabs[0].selection = Some(note);
        apply(&mut a, Action::StartConnect);
        assert_eq!(tab(&a).connect_mode, Some(note));
        apply(&mut a, end.clone());
        assert_eq!(tab(&a).connect_mode, None, "{end:?}");
        assert_eq!(tab(&a).doc.parking, vec![note], "{end:?}");
    }
    assert!(tab(&a).history.undo.is_empty());
}

#[test]
fn connect_mode_ends_on_app_level_actions() {
    let mut d = Document::new("Claim");
    let note = d.add_loose(NewLoose::Note, "n");
    let mut a = app_with(d);
    let insert = Action::Insert {
        what: crate::actions::Insertable::Reason,
        to: None,
        on: None,
    };
    for end in [
        insert,
        Action::CommentSelected,
        Action::EditLink,
        Action::ActivateTab(0),
    ] {
        a.tabs[0].selection = Some(note);
        apply(&mut a, Action::StartConnect);
        assert_eq!(tab(&a).connect_mode, Some(note));
        apply(&mut a, end.clone());
        assert_eq!(tab(&a).connect_mode, None, "{end:?}");
        assert_eq!(tab(&a).doc.parking, vec![note], "{end:?}");
        a.modal = None;
        a.link_edit = None;
    }
    // Picking a target (selecting, zooming) still keeps the mode.
    a.tabs[0].selection = Some(note);
    apply(&mut a, Action::StartConnect);
    apply(&mut a, Action::ZoomIn);
    apply(&mut a, Action::Select(None));
    assert_eq!(tab(&a).connect_mode, Some(note));
}

/// Children [cardA, source, cardB] show as [source, cardA, cardB]: moving
/// cardA to just after the source changes only hidden document order.
#[test]
fn move_that_only_changes_hidden_order_is_disabled() {
    let mut d = Document::new("Claim");
    let root = d.root;
    let ca = d
        .add_card(Parent::Box(root), CardKind::Reason, "a")
        .unwrap()
        .0;
    let src = d
        .add_box(
            Parent::Box(root),
            BoxKind::Source {
                source_type: SourceType::Web,
            },
            "s",
        )
        .unwrap();
    let cb = d
        .add_card(Parent::Box(root), CardKind::Reason, "b")
        .unwrap()
        .0;
    let a = app_with(d);
    assert_eq!(
        root_cards(&a),
        vec![Item::Card(ca), Item::Box(src), Item::Card(cb)]
    );
    let mv = |item, index| Action::Move {
        item,
        to: Parent::Box(root),
        index: Some(index),
    };
    // cardA to index 1 (after the source, before cardB): shown the same.
    assert!(!is_enabled(tab(&a), &mv(Item::Card(ca), 1)));
    // The source before cardA: sources show first anyway.
    assert!(!is_enabled(tab(&a), &mv(Item::Box(src), 0)));
    // cardA after cardB: a visible change.
    assert!(is_enabled(tab(&a), &mv(Item::Card(ca), 2)));
}

/// The inline editor losing focus (to the ribbon's "Connect to…") must not
/// end the mode it just started; the edit is committed first.
#[test]
fn stop_editing_keeps_connect_mode() {
    let mut d = Document::new("Claim");
    let note = d.add_loose(NewLoose::Note, "n");
    let Item::Box(nb) = note else { unreachable!() };
    let mut a = app_with(d);
    a.tabs[0].selection = Some(note);
    apply(&mut a, Action::Edit);
    apply(
        &mut a,
        Action::EditText {
            node: nb,
            text: "typed".into(),
        },
    );
    apply(&mut a, Action::StartConnect);
    assert_eq!(tab(&a).editing, None);
    assert_eq!(tab(&a).history.undo.len(), 1, "edit committed");
    apply(&mut a, Action::StopEditing);
    assert_eq!(tab(&a).connect_mode, Some(note));
    assert_eq!(tab(&a).doc.boxes[&nb].text, "typed");
}
