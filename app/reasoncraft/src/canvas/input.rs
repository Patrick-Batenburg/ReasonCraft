//! Keyboard control of the map: shortcuts, typing into boxes and arrow-key
//! navigation.

use eframe::egui::{self, Event, FocusDirection, Key, Modifiers, Popup};
use reasoncraft_core::{Document, Item, Parent};

use super::editor::editor_id;
use crate::actions::{Action, Dir, is_enabled};
use crate::app::ReasonCraftApp;

/// The command bound to `key` with `mods`, if any. `Modifiers::command` is
/// Ctrl, or Cmd on macOS. While a box is being edited only Esc (finish),
/// Tab / Shift+Tab (finish and add a reason / objection) and Ctrl+Enter
/// (finish and add a co-premise) are commands; every other key is text.
pub fn key_to_action(key: Key, mods: Modifiers, editing: bool) -> Option<Action> {
    use Action as A;
    if mods.alt {
        return None;
    }
    let (cmd, shift) = (mods.command, mods.shift);
    if editing {
        return match (key, cmd) {
            (Key::Escape, false) => Some(A::StopEditing),
            (Key::Tab, false) => Some(if shift { A::AddObjection } else { A::AddReason }),
            (Key::Enter, true) => Some(A::AddCoPremise),
            _ => None,
        };
    }
    Some(match (key, cmd, shift) {
        (Key::Tab, false, false) => A::AddReason,
        (Key::Tab, false, true) => A::AddObjection,
        (Key::Enter, true, _) => A::AddCoPremise,
        (Key::Enter | Key::F2, false, false) => A::Edit,
        (Key::Escape, false, _) => A::StopEditing,
        (Key::Delete, false, false) => A::Delete,
        (Key::Delete, false, true) => A::DeleteBranch,
        (Key::ArrowUp, false, false) => A::Navigate(Dir::Up),
        (Key::ArrowDown, false, false) => A::Navigate(Dir::Down),
        (Key::ArrowLeft, false, false) => A::Navigate(Dir::Left),
        (Key::ArrowRight, false, false) => A::Navigate(Dir::Right),
        (Key::Z, true, false) => A::Undo,
        (Key::Z, true, true) | (Key::Y, true, false) => A::Redo,
        (Key::S, true, false) => A::Save,
        (Key::S, true, true) => A::SaveAs,
        (Key::O, true, false) => A::Open,
        (Key::N, true, false) => A::New,
        (Key::Num0, true, false) => A::Zoom100,
        (Key::Num1, true, false) => A::ZoomFit,
        (Key::Equals | Key::Plus, true, _) => A::ZoomIn,
        (Key::Minus, true, false) => A::ZoomOut,
        _ => return None,
    })
}

