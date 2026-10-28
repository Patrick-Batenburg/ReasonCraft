use super::*;
use reasoncraft_core::CardKind;

#[test]
fn tab_adds_reason() {
    assert_eq!(
        key_to_action(Os::Windows, Key::Tab, Modifiers::NONE, false),
        Some(Action::AddReason)
    );
    assert_eq!(
        key_to_action(Os::Windows, Key::Tab, Modifiers::SHIFT, false),
        Some(Action::AddObjection)
    );
    assert_eq!(
        key_to_action(Os::Windows, Key::Enter, Modifiers::COMMAND, false),
        Some(Action::AddCoPremise)
    );
    assert_eq!(
        key_to_action(Os::Windows, Key::F2, Modifiers::NONE, false),
        Some(Action::Edit)
    );
    assert_eq!(
        key_to_action(Os::Windows, Key::Delete, Modifiers::SHIFT, false),
        Some(Action::DeleteBranch)
    );
    assert_eq!(
        key_to_action(Os::Windows, Key::ArrowLeft, Modifiers::NONE, false),
        Some(Action::Navigate(Dir::Left))
    );
    let cmd = Modifiers::COMMAND;
    assert_eq!(
        key_to_action(Os::Windows, Key::Z, cmd, false),
        Some(Action::Undo)
    );
    assert_eq!(
        key_to_action(Os::Windows, Key::Y, cmd, false),
        Some(Action::Redo)
    );
    assert_eq!(
        key_to_action(Os::Windows, Key::Z, cmd | Modifiers::SHIFT, false),
        Some(Action::Redo)
    );
    assert_eq!(
        key_to_action(Os::Windows, Key::S, cmd, false),
        Some(Action::Save)
    );
    let cmd_shift = cmd | Modifiers::SHIFT;
    assert_eq!(
        key_to_action(Os::Windows, Key::O, cmd_shift, false),
        Some(Action::TargetWholeCard(CardKind::Objection))
    );
    assert_eq!(
        key_to_action(Os::Windows, Key::R, cmd_shift, false),
        Some(Action::TargetWholeCard(CardKind::Reason))
    );
    assert_eq!(
        key_to_action(Os::Windows, Key::P, cmd_shift, false),
        Some(Action::JumpParking)
    );
    assert_eq!(key_to_action(Os::Windows, Key::Tab, cmd, false), None);
    assert_eq!(key_to_action(Os::Windows, Key::R, cmd, false), None);
    assert_eq!(
        key_to_action(Os::Windows, Key::Num0, cmd, false),
        Some(Action::Zoom100)
    );
    assert_eq!(
        key_to_action(Os::Windows, Key::Num1, cmd, false),
        Some(Action::ZoomFit)
    );
    assert_eq!(
        key_to_action(Os::Windows, Key::Equals, cmd, false),
        Some(Action::ZoomIn)
    );
    assert_eq!(
        key_to_action(Os::Windows, Key::Minus, cmd, false),
        Some(Action::ZoomOut)
    );
    // Plain letters are text, not commands; Ctrl alone (not Cmd) is no command on macOS.
    assert_eq!(
        key_to_action(Os::Windows, Key::Z, Modifiers::NONE, false),
        None
    );
    assert_eq!(
        key_to_action(Os::Windows, Key::Z, Modifiers::MAC_CMD, false),
        None
    );
}

#[test]
fn typing_mode_only_intercepts_esc_tab_ctrl_enter_and_file_keys() {
    let none = Modifiers::NONE;
    let cmd = Modifiers::COMMAND;
    assert_eq!(
        key_to_action(Os::Windows, Key::S, cmd, true),
        Some(Action::Save)
    );
    assert_eq!(
        key_to_action(Os::Windows, Key::S, cmd | Modifiers::SHIFT, true),
        Some(Action::SaveAs)
    );
    assert_eq!(
        key_to_action(Os::Windows, Key::O, cmd, true),
        Some(Action::Open)
    );
    assert_eq!(
        key_to_action(Os::Windows, Key::N, cmd, true),
        Some(Action::New)
    );
    assert_eq!(
        key_to_action(Os::Windows, Key::Escape, none, true),
        Some(Action::StopEditing)
    );
    assert_eq!(
        key_to_action(Os::Windows, Key::Tab, none, true),
        Some(Action::AddReason)
    );
    assert_eq!(
        key_to_action(Os::Windows, Key::Enter, Modifiers::COMMAND, true),
        Some(Action::AddCoPremise)
    );
    let cmd_shift = cmd | Modifiers::SHIFT;
    assert_eq!(
        key_to_action(Os::Windows, Key::O, cmd_shift, true),
        Some(Action::TargetWholeCard(CardKind::Objection))
    );
    assert_eq!(
        key_to_action(Os::Windows, Key::R, cmd_shift, true),
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
        assert_eq!(
            key_to_action(Os::Windows, key, mods, true),
            None,
            "{key:?} {mods:?}"
        );
    }
}
