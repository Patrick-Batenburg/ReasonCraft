//! Drag and drop: drop a box or card on a box (connect), on a card's bottom
//! edge (target the whole card) or on empty canvas (detach to parking).

use eframe::egui::{
    self, Color32, Painter, PointerButton, Pos2, Rect, Response, Stroke, StrokeKind,
};
use reasoncraft_core::{BoxKind, Document, Item, Layout, NodeId, Parent};

use super::hit::{BOTTOM_EDGE_BAND, Hit, hit_test};
use super::paint::{FRAME_RADIUS, parking_frame};
use crate::actions::{Action, Insertable, is_enabled, target_allowed};
use crate::doc_tab::DocTab;
use crate::theme::Tokens;

/// Pointer travel, in screen px, before a press on an item becomes a drag.
pub const DRAG_START_PX: f32 = 6.0;

/// A press on an item that may turn into a drag. Kept in egui's temp memory.
#[derive(Clone, Copy, Debug)]
struct DragState {
    item: Item,
    /// Where the press started, in screen px.
    origin: Pos2,
    /// Moved at least [`DRAG_START_PX`].
    active: bool,
}

/// Where a drop would put the dragged item, and what to highlight (world units).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DropTarget {
    pub to: Parent,
    pub highlight: Option<Rect>,
}

/// The item a press on `hit` drags. The root never moves; a card's only
/// premise stands for its whole card.
pub fn drag_item(doc: &Document, hit: Hit) -> Option<Item> {
    let item = Item::from(hit);
    if item == Item::Box(doc.root) {
        return None;
    }
    if let (Item::Box(_), Some(Parent::Card(c))) = (item, doc.owner_of(item))
        && doc.cards.get(&c).is_some_and(|k| k.premises.len() == 1)
    {
        return Some(Item::Card(c));
    }
    Some(item)
}

/// Where dropping `item` at `world` would put it: a box takes it as a
/// child (a premise joins the card of the premise it is dropped on), a card
/// frame or its bottom edge takes it as a whole-card child, empty canvas
/// parks it. `None` when nothing would change.
pub fn drop_target(
    doc: &Document,
    layout: &Layout,
    item: Item,
    world: Pos2,
    zoom: f32,
) -> Option<DropTarget> {
    let is_premise = matches!(item, Item::Box(b)
        if doc.boxes.get(&b).is_some_and(|n| n.kind == BoxKind::Premise));
    let inside_item = |b| match item {
        Item::Box(i) => i == b,
        Item::Card(c) => doc.cards.get(&c).is_some_and(|k| k.premises.contains(&b)),
    };
    let target = match hit_test(layout, world, zoom) {
        Some(Hit::Box(b)) => {
            if inside_item(b) {
                return None;
            }
            let to = match doc.owner_of(Item::Box(b)) {
                Some(Parent::Card(c)) if is_premise => Parent::Card(c),
                _ => Parent::Box(b),
            };
            DropTarget {
                to,
                highlight: layout.boxes.get(&b).copied(),
            }
        }
        Some(Hit::CardBottomEdge(c)) => DropTarget {
            to: Parent::Card(c),
            highlight: layout.cards.get(&c).map(|&r| bottom_band(r, zoom)),
        },
        Some(Hit::CardFrame(c)) => DropTarget {
            to: Parent::Card(c),
            highlight: layout.cards.get(&c).copied(),
        },
        None => DropTarget {
            to: Parent::Parking,
            highlight: parking_frame(layout),
        },
    };
    // Back where it is, or a card onto itself: nothing would change.
    let onto_itself = matches!((item, target.to), (Item::Card(a), Parent::Card(b)) if a == b);
    let unchanged = onto_itself || doc.owner_of(item) == Some(target.to);
    (!unchanged).then_some(target)
}

