//! The per-platform shortcut tables: macOS's keys, and how they are shown.

use std::collections::BTreeSet;

use super::*;

const ALL: [Os; 3] = [Os::Windows, Os::Mac, Os::Linux];

/// Opt with a letter or digit types a character on macOS (Opt+1 is "¡",
/// Opt+Q "œ"), so the macOS table has no such shortcut without Cmd.
#[test]
fn mac_table_has_no_alt_letter_or_digit() {
    for (s, _) in shortcuts(Os::Mac) {
        let opt = s.mods.alt && !s.mods.command;
        assert!(!(opt && types_with_opt(s.key)), "{s:?}");
    }
    // They moved to Control, which is free on macOS.
    let ctrl = Modifiers::CTRL;
    assert_eq!(
        key_to_action(Os::Mac, Key::Num1, ctrl, false),
        Some(Action::Rate(crate::actions::Rate::High))
    );
    assert_eq!(
        key_to_action(Os::Mac, Key::Num0, ctrl | Modifiers::SHIFT, false),
        Some(Action::ClearEvaluation(reasoncraft_core::ClearScope::Map))
    );
    // Opt+digit is text, as is Opt+Q; Search commands is Cmd+Shift+A, also
    // while editing.
    assert_eq!(
        key_to_action(Os::Mac, Key::Num1, Modifiers::ALT, false),
        None
    );
    assert_eq!(key_to_action(Os::Mac, Key::Q, Modifiers::ALT, true), None);
    let cmd_shift = Modifiers::MAC_CMD | Modifiers::COMMAND | Modifiers::SHIFT;
    for editing in [false, true] {
        assert_eq!(
            key_to_action(Os::Mac, Key::A, cmd_shift, editing),
            Some(Action::OpenCommandSearch)
        );
    }
    // Opt with the arrows stays (Control with them switches desktops).
    assert_eq!(
        key_to_action(Os::Mac, Key::ArrowLeft, Modifiers::ALT, false),
        Some(Action::Reorder(Dir::Left))
    );
    // Cmd+1 is still Zoom to fit, not a rating.
    let cmd = Modifiers::MAC_CMD | Modifiers::COMMAND;
    assert_eq!(
        key_to_action(Os::Mac, Key::Num1, cmd, false),
        Some(Action::ZoomFit)
    );
    assert_eq!(key_to_action(Os::Mac, Key::Num1, cmd | ctrl, false), None);
}

/// Cmd+H hides the app on macOS: Replace is Cmd+Opt+F there, and works
/// while editing like Ctrl+H elsewhere.
#[test]
fn mac_replace_is_cmd_alt_f() {
    let replace = Action::OpenFind { replace: true };
    let s = shortcut_for(&replace, Os::Mac).unwrap();
    assert_eq!(
        (s.key, s.mods),
        (Key::F, Modifiers::COMMAND | Modifiers::ALT)
    );
    let pressed = Modifiers::MAC_CMD | Modifiers::COMMAND | Modifiers::ALT;
    for editing in [false, true] {
        assert_eq!(
            key_to_action(Os::Mac, Key::F, pressed, editing),
            Some(replace.clone())
        );
    }
    let cmd = Modifiers::MAC_CMD | Modifiers::COMMAND;
    assert_eq!(key_to_action(Os::Mac, Key::H, cmd, false), None);
    // Elsewhere Ctrl+H stays, and Ctrl+Alt+F (AltGr+F) is nothing.
    let ctrl = Modifiers::CTRL | Modifiers::COMMAND;
    assert_eq!(
        key_to_action(Os::Windows, Key::H, ctrl, false),
        Some(replace)
    );
    let altgr = ctrl | Modifiers::ALT;
    assert_eq!(key_to_action(Os::Windows, Key::F, altgr, false), None);
}

/// No key combination stands for two entries of one table.
#[test]
fn tables_have_unique_shortcuts_per_os() {
    for os in ALL {
        let mut seen = BTreeSet::new();
        for (s, _) in shortcuts(os) {
            let text = format_shortcut(*s, os, false, "en");
            assert!(seen.insert(text.clone()), "{os:?}: {text} twice");
        }
        assert_eq!(seen.len(), shortcuts(os).len());
    }
    // Windows and Linux share one table; macOS changes only what it must.
    assert!(std::ptr::eq(shortcuts(Os::Windows), shortcuts(Os::Linux)));
    let changed = shortcuts(Os::Windows)
        .iter()
        .zip(shortcuts(Os::Mac))
        .filter(|((a, _), (b, _))| a != b)
        .count();
    assert_eq!(changed, 7, "5 Alt+digit, Alt+Q and Ctrl+H");
}

/// Backspace, like Delete, deletes the selected box on macOS, but not
/// while editing (then it is text editing) and not elsewhere.
#[test]
fn backspace_deletes_on_mac() {
    let none = Modifiers::NONE;
    assert_eq!(
        key_to_action(Os::Mac, Key::Backspace, none, false),
        Some(Action::Delete)
    );
    assert_eq!(key_to_action(Os::Mac, Key::Backspace, none, true), None);
    for os in [Os::Windows, Os::Linux] {
        assert_eq!(key_to_action(os, Key::Backspace, none, false), None);
    }
    assert_eq!(
        key_to_action(Os::Mac, Key::Delete, none, false),
        Some(Action::Delete)
    );
}

/// The macOS symbols are shown only if the UI font has them; otherwise the
/// modifiers are written out.
#[test]
fn mac_glyphs_only_if_font_has_them() {
    // Ask egui's own fonts (the ones the UI draws with).
    let ctx = eframe::egui::Context::default();
    ctx.run_ui(Default::default(), |_| {})
        .textures_delta
        .clear();
    let font = eframe::egui::FontId::proportional(14.0);
    let has = ctx.fonts_mut(|f| f.has_glyphs(&font, MAC_SYMBOLS));
    assert_eq!(*MAC_GLYPHS, has);

    let s = shortcut_for(&Action::SaveAs, Os::Mac).unwrap();
    let expected = if has { "⇧⌘S" } else { "Shift+Cmd+S" };
    assert_eq!(shortcut_text(s, Os::Mac), expected);
    assert_eq!(format_shortcut(s, Os::Mac, false, "en"), "Shift+Cmd+S");
    assert_eq!(format_shortcut(s, Os::Mac, true, "en"), "⇧⌘S");
    let rate = shortcut_for(&Action::Rate(crate::actions::Rate::Low), Os::Mac).unwrap();
    assert_eq!(format_shortcut(rate, Os::Mac, true, "en"), "⌃3");
    // Never on the other platforms.
    let s = shortcut_for(&Action::SaveAs, Os::Windows).unwrap();
    assert_eq!(shortcut_text(s, Os::Windows), "Ctrl+Shift+S");
}
