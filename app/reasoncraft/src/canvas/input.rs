//! Keyboard control of the map: shortcuts, typing into boxes and arrow-key
//! navigation.

use eframe::egui::{self, Event, FocusDirection, Key, Modifiers, Popup};
use reasoncraft_core::{Document, Item, Parent, SourceType};

use super::editor::editor_id;
use crate::actions::{Action, Dir, is_enabled};
use crate::app::ReasonCraftApp;

mod shortcuts;
#[cfg(test)]
pub(crate) use shortcuts::format_shortcut;
pub use shortcuts::{ActionCtor, SHORTCUTS, Shortcut, shortcut_for, shortcut_text};

/// The command bound to `key` with `mods`, if any: an entry of
/// [`SHORTCUTS`], or one of the context keys below. `Modifiers::command`
/// is Ctrl, or Cmd on macOS.
///
/// - Tab / Shift+Tab add a reason / objection, Enter or F2 edits the
///   selected box, Esc stops editing and the arrows move the selection.
/// - While a box is being edited only Esc (finish), Tab / Shift+Tab (finish
///   and add a reason / objection) and the table's whole-card, source,
///   co-premise and file keys (finish, then the command) are commands;
///   every other key is text.
/// - Ctrl+Enter adds a co-premise with or without Shift, and Ctrl+Shift+=
///   zooms in like Ctrl+=.
/// - Alt+Q (Search commands) is the only Alt combination that is a
///   command while editing. Alt with Ctrl never is: AltGr reports Ctrl+Alt.
pub fn key_to_action(key: Key, mods: Modifiers, editing: bool) -> Option<Action> {
    use Action as A;
    if mods.alt {
        if mods.ctrl || mods.command || mods.mac_cmd {
            return None;
        }
        let action = shortcuts::lookup(key, mods);
        return action.filter(|a| !editing || *a == A::OpenCommandSearch);
    }
    let (cmd, shift) = (mods.command, mods.shift);
    let context = match (key, cmd, shift) {
        (Key::Tab, false, false) => Some(A::AddReason),
        (Key::Tab, false, true) => Some(A::AddObjection),
        (Key::Escape, false, _) => Some(A::StopEditing),
        (Key::Enter, true, _) => Some(A::AddCoPremise),
        _ => None,
    };
    if context.is_some() {
        return context;
    }
    if editing {
        return shortcuts::lookup(key, mods).filter(works_while_editing);
    }
    let context = match (key, cmd, shift) {
        (Key::Enter, false, false) => Some(A::Edit),
        (Key::ArrowUp, false, false) => Some(A::Navigate(Dir::Up)),
        (Key::ArrowDown, false, false) => Some(A::Navigate(Dir::Down)),
        (Key::ArrowLeft, false, false) => Some(A::Navigate(Dir::Left)),
        (Key::ArrowRight, false, false) => Some(A::Navigate(Dir::Right)),
        (Key::Equals | Key::Plus, true, true) => Some(A::ZoomIn),
        _ => None,
    };
    context.or_else(|| shortcuts::lookup(key, mods))
}

/// The table's commands that also end an inline edit.
fn works_while_editing(action: &Action) -> bool {
    matches!(
        action,
        Action::TargetWholeCard(_)
            | Action::AddSource(SourceType::Source)
            | Action::Save
            | Action::SaveAs
            | Action::Open
            | Action::New
    )
}

/// Take this frame's command keys out of the input before any widget sees
/// them and return their actions. Typed text on a selected box starts
/// editing it (whitespace alone does not: Space is held to pan). Keys are
/// left alone while a menu is open (Esc closes the menu); while another
/// text field has focus only the file keys are taken (see [`file_keys`]).
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
    if ctx.text_edit_focused() && focused != editing.map(|b| editor_id(tab, b)) {
        return file_keys(ctx, app);
    }
    // Esc drops widget focus as the frame begins; keep it on the editor for
    // this frame so text typed alongside the Esc still lands in the box.
    if let (Some(b), None) = (editing, focused) {
        ctx.memory_mut(|m| m.request_focus(editor_id(tab, b)));
    }
    let can_type = editing.is_none() && matches!(tab.selection, Some(Item::Box(_)));
    let mut out = Vec::new();
    // The text an Alt shortcut also produces ("1", or "¡" or
    // "œ" on macOS) is not typing.
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
                        .and_then(|key| key_to_action(key, *modifiers, editing.is_some()));
                    alt_text = matches!(
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
    } else if tab.comments_open.is_some() && editing.is_none() {
        // Esc closes the comment popover before it stops anything else.
        for a in &mut out {
            if *a == Action::StopEditing {
                *a = Action::CloseComments;
            }
        }
    }
    // Tab and the arrows are ours: keep egui from moving widget focus too.
    ctx.memory_mut(|m| m.move_focus(FocusDirection::None));
    ctx.request_repaint();
    out.retain(|a| is_enabled(tab, a));
    out
}

