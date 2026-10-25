use super::*;
use reasoncraft_core::CardKind;

#[test]
fn tab_adds_reason() {
    assert_eq!(
        key_to_action(Key::Tab, Modifiers::NONE, false),
        Some(Action::AddReason)
    );
    assert_eq!(
        key_to_action(Key::Tab, Modifiers::SHIFT, false),
        Some(Action::AddObjection)
    );
    assert_eq!(
        key_to_action(Key::Enter, Modifiers::COMMAND, false),
        Some(Action::AddCoPremise)
    );
    assert_eq!(
        key_to_action(Key::F2, Modifiers::NONE, false),
        Some(Action::Edit)
    );
    assert_eq!(
        key_to_action(Key::Delete, Modifiers::SHIFT, false),
        Some(Action::DeleteBranch)
    );
    assert_eq!(
        key_to_action(Key::ArrowLeft, Modifiers::NONE, false),
        Some(Action::Navigate(Dir::Left))
    );
    let cmd = Modifiers::COMMAND;
    assert_eq!(key_to_action(Key::Z, cmd, false), Some(Action::Undo));
    assert_eq!(key_to_action(Key::Y, cmd, false), Some(Action::Redo));
    assert_eq!(
        key_to_action(Key::Z, cmd | Modifiers::SHIFT, false),
        Some(Action::Redo)
    );
    assert_eq!(key_to_action(Key::S, cmd, false), Some(Action::Save));
    let cmd_shift = cmd | Modifiers::SHIFT;
    assert_eq!(
        key_to_action(Key::Tab, cmd_shift, false),
        Some(Action::TargetWholeCard(CardKind::Objection))
    );
    assert_eq!(
        key_to_action(Key::R, cmd_shift, false),
        Some(Action::TargetWholeCard(CardKind::Reason))
    );
    assert_eq!(
        key_to_action(Key::P, cmd_shift, false),
        Some(Action::JumpParking)
    );
    assert_eq!(key_to_action(Key::Tab, cmd, false), None);
    assert_eq!(key_to_action(Key::R, cmd, false), None);
    assert_eq!(key_to_action(Key::Num0, cmd, false), Some(Action::Zoom100));
    assert_eq!(key_to_action(Key::Num1, cmd, false), Some(Action::ZoomFit));
    assert_eq!(key_to_action(Key::Equals, cmd, false), Some(Action::ZoomIn));
    assert_eq!(key_to_action(Key::Minus, cmd, false), Some(Action::ZoomOut));
    // Plain letters are text, not commands; Ctrl alone (not Cmd) is no command on macOS.
    assert_eq!(key_to_action(Key::Z, Modifiers::NONE, false), None);
    assert_eq!(key_to_action(Key::Z, Modifiers::MAC_CMD, false), None);
}

#[test]
fn typing_mode_only_intercepts_esc_tab_ctrl_enter_and_file_keys() {
    let none = Modifiers::NONE;
    let cmd = Modifiers::COMMAND;
    assert_eq!(key_to_action(Key::S, cmd, true), Some(Action::Save));
    assert_eq!(
        key_to_action(Key::S, cmd | Modifiers::SHIFT, true),
        Some(Action::SaveAs)
    );
    assert_eq!(key_to_action(Key::O, cmd, true), Some(Action::Open));
    assert_eq!(key_to_action(Key::N, cmd, true), Some(Action::New));
    assert_eq!(
        key_to_action(Key::Escape, none, true),
        Some(Action::StopEditing)
    );
    assert_eq!(key_to_action(Key::Tab, none, true), Some(Action::AddReason));
    assert_eq!(
        key_to_action(Key::Enter, Modifiers::COMMAND, true),
        Some(Action::AddCoPremise)
    );
    let cmd_shift = cmd | Modifiers::SHIFT;
    assert_eq!(
        key_to_action(Key::Tab, cmd_shift, true),
        Some(Action::TargetWholeCard(CardKind::Objection))
    );
    assert_eq!(
        key_to_action(Key::R, cmd_shift, true),
        Some(Action::TargetWholeCard(CardKind::Reason))
    );
    for (key, mods) in [
        (Key::Enter, none),
        (Key::F2, none),
        (Key::Delete, none),
        (Key::Delete, Modifiers::SHIFT),
        (Key::ArrowUp, none),
        (Key::ArrowLeft, none),
        (Key::Z, Modifiers::COMMAND),
        (Key::Y, Modifiers::COMMAND),
        (Key::S, none),
        (Key::Num0, Modifiers::COMMAND),
        (Key::P, cmd_shift),
        (Key::R, none),
        (Key::A, none),
    ] {
        assert_eq!(key_to_action(key, mods, true), None, "{key:?} {mods:?}");
    }
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
    let parked = d.add_loose(reasoncraft_core::NewLoose::Card(CardKind::Reason), "p");
    let Item::Card(pc) = parked else {
        panic!("a loose card")
    };
    let pp = d.cards[&pc].premises[0];
    let (_, deep) = d
        .add_card(Parent::Box(pp), CardKind::Reason, "deep")
        .unwrap();
    d.add_loose(reasoncraft_core::NewLoose::Note, "n");
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

#[test]
fn alt_digits_map_to_rate() {
    use crate::actions::Rate;
    let alt = Modifiers::ALT;
    for (key, rate) in [
        (Key::Num1, Rate::High),
        (Key::Num2, Rate::Mid),
        (Key::Num3, Rate::Low),
        (Key::Num0, Rate::Clear),
    ] {
        assert_eq!(key_to_action(key, alt, false), Some(Action::Rate(rate)));
        // Not while editing: the editor gets the keys.
        assert_eq!(key_to_action(key, alt, true), None);
    }
}

#[test]
fn alt_other_keys_unmapped() {
    let alt = Modifiers::ALT;
    for key in [
        Key::Num4,
        Key::Num9,
        Key::Tab,
        Key::Enter,
        Key::Delete,
        Key::Z,
        Key::S,
        Key::F4,
    ] {
        assert_eq!(key_to_action(key, alt, false), None, "{key:?}");
    }
    // Alt with Ctrl or Shift is not a rating (AltGr reports Ctrl+Alt).
    for mods in [alt | Modifiers::COMMAND, alt | Modifiers::SHIFT] {
        assert_eq!(key_to_action(Key::Num1, mods, false), None, "{mods:?}");
    }
}