/// Take this frame's command keys out of the input before any widget sees
/// them and return their actions. Typed text on a selected box starts
/// editing it (whitespace alone does not: Space is held to pan). Keys are
/// left alone while another text field has focus or a menu is open (Esc
/// closes the menu).
///
/// The caller queues the returned actions after the frame's widgets have
/// run, so text typed into the inline editor in the same frame as a command
/// (say "x" then Tab) reaches the edited box before the command ends the edit.
pub fn handle_keys(ctx: &egui::Context, app: &ReasonCraftApp) -> Vec<Action> {
    let Some(tab) = app.tabs.get(app.active) else {
        return Vec::new();
    };
    let editing = tab.editing;
    let focused = ctx.memory(|m| m.focused());
    if (ctx.text_edit_focused() && focused != editing.map(editor_id)) || Popup::is_any_open(ctx) {
        return Vec::new();
    }
    // Esc drops widget focus as the frame begins; keep it on the editor for
    // this frame so text typed alongside the Esc still lands in the box.
    if let (Some(b), None) = (editing, focused) {
        ctx.memory_mut(|m| m.request_focus(editor_id(b)));
    }
    let can_type = editing.is_none() && matches!(tab.selection, Some(Item::Box(_)));
    let mut out = Vec::new();
    ctx.input_mut(|i| {
        i.events.retain(|e| {
            let action = match e {
                Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => key_to_action(*key, *modifiers, editing.is_some()),
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
    // Tab and the arrows are ours: keep egui from moving widget focus too.
    ctx.memory_mut(|m| m.move_focus(FocusDirection::None));
    ctx.request_repaint();
    out.retain(|a| is_enabled(tab, a));
    out
}

/// The item one step from `from` in direction `dir`, or `None` at an edge.
///
/// - Up: the owner (for a premise, the box its card hangs from).
/// - Down: the first child; a card stands for its first premise.
/// - Left/Right: the previous/next co-premise, then the previous/next
///   sibling in the owner's ordered list.
pub fn navigate(doc: &Document, from: Item, dir: Dir) -> Option<Item> {
    match dir {
        Dir::Up => {
            let owner = match doc.owner_of(from)? {
                Parent::Card(c) if matches!(from, Item::Box(_)) => doc.owner_of(Item::Card(c))?,
                owner => owner,
            };
            match owner {
                Parent::Box(b) => Some(Item::Box(b)),
                Parent::Card(c) => Some(Item::Card(c)),
                Parent::Parking => None,
            }
        }
        Dir::Down => match from {
            Item::Card(c) => enter(doc, Item::Card(c), true),
            Item::Box(b) => enter(doc, *doc.boxes.get(&b)?.children.first()?, true),
        },
        Dir::Left => sideways(doc, from, -1),
        Dir::Right => sideways(doc, from, 1),
    }
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
mod tests {
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
        assert_eq!(key_to_action(Key::Num0, cmd, false), Some(Action::Zoom100));
        assert_eq!(key_to_action(Key::Num1, cmd, false), Some(Action::ZoomFit));
        assert_eq!(key_to_action(Key::Equals, cmd, false), Some(Action::ZoomIn));
        assert_eq!(key_to_action(Key::Minus, cmd, false), Some(Action::ZoomOut));
        // Plain letters are text, not commands; Ctrl alone (not Cmd) is no command on macOS.
        assert_eq!(key_to_action(Key::Z, Modifiers::NONE, false), None);
        assert_eq!(key_to_action(Key::Z, Modifiers::MAC_CMD, false), None);
    }

    #[test]
    fn typing_mode_only_intercepts_esc_tab_ctrl_enter() {
        let none = Modifiers::NONE;
        assert_eq!(
            key_to_action(Key::Escape, none, true),
            Some(Action::StopEditing)
        );
        assert_eq!(key_to_action(Key::Tab, none, true), Some(Action::AddReason));
        assert_eq!(
            key_to_action(Key::Enter, Modifiers::COMMAND, true),
            Some(Action::AddCoPremise)
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
            (Key::S, Modifiers::COMMAND),
            (Key::Num0, Modifiers::COMMAND),
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
        let (_, c1) = d.add_card(Parent::Box(a1), CardKind::Reason, "c1").unwrap();
        let b = Item::Box;
        let nav = |from: Item, dir: Dir| navigate(&d, from, dir);

        // Up: premise → the box its card hangs from; card → its owner box.
        assert_eq!(nav(b(a1), Dir::Up), Some(b(root)));
        assert_eq!(nav(Item::Card(ca), Dir::Up), Some(b(root)));
        assert_eq!(nav(b(c1), Dir::Up), Some(b(a1)));
        assert_eq!(nav(b(root), Dir::Up), None);

        // Down: first child; a card child means its first premise.
        assert_eq!(nav(b(root), Dir::Down), Some(b(a1)));
        assert_eq!(nav(Item::Card(ca), Dir::Down), Some(b(a1)));
        assert_eq!(nav(b(a1), Dir::Down), Some(b(c1)));
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
}