/// The file keys while a text field other than the box editor has focus
/// (a snippet's text, citation or link, the document title or author, an
/// essay paragraph):
/// Ctrl+S, Ctrl+Shift+S, Ctrl+O and Ctrl+N, the same subset that works
/// while editing a box, and Alt+Q. They are taken out of the input; every other key
/// stays with the field. Applying the action first ends the field's
/// editing session, so its typing becomes a history step before the save.
pub fn file_keys(ctx: &egui::Context, app: &ReasonCraftApp) -> Vec<Action> {
    let Some(tab) = app.tabs.get(app.active) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    ctx.input_mut(|i| {
        i.events.retain(|e| {
            let Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } = e
            else {
                return true;
            };
            match key_to_action(*key, *modifiers, true) {
                Some(
                    a @ (Action::Save
                    | Action::SaveAs
                    | Action::Open
                    | Action::New
                    | Action::OpenCommandSearch),
                ) => {
                    out.push(a);
                    false
                }
                _ => true,
            }
        });
    });
    out.retain(|a| is_enabled(tab, a));
    out
}

/// The key [`key_to_action`] looks up for a key event, `None` for none.
/// Alt+Shift goes by the physical key: a shifted digit arrives as the
/// symbol it types (")" for Shift+0 on a US layout, which egui has no key
/// for, "=" on a German one).
/// Plain Alt drops the keys of a Windows Alt-code (see [`is_alt_code_key`]).
pub fn shortcut_key(key: Key, physical: Option<Key>, mods: Modifiers) -> Option<Key> {
    match (mods.alt, mods.shift) {
        (true, true) => Some(physical.unwrap_or(key)),
        (true, false) if is_alt_code_key(key, physical) => None,
        _ => Some(key),
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

/// The item one step from `from` in direction `dir`, or `None` at an edge.
///
/// - Up: the owner (a premise's card; a card's box, or the card it targets
///   as a whole; nothing for a parked item).
/// - Down: a card's first premise; a box's first child (a card, not its
///   premise).
/// - Left/Right: the previous/next co-premise, then the previous/next
///   sibling in the owner's ordered list.
pub fn navigate(doc: &Document, from: Item, dir: Dir) -> Option<Item> {
    match dir {
        Dir::Up => match doc.owner_of(from)? {
            Parent::Box(b) => Some(Item::Box(b)),
            Parent::Card(c) => Some(Item::Card(c)),
            Parent::Parking => None,
        },
        Dir::Down => match from {
            Item::Card(c) => enter(doc, Item::Card(c), true),
            Item::Box(b) => doc.boxes.get(&b)?.children.first().copied(),
        },
        Dir::Left => sideways(doc, from, -1),
        Dir::Right => sideways(doc, from, 1),
    }
}

/// Where Ctrl+Shift+P goes from `from`: the main root from anything in the
/// parking area, otherwise the first parked item (`None` if there is none).
pub fn parking_jump(doc: &Document, from: Option<Item>) -> Option<Item> {
    if from.is_some_and(|item| is_parked(doc, item)) {
        Some(Item::Box(doc.root))
    } else {
        doc.parking.first().copied()
    }
}

/// Whether `item` hangs, at any depth, under a parking entry.
fn is_parked(doc: &Document, mut item: Item) -> bool {
    // Bounded by the item count, so even a damaged cycle ends.
    for _ in 0..=doc.boxes.len() + doc.cards.len() {
        item = match doc.owner_of(item) {
            Some(Parent::Parking) => return true,
            Some(Parent::Box(b)) => Item::Box(b),
            Some(Parent::Card(c)) => Item::Card(c),
            None => return false,
        };
    }
    false
}

/// A card stands for its first (or last) premise when reached from a box.
fn enter(doc: &Document, item: Item, first: bool) -> Option<Item> {
    match item {
        Item::Card(c) => {
            let premises = &doc.cards.get(&c)?.premises;
            let p = if first {
                premises.first()
            } else {
                premises.last()
            };
            p.map(|&p| Item::Box(p))
        }
        b => Some(b),
    }
}

fn sideways(doc: &Document, from: Item, step: isize) -> Option<Item> {
    if let (Item::Box(b), Some(Parent::Card(c))) = (from, doc.owner_of(from)) {
        let premises = &doc.cards.get(&c)?.premises;
        let i = premises.iter().position(|&p| p == b)?;
        if let Some(&p) = neighbour(premises, i, step) {
            return Some(Item::Box(p));
        }
        let next = sibling(doc, Item::Card(c), step)?;
        return enter(doc, next, step > 0);
    }
    let next = sibling(doc, from, step)?;
    match from {
        Item::Card(_) => Some(next),
        Item::Box(_) => enter(doc, next, step > 0),
    }
}

/// The item `step` places away from `item` in its owner's ordered list.
fn sibling(doc: &Document, item: Item, step: isize) -> Option<Item> {
    let list: Vec<Item> = match doc.owner_of(item)? {
        Parent::Box(b) => doc.boxes.get(&b)?.children.clone(),
        Parent::Card(c) => {
            let card = doc.cards.get(&c)?;
            card.children.iter().map(|&k| Item::Card(k)).collect()
        }
        Parent::Parking => doc.parking.clone(),
    };
    let i = list.iter().position(|&x| x == item)?;
    neighbour(&list, i, step).copied()
}

fn neighbour<T>(list: &[T], i: usize, step: isize) -> Option<&T> {
    list.get(i.checked_add_signed(step)?)
}

#[cfg(test)]
mod tests;
