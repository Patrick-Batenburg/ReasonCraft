//! Drag and drop: drop a box or card on a box (connect), on a card's bottom
//! edge (target the whole card) or on empty canvas (detach to parking).

use eframe::egui::{
    self, Color32, Painter, PointerButton, Pos2, Rect, Response, Stroke, StrokeKind,
};
use reasoncraft_core::{BoxKind, Document, Item, Layout, Parent};

use super::hit::{BOTTOM_EDGE_BAND, Hit, hit_test};
use super::paint::{FRAME_RADIUS, parking_frame};
use crate::actions::{Action, is_enabled};
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
            highlight: layout.cards.get(&c).map(|r| {
                let half = BOTTOM_EDGE_BAND / zoom / 2.0;
                Rect::from_x_y_ranges(r.x_range(), (r.bottom() - half)..=(r.bottom() + half))
            }),
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
                    let r = cam.world_rect_to_screen(rect).expand(3.0);
                    painter.rect_stroke(r, 6.0, Stroke::new(2.5, colour), StrokeKind::Middle);
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
mod tests {
    use super::*;
    use eframe::egui::vec2;
    use reasoncraft_core::fixtures::FixedMeasure;
    use reasoncraft_core::{CardKind, LayoutParams, NewLoose, layout};

    #[test]
    fn drop_targets_connect_reattach_and_park() {
        let mut d = Document::new("Claim");
        let root = d.root;
        let (a, p) = d
            .add_card(Parent::Box(root), CardKind::Reason, "p")
            .unwrap();
        let p2 = d.add_premise(a, None, "p2").unwrap();
        let Item::Card(loose) = d.add_loose(NewLoose::Card(CardKind::Objection), "q") else {
            unreachable!()
        };
        let q = d.cards[&loose].premises[0];
        let l = layout(&d, &FixedMeasure, &LayoutParams::default());
        let target = |item, world| drop_target(&d, &l, item, world, 1.0).map(|t| t.to);

        // The root is fixed; a sole premise drags its card; a co-premise drags itself.
        assert_eq!(drag_item(&d, Hit::Box(root)), None);
        assert_eq!(drag_item(&d, Hit::Box(q)), Some(Item::Card(loose)));
        assert_eq!(drag_item(&d, Hit::Box(p)), Some(Item::Box(p)));

        let lc = Item::Card(loose);
        assert_eq!(target(lc, l.boxes[&root].center()), Some(Parent::Box(root)));
        assert_eq!(target(lc, l.boxes[&p].center()), Some(Parent::Box(p)));
        assert_eq!(
            target(lc, l.cards[&a].center_bottom()),
            Some(Parent::Card(a))
        );
        let empty = l.bounds.right_bottom() + vec2(500.0, 500.0);
        assert_eq!(target(lc, empty), None, "already parked");
        assert_eq!(target(lc, l.boxes[&q].center()), None, "onto itself");

        let ac = Item::Card(a);
        assert_eq!(target(ac, empty), Some(Parent::Parking));
        assert_eq!(
            target(ac, l.boxes[&p2].center()),
            None,
            "onto its own premise"
        );
        // A co-premise dropped on another card's premise joins that card.
        assert_eq!(
            target(Item::Box(p2), l.boxes[&q].center()),
            Some(Parent::Card(loose))
        );
        assert_eq!(target(Item::Box(p2), l.boxes[&p].center()), None);
    }
}