/// Where dropping `what` from the building panel at `world` would insert
/// it, and whether that is allowed. A box takes it as a child (a co-premise
/// joins the box's card; a comment goes on the box), a card frame or its
/// bottom edge as a whole-card child (cards only; a co-premise joins the
/// card), empty canvas parks it (never a comment or co-premise).
pub fn panel_drop_target(
    doc: &Document,
    layout: &Layout,
    what: &Insertable,
    world: Pos2,
    zoom: f32,
) -> Option<(DropTarget, bool)> {
    let card = |c| layout.cards.get(&c).copied();
    let target = match hit_test(layout, world, zoom) {
        Some(Hit::Box(b)) => match doc.owner_of(Item::Box(b)) {
            Some(Parent::Card(c)) if *what == Insertable::CoPremise => DropTarget {
                to: Parent::Box(b),
                highlight: card(c),
            },
            _ => DropTarget {
                to: Parent::Box(b),
                highlight: layout.boxes.get(&b).copied(),
            },
        },
        Some(Hit::CardFrame(c) | Hit::CardBottomEdge(c)) if *what == Insertable::CoPremise => {
            DropTarget {
                to: Parent::Box(*doc.cards.get(&c)?.premises.first()?),
                highlight: card(c),
            }
        }
        Some(Hit::CardBottomEdge(c)) => DropTarget {
            to: Parent::Card(c),
            highlight: card(c).map(|r| bottom_band(r, zoom)),
        },
        Some(Hit::CardFrame(c)) => DropTarget {
            to: Parent::Card(c),
            highlight: card(c),
        },
        None => DropTarget {
            to: Parent::Parking,
            highlight: parking_frame(layout),
        },
    };
    let allowed =
        insert_target(what, &target).is_some_and(|(to, on)| target_allowed(doc, what, to, on));
    Some((target, allowed))
}

/// `to` and `on` of the insert a building-panel drop of `what` on `target`
/// asks for; `None` when `what` needs a box and the target is none.
fn insert_target(
    what: &Insertable,
    target: &DropTarget,
) -> Option<(Option<Parent>, Option<NodeId>)> {
    match (what.goes_on_node(), target.to) {
        (true, Parent::Box(b)) => Some((None, Some(b))),
        (true, _) => None,
        (false, to) => Some((Some(to), None)),
    }
}

/// The [`Action::Insert`] for a building-panel drop of `what` on `target`.
pub fn panel_drop_action(what: &Insertable, target: &DropTarget) -> Option<Action> {
    let (to, on) = insert_target(what, target)?;
    Some(Action::Insert {
        what: what.clone(),
        to,
        on,
    })
}

/// The band around the bottom edge of card frame `r` (world units).
fn bottom_band(r: Rect, zoom: f32) -> Rect {
    let half = BOTTOM_EDGE_BAND / zoom / 2.0;
    Rect::from_x_y_ranges(r.x_range(), (r.bottom() - half)..=(r.bottom() + half))
}

