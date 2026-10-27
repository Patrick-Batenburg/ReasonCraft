use super::*;
use reasoncraft_core::{CardKind, ClearScope};

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
        assert_eq!(
            key_to_action(Os::Windows, key, alt, false),
            Some(Action::Rate(rate))
        );
        // Not while editing: the editor gets the keys.
        assert_eq!(key_to_action(Os::Windows, key, alt, true), None);
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
        assert_eq!(key_to_action(Os::Windows, key, alt, false), None, "{key:?}");
    }
    // Alt with Ctrl or Shift is not a rating (AltGr reports Ctrl+Alt).
    for mods in [alt | Modifiers::COMMAND, alt | Modifiers::SHIFT] {
        assert_eq!(
            key_to_action(Os::Windows, Key::Num1, mods, false),
            None,
            "{mods:?}"
        );
    }
}

#[test]
fn alt_shift_0_clears_the_whole_map() {
    let map = Some(Action::ClearEvaluation(ClearScope::Map));
    let alt_shift = Modifiers::ALT | Modifiers::SHIFT;
    assert_eq!(key_to_action(Os::Windows, Key::Num0, alt_shift, false), map);
    assert_eq!(
        key_to_action(Os::Windows, Key::Num0, alt_shift, true),
        None,
        "editing"
    );
    // Not with Ctrl: AltGr reports Ctrl+Alt.
    for mods in [alt_shift | Modifiers::CTRL, alt_shift | Modifiers::COMMAND] {
        assert_eq!(
            key_to_action(Os::Windows, Key::Num0, mods, false),
            None,
            "{mods:?}"
        );
    }
    for key in [Key::Num1, Key::ArrowLeft, Key::Equals] {
        assert_eq!(
            key_to_action(Os::Windows, key, alt_shift, false),
            None,
            "{key:?}"
        );
    }
    // Shift+0 types a symbol ("=" on a German layout): the physical key counts.
    for typed in [Key::Equals, Key::Num0] {
        let key = shortcut_key(typed, Some(Key::Num0), alt_shift);
        assert_eq!(key, Some(Key::Num0), "{typed:?}");
        assert_eq!(
            key_to_action(Os::Windows, key.unwrap(), alt_shift, false),
            map
        );
    }
    // Without Alt+Shift the logical key counts, and Alt-codes are dropped.
    let alt = Modifiers::ALT;
    assert_eq!(
        shortcut_key(Key::Num1, Some(Key::Num1), alt),
        Some(Key::Num1)
    );
    assert_eq!(shortcut_key(Key::End, Some(Key::Num1), alt), None);
    let shift = Modifiers::SHIFT;
    assert_eq!(
        shortcut_key(Key::Tab, Some(Key::Tab), shift),
        Some(Key::Tab)
    );
}

#[test]
fn ctrl_shift_e_adds_source() {
    use reasoncraft_core::SourceType;
    let cmd_shift = Modifiers::COMMAND | Modifiers::SHIFT;
    let add = Some(Action::AddSource(SourceType::Source));
    assert_eq!(key_to_action(Os::Windows, Key::E, cmd_shift, false), add);
    assert_eq!(key_to_action(Os::Windows, Key::E, cmd_shift, true), add);
    assert_eq!(
        key_to_action(Os::Windows, Key::E, Modifiers::COMMAND, false),
        None
    );
    assert_eq!(
        key_to_action(Os::Windows, Key::E, Modifiers::COMMAND, true),
        None
    );
}

#[test]
fn ctrl_shift_m_opens_comments() {
    let cmd_shift = Modifiers::COMMAND | Modifiers::SHIFT;
    assert_eq!(
        key_to_action(Os::Windows, Key::M, cmd_shift, false),
        Some(Action::CommentSelected)
    );
    // While editing, M is text (no command).
    assert_eq!(key_to_action(Os::Windows, Key::M, cmd_shift, true), None);
    assert_eq!(
        key_to_action(Os::Windows, Key::M, Modifiers::COMMAND, false),
        None
    );
}

