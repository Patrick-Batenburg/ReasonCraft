//! The keyboard shortcut tables, one per platform, and how they are shown.
//!
//! README's keyboard tables are checked against [`shortcuts`] by a test:
//! the same keys, each on the row of the action it runs.

use std::sync::LazyLock;

use eframe::egui::{FontDefinitions, FontFamily, Key, Modifiers};
use reasoncraft_core::{CardKind, ClearScope, SourceType};

use crate::actions::{Action, Dir, Rate};
use crate::i18n;

/// The platform whose keys apply: macOS has its own table (see [`shortcuts`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Os {
    Windows,
    Mac,
    Linux,
}

impl Os {
    /// The platform this build runs on.
    pub fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::Mac
        } else if cfg!(target_os = "windows") {
            Self::Windows
        } else {
            Self::Linux
        }
    }
}

/// A key with its modifiers. `Modifiers::command` is Ctrl, or Cmd on macOS;
/// `ctrl` without `command` is Control on macOS (see [`mac_ctrl`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shortcut {
    pub key: Key,
    pub mods: Modifiers,
}

impl Shortcut {
    const fn new(key: Key, mods: Modifiers) -> Self {
        Self { key, mods }
    }

    /// Whether a press of `key` with `mods` is this shortcut.
    fn matches(self, key: Key, mods: Modifiers) -> bool {
        self.key == key
            && self.mods.command == mods.command
            && self.mods.shift == mods.shift
            && self.mods.alt == mods.alt
            && mac_ctrl(self.mods) == mac_ctrl(mods)
    }
}

/// Whether `mods` hold Control as a key of its own: macOS's Control, next
/// to Cmd. On Windows and Linux Ctrl is `command`, so this is never true.
pub(crate) fn mac_ctrl(mods: Modifiers) -> bool {
    mods.ctrl && (mods.mac_cmd || !mods.command)
}

/// Builds the action of a shortcut.
pub type ActionCtor = fn() -> Action;

const NONE: Modifiers = Modifiers::NONE;
const SHIFT: Modifiers = Modifiers::SHIFT;
const ALT: Modifiers = Modifiers::ALT;
const ALT_SHIFT: Modifiers = Modifiers::ALT.plus(Modifiers::SHIFT);
const CMD: Modifiers = Modifiers::COMMAND;
const CMD_SHIFT: Modifiers = Modifiers::COMMAND.plus(Modifiers::SHIFT);
const CMD_ALT: Modifiers = Modifiers::COMMAND.plus(Modifiers::ALT);

/// The shortcuts of `os`: every shortcut that means the same wherever the
/// map has the keys. Keys whose meaning depends on the context (Tab, Enter,
/// Esc, the arrows, typing, Backspace on macOS, keys while editing) are
/// handled in [`super::key_to_action`]. The first entry for an action is
/// the one menus show.
pub fn shortcuts(os: Os) -> &'static [(Shortcut, ActionCtor)] {
    match os {
        Os::Mac => &MAC,
        Os::Windows | Os::Linux => SHORTCUTS,
    }
}

