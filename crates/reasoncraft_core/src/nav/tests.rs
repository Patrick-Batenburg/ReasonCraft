use super::*;
use crate::CardKind;

/// Every arrow maps to a distinct direction, top-down keeps them all, and
/// the parent (`Up`) is on the screen side the parent is drawn on.
#[test]
fn nav_dir_maps_screen_arrows() {
    use crate::LayoutDirection as L;
    let all = [Dir::Up, Dir::Down, Dir::Left, Dir::Right];
    for layout in L::ALL {
        let mapped: Vec<Dir> = all.iter().map(|&d| nav_dir(d, layout)).collect();
        for d in all {
            assert!(mapped.contains(&d), "{layout:?} misses {d:?}");
        }
        let to_parent = all
            .iter()
            .find(|&&d| nav_dir(d, layout) == Dir::Up)
            .unwrap();
        let screen = match to_parent {
            Dir::Up => emath::vec2(0.0, -1.0),
            Dir::Down => emath::vec2(0.0, 1.0),
            Dir::Left => emath::vec2(-1.0, 0.0),
            Dir::Right => emath::vec2(1.0, 0.0),
        };
        assert_eq!(screen, layout.toward_parent(), "{layout:?}");
    }
    for d in all {
        assert_eq!(nav_dir(d, L::TopDown), d);
    }
    assert_eq!(
        nav_dir(Dir::Up, L::LeftRight),
        Dir::Left,
        "previous sibling"
    );
    assert_eq!(nav_dir(Dir::Down, L::LeftRight), Dir::Right, "next sibling");
}

/// root ← card A [a1, a2] ← card C [c1] under a1; root ← card B [b1].
#[test]
fn navigate_up_down_siblings() {
    let mut d = Document::new("Claim");
    let root = d.root;
    let (ca, a1) = d
        .add_card(Parent::Box(root), CardKind::Reason, "a1")
        .unwrap();
    let a2 = d.add_premise(ca, None, "a2").unwrap();
    let (cb, b1) = d
        .add_card(Parent::Box(root), CardKind::Objection, "b1")
        .unwrap();
    let (cc, c1) = d.add_card(Parent::Box(a1), CardKind::Reason, "c1").unwrap();
    let b = Item::Box;
    let nav = |from: Item, dir: Dir| navigate(&d, from, dir);

    // Up: premise → its card; card → the box it hangs from.
    assert_eq!(nav(b(a1), Dir::Up), Some(Item::Card(ca)));
    assert_eq!(nav(Item::Card(ca), Dir::Up), Some(b(root)));
    assert_eq!(nav(b(c1), Dir::Up), Some(Item::Card(cc)));
    assert_eq!(nav(Item::Card(cc), Dir::Up), Some(b(a1)));
    assert_eq!(nav(b(root), Dir::Up), None);

    // Down: a box's first child (a card itself); a card's first premise.
    assert_eq!(nav(b(root), Dir::Down), Some(Item::Card(ca)));
    assert_eq!(nav(Item::Card(ca), Dir::Down), Some(b(a1)));
    assert_eq!(nav(b(a1), Dir::Down), Some(Item::Card(cc)));
    assert_eq!(nav(b(a2), Dir::Down), None);

    // Left/right: co-premises first, then the neighbouring cards.
    assert_eq!(nav(b(a1), Dir::Right), Some(b(a2)));
    assert_eq!(nav(b(a2), Dir::Left), Some(b(a1)));
    assert_eq!(nav(b(a2), Dir::Right), Some(b(b1)));
    assert_eq!(nav(b(b1), Dir::Left), Some(b(a2)));
    assert_eq!(nav(b(b1), Dir::Right), None);
    assert_eq!(nav(b(a1), Dir::Left), None);
    assert_eq!(nav(Item::Card(ca), Dir::Right), Some(Item::Card(cb)));
    assert_eq!(nav(Item::Card(cb), Dir::Left), Some(Item::Card(ca)));
    assert_eq!(nav(Item::Card(ca), Dir::Left), None);
    assert_eq!(nav(b(root), Dir::Right), None);
}

/// Whole-card attackers and parked items are reachable by keyboard.
#[test]
fn navigate_reaches_whole_card_attackers_and_parking() {
    let mut d = Document::new("Claim");
    let root = d.root;
    let (ca, a1) = d
        .add_card(Parent::Box(root), CardKind::Reason, "a1")
        .unwrap();
    let (w, w1) = d
        .add_card(Parent::Card(ca), CardKind::Objection, "w1")
        .unwrap();
    assert_eq!(navigate(&d, Item::Box(w1), Dir::Up), Some(Item::Card(w)));
    assert_eq!(navigate(&d, Item::Card(w), Dir::Up), Some(Item::Card(ca)));
    assert_eq!(navigate(&d, Item::Card(ca), Dir::Down), Some(Item::Box(a1)));

    // Parking: no way up out of it; Ctrl+Shift+P toggles.
    assert_eq!(parking_jump(&d, Some(Item::Box(root))), None);
    let parked = d.add_loose(crate::NewLoose::Card(CardKind::Reason), "p");
    let Item::Card(pc) = parked else {
        panic!("a loose card")
    };
    let pp = d.cards[&pc].premises[0];
    let (_, deep) = d
        .add_card(Parent::Box(pp), CardKind::Reason, "deep")
        .unwrap();
    d.add_loose(crate::NewLoose::Note, "n");
    assert_eq!(navigate(&d, parked, Dir::Up), None);
    assert_eq!(navigate(&d, Item::Box(pp), Dir::Up), Some(parked));
    assert_eq!(parking_jump(&d, None), Some(parked));
    assert_eq!(parking_jump(&d, Some(Item::Box(a1))), Some(parked));
    assert_eq!(parking_jump(&d, Some(Item::Card(w))), Some(parked));
    assert_eq!(parking_jump(&d, Some(parked)), Some(Item::Box(root)));
    assert_eq!(
        parking_jump(&d, Some(Item::Box(deep))),
        Some(Item::Box(root))
    );
}

/// The arrows follow the order the map shows a box's children in (notes
/// and sources first), not the stored order (CORE-R1).
#[test]
fn arrows_follow_the_shown_order() {
    let mut d = Document::new("Claim");
    let root = Parent::Box(d.root);
    let (card, premise) = d.add_card(root, CardKind::Reason, "reason").unwrap();
    let note = d.add_box(root, crate::BoxKind::Note, "note").unwrap();
    let shown = crate::layout::display_children(&d, d.root);
    assert_eq!(
        shown,
        [Item::Box(note), Item::Card(card)],
        "the note shows first"
    );
    let from_root = navigate(&d, Item::Box(d.root), Dir::Down);
    assert_eq!(from_root, Some(Item::Box(note)), "Down: the leftmost child");
    assert_eq!(
        navigate(&d, Item::Card(card), Dir::Left),
        Some(Item::Box(note))
    );
    assert_eq!(
        navigate(&d, Item::Box(premise), Dir::Left),
        Some(Item::Box(note))
    );
    assert_eq!(
        navigate(&d, Item::Box(note), Dir::Right),
        Some(Item::Box(premise))
    );
    assert_eq!(navigate(&d, Item::Card(card), Dir::Right), None);
}
