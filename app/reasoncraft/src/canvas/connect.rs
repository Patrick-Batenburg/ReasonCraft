//! The canvas side of "Connect to…": while a parked item waits for a
//! target, hovering a box or card outlines it green (allowed) or red
//! (refused), a click on an allowed target connects, Enter connects to the
//! selection and Esc cancels.

use eframe::egui::{self, CursorIcon, Pos2, Response, Stroke, StrokeKind};
use reasoncraft_core::{Item, Layout, Parent};

use super::drag::paint_target;
use super::hit::{Hit, hit_test};
use crate::actions::{Action, is_enabled};
use crate::doc_tab::DocTab;
use crate::scene::metrics::FRAME_RADIUS;
use crate::theme::Tokens;

/// The parent that box or card `target` offers: a box takes the item as a
/// child, a card as a whole-card child.
pub fn parent_of(target: Item) -> Parent {
    match target {
        Item::Box(b) => Parent::Box(b),
        Item::Card(c) => Parent::Card(c),
    }
}

/// The connecting move for a click at `world`: what is under the pointer
/// (a box, or a card frame for a whole-card target). `None` over empty canvas.
pub fn click_target(layout: &Layout, item: Item, world: Pos2, zoom: f32) -> Option<Action> {
    let hit = hit_test(layout, world, zoom)?;
    let to = match hit {
        Hit::Box(b) => Parent::Box(b),
        Hit::CardFrame(c) | Hit::CardFarEdge(c) => Parent::Card(c),
    };
    Some(Action::Move {
        item,
        to,
        index: None,
    })
}

/// Rewrite this frame's key actions while `item` waits for a target: Enter
/// (and F2) connect to the selection, Esc cancels, the arrows, Ctrl+Shift+P
/// and the zoom keys work as usual, and every other key does nothing.
pub fn map_keys(item: Item, selection: Option<Item>, out: &mut Vec<Action>) {
    out.retain_mut(|a| {
        match a {
            Action::Edit => match selection {
                Some(target) => {
                    *a = Action::Move {
                        item,
                        to: parent_of(target),
                        index: None,
                    }
                }
                None => return false,
            },
            Action::StopEditing => *a = Action::CancelConnect,
            Action::Navigate(_)
            | Action::JumpParking
            | Action::ZoomIn
            | Action::ZoomOut
            | Action::ZoomFit
            | Action::ZoomFitSelection
            | Action::Zoom100 => {}
            _ => return false,
        }
        true
    });
}

/// While a parked item waits for a target: outline it in the accent colour,
/// outline the target under the pointer green or red, and turn a click on
/// an allowed target into the connecting move (the canvas does not select
/// while the mode is on).
pub(super) fn update(
    ui: &egui::Ui,
    response: &Response,
    layout: &Layout,
    tab: &DocTab,
    tk: &Tokens,
    pending: &mut Vec<Action>,
    panning: bool,
) {
    let Some(item) = tab.connect_mode else {
        return;
    };
    let cam = tab.camera;
    let painter = ui.painter_at(response.rect);
    let own = match item {
        Item::Box(b) => layout.boxes.get(&b),
        Item::Card(c) => layout.cards.get(&c),
    };
    if let Some(&own) = own {
        let r = cam.world_rect_to_screen(own).expand(4.0);
        painter.rect_stroke(
            r,
            FRAME_RADIUS * cam.zoom + 4.0,
            Stroke::new(2.0, tk.accent),
            StrokeKind::Middle,
        );
    }
    let Some(pos) = response.hover_pos() else {
        return;
    };
    let world = cam.screen_to_world(pos);
    let Some(action) = click_target(layout, item, world, cam.zoom) else {
        return;
    };
    let allowed = is_enabled(tab, &action);
    let highlight = match hit_test(layout, world, cam.zoom) {
        Some(Hit::Box(b)) => layout.boxes.get(&b),
        Some(Hit::CardFrame(c) | Hit::CardFarEdge(c)) => layout.cards.get(&c),
        None => None,
    };
    if let Some(&rect) = highlight {
        paint_target(
            &painter,
            tab,
            rect,
            if allowed { tk.reason } else { tk.objection },
        );
    }
    ui.ctx().set_cursor_icon(if allowed {
        CursorIcon::PointingHand
    } else {
        CursorIcon::NotAllowed
    });
    if allowed && response.clicked() && !panning {
        pending.push(action);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reasoncraft_core::{CardKind, Document};

    #[test]
    fn enter_connects_to_selection_and_esc_cancels() {
        let mut d = Document::new("Claim");
        let root = d.root;
        let parked = d.add_loose(reasoncraft_core::NewLoose::Card(CardKind::Reason), "p");
        let mut out = vec![
            Action::Navigate(crate::actions::Dir::Down),
            Action::Edit,
            Action::AddReason,
            Action::StopEditing,
        ];
        map_keys(parked, Some(Item::Box(root)), &mut out);
        assert_eq!(
            out,
            vec![
                Action::Navigate(crate::actions::Dir::Down),
                Action::Move {
                    item: parked,
                    to: Parent::Box(root),
                    index: None
                },
                Action::CancelConnect,
            ]
        );
    }
}
