//! Drops from the building panel onto the canvas.

use eframe::egui::{self, DragAndDrop, Pos2, Response};
use reasoncraft_core::{Document, Item, Layout, NodeId, Parent};

use super::{DropTarget, bottom_band, paint_target};
use crate::actions::{Action, Insertable, target_allowed};
use crate::canvas::hit::{Hit, hit_test};
use crate::canvas::paint::parking_frame;
use crate::doc_tab::DocTab;
use crate::theme::Tokens;

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
            Some(Parent::Card(c)) if *what == Insertable::CoPremise => {
                DropTarget::new(Parent::Box(b), card(c))
            }
            _ => DropTarget::new(Parent::Box(b), layout.boxes.get(&b).copied()),
        },
        Some(Hit::CardFrame(c) | Hit::CardBottomEdge(c)) if *what == Insertable::CoPremise => {
            DropTarget::new(Parent::Box(*doc.cards.get(&c)?.premises.first()?), card(c))
        }
        Some(Hit::CardBottomEdge(c)) => {
            DropTarget::new(Parent::Card(c), card(c).map(|r| bottom_band(r, zoom)))
        }
        Some(Hit::CardFrame(c)) => DropTarget::new(Parent::Card(c), card(c)),
        None => DropTarget::new(Parent::Parking, parking_frame(layout)),
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

/// A drag from the building panel over the canvas: highlight where it
/// would land (green allowed, red refused), tint the ghost chip, and push
/// [`Action::Insert`] when it is released on an allowed target.
pub(in crate::canvas) fn panel_drop(
    ui: &egui::Ui,
    response: &Response,
    layout: &Layout,
    tab: &DocTab,
    tk: &Tokens,
    pending: &mut Vec<Action>,
) {
    // By the canvas rect and layer, not `response.contains_pointer()`, so
    // the widgets on top of the canvas in its own layer (the inline editor,
    // comment badges) never hide a drop; popovers and menus in layers above
    // do.
    let ctx = ui.ctx();
    if !ui.rect_contains_pointer(response.rect) {
        return;
    }
    let Some(what) = DragAndDrop::payload::<Insertable>(ctx) else {
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
    if ui.input(|i| i.pointer.primary_released())
        && DragAndDrop::take_payload::<Insertable>(ctx).is_some()
    {
        if allowed && let Some(action) = panel_drop_action(&what, &target) {
            pending.push(action);
        }
        return;
    }
    let colour = if allowed { tk.reason } else { tk.objection };
    if let Some(rect) = target.highlight {
        paint_target(&ui.painter_at(response.rect), tab, rect, colour);
    }
    crate::shell::building::tint_ghost(ctx, colour);
    if !allowed {
        ctx.set_cursor_icon(egui::CursorIcon::NotAllowed);
    }
}