/// Track a primary-button drag that started on an item, paint its ghost and
/// drop target, and push [`Action::Move`] on a valid drop. `panning`: Space
/// is held, so the drag pans instead.
pub(super) fn update(
    ui: &egui::Ui,
    response: &Response,
    layout: &Layout,
    tab: &DocTab,
    tk: &Tokens,
    pending: &mut Vec<Action>,
    panning: bool,
) {
    let id = response.id.with("drag");
    let painter = &ui.painter_at(response.rect);
    let cam = tab.camera;
    let mut state: Option<DragState> = ui.data(|d| d.get_temp(id));
    if response.drag_started_by(PointerButton::Primary) && !panning {
        let origin = ui.input(|i| i.pointer.press_origin());
        state = origin.and_then(|o| {
            let hit = hit_test(layout, cam.screen_to_world(o), cam.zoom)?;
            Some(DragState {
                item: drag_item(&tab.doc, hit)?,
                origin: o,
                active: false,
            })
        });
    }
    let pointer = ui.input(|i| i.pointer.latest_pos());
    let released = response.drag_stopped();
    if let (Some(s), Some(pos)) = (state.as_mut(), pointer) {
        s.active |= (pos - s.origin).length() >= DRAG_START_PX;
        if s.active {
            let target = drop_target(&tab.doc, layout, s.item, cam.screen_to_world(pos), cam.zoom);
            let allowed = target.is_some_and(|d| {
                is_enabled(
                    tab,
                    &Action::Move {
                        item: s.item,
                        to: d.to,
                    },
                )
            });
            if released {
                if let (true, Some(d)) = (allowed, target) {
                    pending.push(Action::Move {
                        item: s.item,
                        to: d.to,
                    });
                }
            } else {
                // Green or red where it would land; neutral over itself.
                let colour = match target {
                    None => tk.accent,
                    Some(_) if allowed => tk.reason,
                    Some(_) => tk.objection,
                };
                paint_ghost(painter, layout, tab, colour, s, pos);
                if let Some(rect) = target.and_then(|d| d.highlight) {
                    paint_target(painter, tab, rect, colour);
                }
            }
        }
    }
    if released || !(response.dragged_by(PointerButton::Primary) || response.drag_started()) {
        state = None;
    }
    ui.data_mut(|d| match state {
        Some(s) => {
            d.insert_temp(id, s);
        }
        None => d.remove::<DragState>(id),
    });
}

/// A drop target's outline: green where a drop is allowed, red where refused.
fn paint_target(painter: &Painter, tab: &DocTab, world: Rect, colour: Color32) {
    let r = tab.camera.world_rect_to_screen(world).expand(3.0);
    painter.rect_stroke(r, 6.0, Stroke::new(2.5, colour), StrokeKind::Middle);
}

/// A drag from the building panel over the canvas: highlight where it
/// would land (green allowed, red refused), tint the ghost chip, and push
/// [`Action::Insert`] when it is released on an allowed target.
pub(super) fn panel_drop(
    ui: &egui::Ui,
    response: &Response,
    layout: &Layout,
    tab: &DocTab,
    tk: &Tokens,
    pending: &mut Vec<Action>,
) {
    let Some(what) = response.dnd_hover_payload::<Insertable>() else {
        return;
    };
    let Some(pos) = ui.input(|i| i.pointer.latest_pos()) else {
        return;
    };
    let cam = tab.camera;
    let world = cam.screen_to_world(pos);
    let Some((target, allowed)) = panel_drop_target(&tab.doc, layout, &what, world, cam.zoom)
    else {
        return;
    };
    if response.dnd_release_payload::<Insertable>().is_some() {
        if allowed && let Some(action) = panel_drop_action(&what, &target) {
            pending.push(action);
        }
        return;
    }
    let colour = if allowed { tk.reason } else { tk.objection };
    if let Some(rect) = target.highlight {
        paint_target(&ui.painter_at(response.rect), tab, rect, colour);
    }
    crate::shell::building::tint_ghost(ui.ctx(), colour);
    if !allowed {
        ui.ctx().set_cursor_icon(egui::CursorIcon::NotAllowed);
    }
}

/// The dragged item's outline, following the pointer.
fn paint_ghost(
    painter: &Painter,
    layout: &Layout,
    tab: &DocTab,
    colour: Color32,
    s: &DragState,
    pos: Pos2,
) {
    let world = match s.item {
        Item::Box(b) => layout.boxes.get(&b),
        Item::Card(c) => layout.cards.get(&c),
    };
    if let Some(&world) = world {
        let r = tab
            .camera
            .world_rect_to_screen(world)
            .translate(pos - s.origin);
        let radius = FRAME_RADIUS * tab.camera.zoom;
        painter.rect_filled(r, radius, colour.gamma_multiply(0.08));
        painter.rect_stroke(r, radius, Stroke::new(1.5, colour), StrokeKind::Middle);
    }
}

#[cfg(test)]
mod tests;
