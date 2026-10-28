//! Keyboard control of the map: shortcuts, typing into boxes and arrow-key
//! navigation.

use eframe::egui::{self, Event, FocusDirection, Key, Modifiers, Popup};
use reasoncraft_core::Item;

use super::editor::editor_id;
use crate::actions::{Action, Dir, can_run};
use crate::app::ReasonCraftApp;

mod shortcuts;
pub use shortcuts::{
    ActionCtor, Os, Shortcut, paste_shortcut, shortcut_for, shortcut_text, shortcuts,
};
#[cfg(test)]
pub(crate) use shortcuts::{
    MAC_GLYPHS, MAC_SYMBOLS, format_shortcut, key_name, mac_ctrl, types_with_opt, ui_font_has,
};

/// The command bound to `key` with `mods` on `os`, if any: an entry of
/// [`shortcuts`]`(os)`, or one of the context keys below.
/// `Modifiers::command` is Ctrl, or Cmd on macOS.
///
/// - Tab / Shift+Tab add a reason / objection, Enter or F2 edits the
///   selected box, Esc stops editing and the arrows move the selection.
///   On macOS Backspace / Shift+Backspace delete like Delete / Shift+Delete.
///   On macOS none of these work with Control held: Control+key is only
///   ever a table entry.
/// - While a box is being edited only Esc (finish), Tab / Shift+Tab (finish
///   and add a reason / objection) and the table's whole-card, source,
///   co-premise, file and find keys (finish, then the command) are commands;
///   every other key is text.
/// - Ctrl+Enter adds a co-premise with or without Shift, and Ctrl+Shift+=
///   zooms in like Ctrl+=.
/// - On Windows and Linux Alt+Q (Search commands) is the only Alt
///   combination that is a command while editing, and Alt with Ctrl never
///   is: AltGr reports Ctrl+Alt. On macOS Opt is an ordinary modifier.
pub fn key_to_action(os: Os, key: Key, mods: Modifiers, editing: bool) -> Option<Action> {
    use Action as A;
    if mods.alt {
        if os != Os::Mac && (mods.ctrl || mods.command || mods.mac_cmd) {
            return None;
        }
        let editable = |a: &Action| match os {
            Os::Mac => a.traits().works_while_editing,
            Os::Windows | Os::Linux => *a == A::OpenCommandSearch,
        };
        return shortcuts::lookup(os, key, mods).filter(|a| !editing || editable(a));
    }
    if shortcuts::mac_ctrl(mods) {
        return shortcuts::lookup(os, key, mods)
            .filter(|a| !editing || a.traits().works_while_editing);
    }
    let (cmd, shift) = (mods.command, mods.shift);
    let context = match (key, cmd, shift) {
        (Key::Tab, false, false) => Some(A::AddReason),
        (Key::Tab, false, true) => Some(A::AddObjection),
        (Key::Escape, false, _) => Some(A::StopEditing),
        // Also a (display-only) row of the table; Shift does not matter here.
        (Key::Enter, true, _) => Some(A::AddCoPremise),
        _ => None,
    };
    if context.is_some() {
        return context;
    }
    if editing {
        return shortcuts::lookup(os, key, mods).filter(|a| a.traits().works_while_editing);
    }
    let context = match (key, cmd, shift) {
        (Key::Enter, false, false) => Some(A::Edit),
        (Key::ArrowUp, false, false) => Some(A::Navigate(Dir::Up)),
        (Key::ArrowDown, false, false) => Some(A::Navigate(Dir::Down)),
        (Key::ArrowLeft, false, false) => Some(A::Navigate(Dir::Left)),
        (Key::ArrowRight, false, false) => Some(A::Navigate(Dir::Right)),
        (Key::Equals | Key::Plus, true, true) => Some(A::ZoomIn),
        (Key::Backspace, false, false) if os == Os::Mac => Some(A::Delete),
        (Key::Backspace, false, true) if os == Os::Mac => Some(A::DeleteBranch),
        _ => None,
    };
    context.or_else(|| shortcuts::lookup(os, key, mods))
}

