//! The one table of keyboard shortcuts, and how they are shown.
//!
//! README's keyboard table is checked against [`SHORTCUTS`] by a test.

use eframe::egui::{Key, Modifiers};
use reasoncraft_core::{CardKind, ClearScope, SourceType};

use crate::actions::{Action, Dir, Rate};

/// A key with its modifiers. `Modifiers::command` is Ctrl, or Cmd on macOS.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shortcut {
    pub key: Key,
    pub mods: Modifiers,
}

impl Shortcut {
    const fn new(key: Key, mods: Modifiers) -> Self {
        Self { key, mods }
    }

    /// Whether a press of `key` with `mods` is this shortcut. Ctrl on
    /// macOS (not Cmd) is ignored, as it always was.
    fn matches(self, key: Key, mods: Modifiers) -> bool {
        self.key == key
            && self.mods.command == mods.command
            && self.mods.shift == mods.shift
            && self.mods.alt == mods.alt
    }
}

/// Builds the action of a shortcut.
pub type ActionCtor = fn() -> Action;

const NONE: Modifiers = Modifiers::NONE;
const SHIFT: Modifiers = Modifiers::SHIFT;
const ALT: Modifiers = Modifiers::ALT;
const ALT_SHIFT: Modifiers = Modifiers::ALT.plus(Modifiers::SHIFT);
const CMD: Modifiers = Modifiers::COMMAND;
const CMD_SHIFT: Modifiers = Modifiers::COMMAND.plus(Modifiers::SHIFT);

/// Every shortcut that means the same wherever the map has the keys. Keys
/// whose meaning depends on the context (Tab, Enter, Esc, the arrows,
/// typing, keys while editing) are handled in [`super::key_to_action`].
/// The first entry for an action is the one menus show.
pub const SHORTCUTS: &[(Shortcut, ActionCtor)] = &[
    // Display only: `key_to_action` takes Ctrl+Enter (with or without
    // Shift, editing or not) among its context keys before the table.
    (Shortcut::new(Key::Enter, CMD), || Action::AddCoPremise),
    (Shortcut::new(Key::R, CMD_SHIFT), || {
        Action::TargetWholeCard(CardKind::Reason)
    }),
    (Shortcut::new(Key::O, CMD_SHIFT), || {
        Action::TargetWholeCard(CardKind::Objection)
    }),
    (Shortcut::new(Key::E, CMD_SHIFT), || {
        Action::AddSource(SourceType::Source)
    }),
    (Shortcut::new(Key::F2, NONE), || Action::Edit),
    (Shortcut::new(Key::Delete, NONE), || Action::Delete),
    (Shortcut::new(Key::Delete, SHIFT), || Action::DeleteBranch),
    (Shortcut::new(Key::P, CMD_SHIFT), || Action::JumpParking),
    (Shortcut::new(Key::M, CMD_SHIFT), || Action::CommentSelected),
    (Shortcut::new(Key::K, CMD_SHIFT), || Action::StartConnect),
    (Shortcut::new(Key::ArrowLeft, ALT), || {
        Action::Reorder(Dir::Left)
    }),
    (Shortcut::new(Key::ArrowRight, ALT), || {
        Action::Reorder(Dir::Right)
    }),
    (Shortcut::new(Key::ArrowUp, ALT), || {
        Action::Reorder(Dir::Up)
    }),
    (Shortcut::new(Key::ArrowDown, ALT), || {
        Action::Reorder(Dir::Down)
    }),
    (Shortcut::new(Key::Num1, ALT), || Action::Rate(Rate::High)),
    (Shortcut::new(Key::Num2, ALT), || Action::Rate(Rate::Mid)),
    (Shortcut::new(Key::Num3, ALT), || Action::Rate(Rate::Low)),
    (Shortcut::new(Key::Num0, ALT), || Action::Rate(Rate::Clear)),
    (Shortcut::new(Key::Num0, ALT_SHIFT), || {
        Action::ClearEvaluation(ClearScope::Map)
    }),
    (Shortcut::new(Key::Z, CMD), || Action::Undo),
    (Shortcut::new(Key::Y, CMD), || Action::Redo),
    (Shortcut::new(Key::Z, CMD_SHIFT), || Action::Redo),
    (Shortcut::new(Key::S, CMD), || Action::Save),
    (Shortcut::new(Key::S, CMD_SHIFT), || Action::SaveAs),
    (Shortcut::new(Key::O, CMD), || Action::Open),
    (Shortcut::new(Key::N, CMD), || Action::New),
    (Shortcut::new(Key::Num0, CMD), || Action::Zoom100),
    (Shortcut::new(Key::Num1, CMD), || Action::ZoomFit),
    (Shortcut::new(Key::Num1, CMD_SHIFT), || {
        Action::ZoomFitSelection
    }),
    (Shortcut::new(Key::Equals, CMD), || Action::ZoomIn),
    (Shortcut::new(Key::Plus, CMD), || Action::ZoomIn),
    (Shortcut::new(Key::Minus, CMD), || Action::ZoomOut),
    (Shortcut::new(Key::Q, ALT), || Action::OpenCommandSearch),
    (Shortcut::new(Key::F, CMD), || Action::OpenFind {
        replace: false,
    }),
    // Cmd+H hides the app on macOS: there Replace is on the ribbon and in
    // Search commands.
    (Shortcut::new(Key::H, CMD), || Action::OpenFind {
        replace: true,
    }),
    (Shortcut::new(Key::F3, NONE), || Action::FindNext),
    (Shortcut::new(Key::F3, SHIFT), || Action::FindPrev),
];

/// The action of the table entry `key` with `mods` presses, if any.
pub(super) fn lookup(key: Key, mods: Modifiers) -> Option<Action> {
    SHORTCUTS
        .iter()
        .find(|(s, _)| s.matches(key, mods))
        .map(|(_, ctor)| ctor())
}

/// The shortcut shown for `action`: its first entry in [`SHORTCUTS`].
pub fn shortcut_for(action: &Action) -> Option<Shortcut> {
    SHORTCUTS
        .iter()
        .find(|(_, ctor)| ctor() == *action)
        .map(|(s, _)| *s)
}

/// How a shortcut is written: "Ctrl+Shift+R", or "⌘⇧R" on macOS.
pub fn shortcut_text(s: Shortcut) -> String {
    format_shortcut(s, cfg!(target_os = "macos"))
}

/// [`shortcut_text`] for either platform (README lists the non-Mac form).
pub(crate) fn format_shortcut(s: Shortcut, mac: bool) -> String {
    let mods = [
        (s.mods.command, "Ctrl", "⌘"),
        (s.mods.alt, "Alt", "⌥"),
        (s.mods.shift, "Shift", "⇧"),
    ];
    let mut parts: Vec<&str> = mods
        .iter()
        .filter(|(on, ..)| *on)
        .map(|&(_, pc, mac_sym)| if mac { mac_sym } else { pc })
        .collect();
    parts.push(key_name(s.key));
    parts.join(if mac { "" } else { "+" })
}

fn key_name(key: Key) -> &'static str {
    match key {
        Key::Num0 => "0",
        Key::Num1 => "1",
        Key::Num2 => "2",
        Key::Num3 => "3",
        Key::Equals => "=",
        Key::Plus => "+",
        Key::Minus => "-",
        Key::ArrowLeft => "Left",
        Key::ArrowRight => "Right",
        Key::Escape => "Esc",
        other => other.name(),
    }
}