/// The Windows and Linux table.
const SHORTCUTS: &[(Shortcut, ActionCtor)] = &[
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
    (Shortcut::new(Key::L, CMD_SHIFT), || {
        Action::OpenSelectedLink
    }),
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
    // Display only: egui turns these keys into its Cut, Copy and Paste
    // events (see `super::clipboard_events`), so `lookup` skips them.
    (Shortcut::new(Key::X, CMD), || Action::Cut),
    (Shortcut::new(Key::C, CMD), || Action::Copy),
    (Shortcut::new(Key::V, CMD), || Action::Paste(None)),
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
    // Cmd+H hides the app on macOS: there Replace is Cmd+Opt+F.
    (Shortcut::new(Key::H, CMD), || Action::OpenFind {
        replace: true,
    }),
    (Shortcut::new(Key::F3, NONE), || Action::FindNext),
    (Shortcut::new(Key::F3, SHIFT), || Action::FindPrev),
    (Shortcut::new(Key::F7, NONE), || Action::OpenSpelling),
    (Shortcut::new(Key::F1, NONE), || Action::ContextHelp),
    (Shortcut::new(Key::F6, NONE), || Action::NextPane),
    (Shortcut::new(Key::F6, SHIFT), || Action::PrevPane),
    (Shortcut::new(Key::Tab, CMD), || Action::NextTab),
    (Shortcut::new(Key::Tab, CMD_SHIFT), || Action::PrevTab),
    (Shortcut::new(Key::PageDown, CMD), || Action::NextTab),
    (Shortcut::new(Key::PageUp, CMD), || Action::PrevTab),
    (Shortcut::new(Key::W, CMD), || Action::CloseActiveTab),
    (Shortcut::new(Key::F4, CMD), || Action::CloseActiveTab),
];

/// The macOS table, [`SHORTCUTS`] with these changes:
/// - Opt with a letter or digit types a character on macOS. With a digit
///   it takes Cmd as well (Cmd+Opt types nothing; Control+digit can be
///   Mission Control's "Switch to Desktop"); with a letter it becomes
///   Control. Opt with an arrow key stays (Control with the arrows
///   switches desktops);
/// - Cmd+H hides the app, so Replace is Cmd+Opt+F;
/// - Search commands is Cmd+Shift+A;
/// - the next and previous tab take Control, as Cmd+Tab switches apps.
static MAC: LazyLock<Vec<(Shortcut, ActionCtor)>> = LazyLock::new(|| {
    SHORTCUTS
        .iter()
        .map(|&(s, ctor)| (mac_shortcut(s, &ctor()), ctor))
        .collect()
});

fn mac_shortcut(s: Shortcut, action: &Action) -> Shortcut {
    match action {
        Action::OpenFind { replace: true } => Shortcut::new(Key::F, CMD_ALT),
        Action::OpenCommandSearch => Shortcut::new(Key::A, CMD_SHIFT),
        Action::NextTab | Action::PrevTab => {
            let mods = Modifiers {
                command: false,
                ctrl: true,
                ..s.mods
            };
            Shortcut::new(s.key, mods)
        }
        _ if s.mods.alt && is_digit(s.key) => Shortcut::new(s.key, s.mods.plus(CMD)),
        _ if s.mods.alt && types_with_opt(s.key) => {
            let mods = Modifiers {
                alt: false,
                ctrl: true,
                ..s.mods
            };
            Shortcut::new(s.key, mods)
        }
        _ => s,
    }
}

/// Whether Opt with `key` types a character on macOS: letters and digits.
pub(crate) fn types_with_opt(key: Key) -> bool {
    let name = key.name();
    name.len() == 1 && name.chars().all(|c| c.is_ascii_alphanumeric())
}

/// Whether `key` is a digit key.
fn is_digit(key: Key) -> bool {
    key.name().len() == 1 && key.name().chars().all(|c| c.is_ascii_digit())
}

/// Paste: Ctrl+V, or Cmd+V on macOS (a display-only entry of the table).
pub fn paste_shortcut() -> Shortcut {
    Shortcut::new(Key::V, CMD)
}

/// Whether `action`'s entries are there to be shown only: egui delivers
/// those keys as its own clipboard events, never as key presses.
fn display_only(action: &Action) -> bool {
    matches!(action, Action::Cut | Action::Copy | Action::Paste(_))
}

/// The action of the entry of `os`'s table that `key` with `mods` presses.
pub(super) fn lookup(os: Os, key: Key, mods: Modifiers) -> Option<Action> {
    shortcuts(os)
        .iter()
        .find(|(s, ctor)| s.matches(key, mods) && !display_only(&ctor()))
        .map(|(_, ctor)| ctor())
}

