//! Drag and drop: drop a box or card on a box (connect), on a card's far
//! edge (target the whole card), between siblings (reorder, shown by an
//! insertion bar) or on empty canvas (detach to parking).

use eframe::egui::{
    self, Color32, Painter, PointerButton, Pos2, Rect, Response, Stroke, StrokeKind,
};
use reasoncraft_core::{
    BoxKind, Document, Item, Layout, LayoutDirection, LayoutParams, Parent, same_display_group,
};

use super::hit::{Hit, far_edge_band, hit_test};
use crate::actions::{Action, is_enabled, moves_nowhere, sibling_list};
use crate::doc_tab::DocTab;
use crate::scene::metrics::{FRAME_RADIUS, parking_frame};
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

/// Width of the insertion bar between siblings, in screen px.
const BAR_PX: f32 = 3.0;

/// Where a drop would put the dragged item, and what to highlight (world units).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DropTarget {
    pub to: Parent,
    /// The index in `to`'s children ([`Action::Move`]); `None` appends.
    pub index: Option<usize>,
    pub highlight: Option<Rect>,
    /// The insertion bar between siblings: a zero-width rect (world units).
    pub bar: Option<Rect>,
}

impl DropTarget {
    fn new(to: Parent, highlight: Option<Rect>) -> Self {
        Self {
            to,
            index: None,
            highlight,
            bar: None,
        }
    }
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
/// frame or its far edge takes it as a whole-card child, the gap between
/// a parent's children (or just past either end) takes it there, and empty
/// canvas parks it. The index among the new siblings comes from the
/// pointer's position along the siblings (x, or y left-to-right), so
/// dropping on the item's own parent at a new place reorders it. `None` when nothing would change.
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
    let mut target = match hit_test(layout, world, zoom) {
        Some(Hit::Box(b)) => {
            if inside_item(b) {
                return None;
            }
            let to = match doc.owner_of(Item::Box(b)) {
                Some(Parent::Card(c)) if is_premise => Parent::Card(c),
                _ => Parent::Box(b),
            };
            DropTarget::new(to, layout.boxes.get(&b).copied())
        }
        Some(Hit::CardFarEdge(c)) => DropTarget::new(
            Parent::Card(c),
            layout
                .cards
                .get(&c)
                .map(|&r| far_edge_band(r, layout.dir, zoom)),
        ),
        Some(Hit::CardFrame(c)) => DropTarget::new(Parent::Card(c), layout.cards.get(&c).copied()),
        // Premises go into cards only, never into a row of box children.
        None => match gap_row(doc, layout, world).filter(|_| !is_premise) {
            Some(to) => DropTarget::new(to, parent_item(to).and_then(|p| item_rect(layout, p))),
            None => DropTarget::new(Parent::Parking, parking_frame(layout)),
        },
    };
    if target.to != Parent::Parking {
        (target.index, target.bar) = insertion(doc, layout, item, target.to, world, is_premise);
    }
    // Back where it is (as shown), or a card onto itself: nothing would change.
    let onto_itself = matches!((item, target.to), (Item::Card(a), Parent::Card(b)) if a == b);
    let same_place = moves_nowhere(doc, item, target.to, target.index);
    (!(onto_itself || same_place)).then_some(target)
}

/// The box or card standing for parent `to` (none for parking).
fn parent_item(to: Parent) -> Option<Item> {
    match to {
        Parent::Box(b) => Some(Item::Box(b)),
        Parent::Card(c) => Some(Item::Card(c)),
        Parent::Parking => None,
    }
}

/// A box's rect, or a card's frame (world units).
fn item_rect(layout: &Layout, item: Item) -> Option<Rect> {
    match item {
        Item::Box(b) => layout.boxes.get(&b).copied(),
        Item::Card(c) => layout.cards.get(&c).copied(),
    }
}

/// The parent whose row of children holds `world`, a point on no item:
/// between two of the children or within half a gap past either end. Of
/// overlapping rows the most nested (furthest from the root) wins.
fn gap_row(doc: &Document, layout: &Layout, world: Pos2) -> Option<Parent> {
    let half_gap = LayoutParams::default().h_gap / 2.0;
    let along = Along(layout.dir);
    let row = |to: Parent, children: &mut dyn Iterator<Item = Item>| {
        let r = children
            .filter_map(|c| item_rect(layout, c))
            .reduce(Rect::union)?;
        r.expand2(along.vec(half_gap))
            .contains(world)
            .then_some((along.depth(r), to))
    };
    let boxes = doc
        .boxes
        .values()
        .filter_map(|n| row(Parent::Box(n.id), &mut n.children.iter().copied()));
    let cards = doc.cards.values().filter_map(|k| {
        row(
            Parent::Card(k.id),
            &mut k.children.iter().map(|&c| Item::Card(c)),
        )
    });
    boxes
        .chain(cards)
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, to)| to)
}