/// Take this frame's command keys out of the input before any widget sees
/// them and return their actions. Typed text on a selected box starts
/// editing it (whitespace alone does not: Space is held to pan). Keys are
/// left alone while a menu is open (Esc closes the menu); while another
/// control has the focus (a text field, or a button reached with Tab) only
/// the file keys are taken (see [`file_keys`]).
///
/// The caller queues the returned actions after the frame's widgets have
/// run, so text typed into the inline editor in the same frame as a command
/// (say "x" then Tab) reaches the edited box before the command ends the edit.
pub fn handle_keys(ctx: &egui::Context, app: &ReasonCraftApp) -> Vec<Action> {
    let Some(tab) = app.tabs.get(app.active) else {
        return Vec::new();
    };
    let editing = tab.editing();
    let focused = ctx.memory(|m| m.focused());
    if Popup::is_any_open(ctx) {
        return Vec::new();
    }
    // Another control has the focus (a field, or a button reached with
    // Tab): its keys are its own (Tab moves on, Enter presses it); only
    // the file keys still work.
    let map_focused = focused.is_none_or(|id| {
        id == super::canvas_id() || Some(id) == editing.map(|b| editor_id(tab, b))
    });
    if !map_focused {
        return file_keys(ctx, app);
    }
    // Esc drops widget focus as the frame begins; keep it on the editor for
    // this frame so text typed alongside the Esc still lands in the box.
    if let (Some(b), None) = (editing, focused) {
        ctx.memory_mut(|m| m.request_focus(editor_id(tab, b)));
    }
    let can_type = editing.is_none() && matches!(tab.selection, Some(Item::Box(_)));
    let mut out = Vec::new();
    // The text an Alt shortcut also produces (say "1") is not typing.
    let mut alt_text = false;
    ctx.input_mut(|i| {
        i.events.retain(|e| {
            let action = match e {
                Event::Key {
                    key,
                    physical_key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    let action = shortcut_key(*key, *physical_key, *modifiers)
                        .and_then(|key| key_to_action(app.os, key, *modifiers, editing.is_some()));
                    alt_text = modifiers.alt
                        && matches!(
                            action,
                            Some(
                                Action::Rate(_)
                                    | Action::ClearEvaluation(_)
                                    | Action::OpenCommandSearch
                            )
                        );
                    action
                }
                Event::Text(_) if std::mem::take(&mut alt_text) => return false,
                Event::Text(s) if can_type && !s.trim().is_empty() => {
                    Some(Action::TypeText(s.clone()))
                }
                _ => None,
            };
            action.map(|a| out.push(a)).is_none()
        });
    });
    if out.is_empty() {
        return out;
    }
    if let Some(item) = tab.connect_mode {
        super::connect::map_keys(item, tab.selection, &mut out);
    } else if editing.is_none() {
        // Esc closes the comment popover, the spelling box (`esc_closes`),
        // then the find bar, before it stops anything else.
        let close = if tab.comments_open.is_some() {
            Some(Action::CloseComments)
        } else if crate::shell::spelling_panel::esc_closes(ctx, tab) {
            Some(Action::CloseSpelling)
        } else {
            tab.find.open.then_some(Action::CloseFind)
        };
        for a in &mut out {
            if *a == Action::StopEditing
                && let Some(close) = &close
            {
                *a = close.clone();
            }
        }
    }
    // Tab and the arrows are ours: keep egui from moving widget focus too.
    ctx.memory_mut(|m| m.move_focus(FocusDirection::None));
    ctx.request_repaint();
    out.retain(|a| can_run(app, a));
    out
}

/// The file keys while a text field other than the box editor has focus
/// (a snippet's text, citation or link, the document title or author, an
/// essay paragraph, the find bar):
/// Ctrl+S, Ctrl+Shift+S, Ctrl+O and Ctrl+N, the find keys (Ctrl+F,
/// Ctrl+H, F3, Shift+F3), the same subset that works while editing a box,
/// Search commands (Alt+Q), F6 / Shift+F6 and F1, or their macOS keys. They
/// are taken out of the input; every other key stays with the field.
/// Applying the action first ends the field's editing session, so its
/// typing becomes a history step before the save.
pub fn file_keys(ctx: &egui::Context, app: &ReasonCraftApp) -> Vec<Action> {
    if app.tabs.get(app.active).is_none() {
        return Vec::new();
    }
    let mut out = Vec::new();
    // The text an Alt shortcut also types stays out of the field.
    let mut alt_text = false;
    ctx.input_mut(|i| {
        i.events.retain(|e| {
            let (key, modifiers) = match e {
                Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => (key, modifiers),
                Event::Text(_) => return !std::mem::take(&mut alt_text),
                _ => return true,
            };
            match key_to_action(app.os, *key, *modifiers, true) {
                Some(
                    a @ (Action::Save
                    | Action::SaveAs
                    | Action::Open
                    | Action::New
                    | Action::OpenCommandSearch
                    | Action::OpenFind { .. }
                    | Action::FindNext
                    | Action::FindPrev
                    | Action::OpenSpelling
                    | Action::NextPane
                    | Action::PrevPane
                    | Action::ContextHelp),
                ) => {
                    alt_text = modifiers.alt;
                    out.push(a);
                    false
                }
                _ => {
                    alt_text = false;
                    true
                }
            }
        });
    });
    out.retain(|a| can_run(app, a));
    out
}

/// The key [`key_to_action`] looks up for a key event, `None` for none.
/// Alt+Shift goes by the physical key: a shifted digit arrives as the
/// symbol it types (")" for Shift+0 on a US layout, which egui has no key
/// for, "=" on a German one).
/// Plain Alt drops the keys of a Windows Alt-code (see [`is_alt_code_key`]).
pub fn shortcut_key(key: Key, physical: Option<Key>, mods: Modifiers) -> Option<Key> {
    if mods.alt && mods.shift {
        Some(physical.unwrap_or(key))
    } else if mods.alt && is_alt_code_key(key, physical) {
        None
    } else {
        Some(key)
    }
}

/// A numpad key typed as part of a Windows Alt-code (Alt+0128) with NumLock
/// off: the physical key is a digit, but it arrives as a navigation key
/// (Numpad4 as ArrowLeft, Numpad1 as End). With NumLock on, egui 0.37 reports
/// numpad digits exactly like the top-row digits, so those cannot be told
/// apart here.
pub fn is_alt_code_key(key: Key, physical: Option<Key>) -> bool {
    use Key::*;
    let digit = |k| {
        matches!(
            k,
            Num0 | Num1 | Num2 | Num3 | Num4 | Num5 | Num6 | Num7 | Num8 | Num9
        )
    };
    physical.is_some_and(|p| digit(p) && p != key)
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_alt;
#[cfg(test)]
mod tests_os;