/// The shortcut shown for `action` on `os`: its first entry in the table.
pub fn shortcut_for(action: &Action, os: Os) -> Option<Shortcut> {
    shortcuts(os)
        .iter()
        .find(|(_, ctor)| ctor() == *action)
        .map(|(s, _)| *s)
}

/// How a shortcut of `os` is written in the UI language: "Ctrl+Shift+R";
/// on macOS "⇧⌘R", or "Shift+Cmd+R" if the UI font lacks the symbols.
pub fn shortcut_text(s: Shortcut, os: Os) -> String {
    format_shortcut(s, os, *MAC_GLYPHS, i18n::ui_language())
}

/// The macOS modifier symbols, in Apple's order.
pub(crate) const MAC_SYMBOLS: &str = "⌃⌥⇧⌘";

/// Whether the UI font (egui's bundled proportional family) has every
/// symbol of [`MAC_SYMBOLS`].
pub(crate) static MAC_GLYPHS: LazyLock<bool> =
    LazyLock::new(|| MAC_SYMBOLS.chars().all(ui_font_has));

/// Whether a font of the bundled proportional family maps `c`.
pub(crate) fn ui_font_has(c: char) -> bool {
    use skrifa::MetadataProvider as _;
    let defs = FontDefinitions::default();
    defs.families[&FontFamily::Proportional].iter().any(|name| {
        let data = &defs.font_data[name];
        skrifa::FontRef::from_index((*data.font).as_ref(), data.index)
            .is_ok_and(|f| f.charmap().map(c).is_some())
    })
}

/// [`shortcut_text`] for any platform, with key names in `lang` (README
/// lists the shortcuts in English). On macOS the modifiers come in Apple's
/// order (Control, Option, Shift, Command): as symbols with `glyphs`, else
/// as names ("Control+Option+Shift+Cmd"). Elsewhere they are Ctrl+Alt+Shift.
pub(crate) fn format_shortcut(s: Shortcut, os: Os, glyphs: bool, lang: &str) -> String {
    let m = s.mods;
    let mac = os == Os::Mac;
    let mods: &[(bool, &str, &str)] = if mac {
        &[
            (mac_ctrl(m), "key.control", "⌃"),
            (m.alt, "key.opt", "⌥"),
            (m.shift, "key.shift", "⇧"),
            (m.command, "key.cmd", "⌘"),
        ]
    } else {
        &[
            (m.command, "key.ctrl", ""),
            (m.alt, "key.alt", ""),
            (m.shift, "key.shift", ""),
        ]
    };
    let glyphs = glyphs && mac;
    let mut parts: Vec<String> = mods
        .iter()
        .filter(|(on, ..)| *on)
        .map(|&(_, key, symbol)| {
            if glyphs {
                symbol.to_owned()
            } else {
                i18n::t_lang(lang, key)
            }
        })
        .collect();
    parts.push(key_name(s.key, lang));
    parts.join(if glyphs { "" } else { "+" })
}

/// The name of `key` in `lang`: named keys are translated, letters,
/// digits and function keys are written as they are.
pub(crate) fn key_name(key: Key, lang: &str) -> String {
    let named = match key {
        Key::Escape => "key.esc",
        Key::Enter => "key.enter",
        Key::Space => "key.space",
        Key::Tab => "key.tab",
        Key::Backspace => "key.backspace",
        Key::Delete => "key.delete",
        Key::ArrowLeft => "key.left",
        Key::ArrowRight => "key.right",
        Key::ArrowUp => "key.up",
        Key::ArrowDown => "key.down",
        Key::Num0 => return "0".into(),
        Key::Num1 => return "1".into(),
        Key::Num2 => return "2".into(),
        Key::Num3 => return "3".into(),
        Key::Equals => return "=".into(),
        Key::Plus => return "+".into(),
        Key::Minus => return "-".into(),
        other => return other.name().into(),
    };
    i18n::t_lang(lang, named)
}