/// The world axis siblings follow each other along in a layout direction:
/// x, or y left-to-right.
#[derive(Clone, Copy)]
struct Along(LayoutDirection);

impl Along {
    fn is_y(self) -> bool {
        self.0 == LayoutDirection::LeftRight
    }
    /// `p`'s coordinate along the siblings.
    fn of(self, p: Pos2) -> f32 {
        if self.is_y() { p.y } else { p.x }
    }
    /// `r`'s `(low, high)` range along the siblings.
    fn range(self, r: Rect) -> (f32, f32) {
        if self.is_y() {
            (r.top(), r.bottom())
        } else {
            (r.left(), r.right())
        }
    }
    /// A vector of length `v` along the siblings.
    fn vec(self, v: f32) -> egui::Vec2 {
        if self.is_y() {
            egui::vec2(0.0, v)
        } else {
            egui::vec2(v, 0.0)
        }
    }
    /// How far `r`'s near edge lies from the root, along the main axis.
    fn depth(self, r: Rect) -> f32 {
        match self.0 {
            LayoutDirection::TopDown => r.top(),
            LayoutDirection::BottomUp => -r.bottom(),
            LayoutDirection::LeftRight => r.left(),
        }
    }
    /// A zero-thickness bar at `at` along the siblings, spanning `across`
    /// (the union of the neighbours' rects).
    fn bar(self, at: f32, across: Rect) -> Rect {
        if self.is_y() {
            Rect::from_x_y_ranges(across.x_range(), at..=at)
        } else {
            Rect::from_x_y_ranges(at..=at, across.y_range())
        }
    }
}

/// The index `item` would get among `to`'s children for a pointer at
/// `world`, and the insertion bar (world units) showing where. The siblings
/// counted are those shown in the same group as `item` (under a box,
/// sources and notes come before everything else), so display order and
/// document order agree among them; the index is in document order with
/// `item` taken out. `(None, None)` when `item` would be alone in its group.
fn insertion(
    doc: &Document,
    layout: &Layout,
    item: Item,
    to: Parent,
    world: Pos2,
    premise: bool,
) -> (Option<usize>, Option<Rect>) {
    let list: Vec<Item> = sibling_list(doc, to, premise)
        .into_iter()
        .filter(|&i| i != item)
        .collect();
    let group = |i: Item| same_display_group(doc, to, i, item);
    let shown: Vec<(usize, Rect)> = list
        .iter()
        .enumerate()
        .filter(|&(_, &i)| group(i))
        .filter_map(|(at, &i)| Some((at, item_rect(layout, i)?)))
        .collect();
    let (Some(&(_, first)), Some(&(last_at, last))) = (shown.first(), shown.last()) else {
        return (None, None);
    };
    // Siblings whose centre comes before the pointer stay before the item.
    let along = Along(layout.dir);
    let pointer = along.of(world);
    let k = shown
        .iter()
        .filter(|(_, r)| along.of(r.center()) < pointer)
        .count();
    let index = shown.get(k).map_or(last_at + 1, |&(at, _)| at);
    // Half the gap the layout leaves between these siblings.
    let p = LayoutParams::default();
    let half_gap = if premise { p.premise_gap } else { p.h_gap } / 2.0;
    let before = k.checked_sub(1).and_then(|i| shown.get(i));
    let bar = match (before, shown.get(k)) {
        (Some(&(_, a)), Some(&(_, b))) => {
            along.bar((along.range(a).1 + along.range(b).0) / 2.0, a.union(b))
        }
        (None, _) => along.bar(along.range(first).0 - half_gap, first),
        (Some(_), None) => along.bar(along.range(last).1 + half_gap, last),
    };
    (Some(index), Some(bar))
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
            let action = target.map(|d| Action::Move {
                item: s.item,
                to: d.to,
                index: d.index,
            });
            let allowed = action.as_ref().is_some_and(|a| is_enabled(tab, a));
            if released {
                if let (true, Some(a)) = (allowed, action) {
                    pending.push(a);
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
                if let Some(bar) = target.and_then(|d| d.bar).filter(|_| allowed) {
                    let r = tab.camera.world_rect_to_screen(bar);
                    painter.rect_filled(r.expand(BAR_PX / 2.0), 1.0, tk.accent);
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
pub(super) fn paint_target(painter: &Painter, tab: &DocTab, world: Rect, colour: Color32) {
    let r = tab.camera.world_rect_to_screen(world).expand(3.0);
    painter.rect_stroke(r, 6.0, Stroke::new(2.5, colour), StrokeKind::Middle);
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

mod autoscroll;
mod panel;
pub(super) use autoscroll::autoscroll;
pub use autoscroll::{EDGE_PX, SPEED_PX, autoscroll_pan};
pub(super) use panel::panel_drop;
pub use panel::{panel_drop_action, panel_drop_target};

#[cfg(test)]
mod tests;