#[test]
fn ctrl_shift_o_targets_whole_card_objection_and_ctrl_shift_tab_unmapped() {
    let cmd_shift = Modifiers::COMMAND | Modifiers::SHIFT;
    let objection = Some(Action::TargetWholeCard(CardKind::Objection));
    for editing in [false, true] {
        assert_eq!(
            key_to_action(Os::Windows, Key::O, cmd_shift, editing),
            objection
        );
        assert_eq!(
            key_to_action(Os::Windows, Key::Tab, cmd_shift, editing),
            None
        );
        // Ctrl+Shift+R stays the whole-card reason.
        assert_eq!(
            key_to_action(Os::Windows, Key::R, cmd_shift, editing),
            Some(Action::TargetWholeCard(CardKind::Reason))
        );
    }
    // Ctrl+O is still Open, in both modes.
    assert_eq!(
        key_to_action(Os::Windows, Key::O, Modifiers::COMMAND, false),
        Some(Action::Open)
    );
    assert_eq!(
        key_to_action(Os::Windows, Key::O, Modifiers::COMMAND, true),
        Some(Action::Open)
    );
}

#[test]
fn alt_arrows_reorder_and_ctrl_shift_k_connects() {
    let alt = Modifiers::ALT;
    assert_eq!(
        key_to_action(Os::Windows, Key::ArrowLeft, alt, false),
        Some(Action::Reorder(Dir::Left))
    );
    assert_eq!(
        key_to_action(Os::Windows, Key::ArrowRight, alt, false),
        Some(Action::Reorder(Dir::Right))
    );
    for key in [
        Key::ArrowLeft,
        Key::ArrowRight,
        Key::ArrowUp,
        Key::ArrowDown,
    ] {
        assert_eq!(
            key_to_action(Os::Windows, key, alt, true),
            None,
            "not while editing"
        );
    }
    assert_eq!(
        key_to_action(Os::Windows, Key::ArrowUp, alt, false),
        Some(Action::Reorder(Dir::Up))
    );
    assert_eq!(
        key_to_action(Os::Windows, Key::ArrowDown, alt, false),
        Some(Action::Reorder(Dir::Down))
    );
    assert_eq!(
        key_to_action(Os::Windows, Key::ArrowLeft, alt | Modifiers::SHIFT, false),
        None
    );
    let cmd_shift = Modifiers::COMMAND | Modifiers::SHIFT;
    assert_eq!(
        key_to_action(Os::Windows, Key::K, cmd_shift, false),
        Some(Action::StartConnect)
    );
    assert_eq!(key_to_action(Os::Windows, Key::K, cmd_shift, true), None);
}

/// AltGr reports Ctrl+Alt: never a rating, so the character it types is text.
#[test]
fn altgr_digits_are_not_ratings() {
    let altgr = Modifiers::CTRL | Modifiers::ALT;
    for key in [Key::Num0, Key::Num1, Key::Num2, Key::Num3, Key::ArrowLeft] {
        assert_eq!(
            key_to_action(Os::Windows, key, altgr, false),
            None,
            "{key:?}"
        );
    }
}

/// Windows Alt-codes with NumLock off: the numpad digit arrives as a
/// navigation key, which must not reorder or do anything else.
#[test]
fn numpad_alt_codes_are_not_commands() {
    assert!(is_alt_code_key(Key::ArrowLeft, Some(Key::Num4)));
    assert!(is_alt_code_key(Key::End, Some(Key::Num1)));
    assert!(is_alt_code_key(Key::Insert, Some(Key::Num0)));
    // The arrow keys themselves, and top-row digits, are not.
    assert!(!is_alt_code_key(Key::ArrowLeft, Some(Key::ArrowLeft)));
    assert!(!is_alt_code_key(Key::Num1, Some(Key::Num1)));
    assert!(!is_alt_code_key(Key::ArrowLeft, None));
}
