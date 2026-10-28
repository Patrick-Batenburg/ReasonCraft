//! A vertical list of rows of any height that lays out only the rows near
//! the view: the others are placed from their cached heights (see
//! [`RowHeights`]), so a large map scrolls smoothly.
//!
//! When rows above the view are measured at heights other than assumed,
//! the list scrolls by the difference, so what is in view stays put.
//!
//! The keyboard still reaches every row: the row holding the keyboard
//! focus stays laid out wherever it is, with its neighbours, so Tab and
//! Shift+Tab go on to the next row; and when the focus moves to a row,
//! the list scrolls to show it.

use std::fmt::Debug;
use std::hash::Hash;

use eframe::egui::{Id, Rect, ScrollArea, Ui, UiBuilder};

use crate::doc_tab::RowHeights;

/// How many rows before and after the focused row stay laid out with it:
/// enough to reach the next row with a focusable widget past a heading.
const FOCUS_REACH: usize = 2;
/// The least height change, in px, of the rows above the view that the
/// list scrolls along with.
const MIN_SHIFT: f32 = 0.1;

/// The rows to show, how to draw one, and what they are measured for.
pub(super) struct Rows<'a, K, D> {
    pub keys: &'a [K],
    /// The height assumed for a row never laid out.
    pub estimate: f32,
    /// The document revision the rows show.
    pub revision: u64,
    /// A row laid out wherever it is (one asking for the keyboard focus,
    /// or being dragged), so it can act on it.
    pub pinned: Option<usize>,
    /// Draws the row `keys[i]`, given `i`.
    pub draw: D,
}

/// Whether the widget `id` was shown this frame so far.
fn shown(ui: &Ui, id: Id) -> bool {
    ui.ctx().viewport(|v| v.this_pass.widgets.get(id).is_some())
}

/// Shows `rows` in a vertical scroll area. Only rows that meet the
/// viewport grown by one screen above and below are laid out (and
/// measured into `heights`), the rows around the keyboard focus and the
/// pinned row. Returns where every row is on screen, laid out or not.
pub(super) fn show<K, D>(ui: &mut Ui, heights: &mut RowHeights<K>, rows: Rows<K, D>) -> Vec<Rect>
where
    K: Clone + Eq + Hash + Debug,
    D: FnMut(&mut Ui, usize),
{
    let Rows {
        keys,
        estimate,
        revision,
        pinned,
        mut draw,
    } = rows;
    let focus = ui.memory(|m| m.focused());
    let last = heights.focus.take();
    let focus_at = last
        .as_ref()
        .filter(|_| focus.is_some())
        .and_then(|(key, _)| keys.iter().position(|k| k == key));
    let keep = focus_at.map(|i| i.saturating_sub(FOCUS_REACH)..=i + FOCUS_REACH);
    let reveal = (heights.reveal.as_ref()).and_then(|key| keys.iter().position(|k| k == key));
    if reveal.is_none() {
        heights.reveal = None;
    }
    let mut area = ScrollArea::vertical().auto_shrink([false, false]);
    if let Some(y) = heights.scroll_to.take() {
        area = area.vertical_scroll_offset(y);
    }
    let out = area.show_viewport(ui, |ui, viewport| {
        let area = ui.max_rect();
        heights.start_frame(revision, area.width(), keys);
        let mut placed = Vec::with_capacity(keys.len());
        let gap = ui.spacing().item_spacing.y;
        let screen = viewport.height();
        let near = (viewport.top() - screen)..=(viewport.bottom() + screen);
        let mut focused_rect = None;
        let mut y = 0.0;
        // How much the rows wholly above the view grew when measured.
        let mut shift = 0.0;
        for (i, key) in keys.iter().enumerate() {
            let known = heights.get(key).unwrap_or(estimate);
            let in_view = y + known >= *near.start() && y <= *near.end();
            let kept = keep.as_ref().is_some_and(|k| k.contains(&i))
                || pinned == Some(i)
                || reveal == Some(i);
            let height = if in_view || kept {
                let top = area.top() + y;
                let rect = Rect::from_x_y_ranges(area.x_range(), top..=top + known.max(1.0));
                let builder = UiBuilder::new().max_rect(rect).id_salt(key);
                let had_focus = focus.is_some_and(|id| shown(ui, id));
                let height = ui
                    .scope_builder(builder, |ui| draw(ui, i))
                    .response
                    .rect
                    .height();
                if let Some(id) = focus.filter(|&id| !had_focus && shown(ui, id)) {
                    heights.focus = Some((key.clone(), id));
                    focused_rect = ui.ctx().read_response(id).map(|r| r.rect);
                }
                heights.measured(key.clone(), height);
                if y + known <= viewport.top() + shift {
                    shift += height - known;
                }
                height
            } else {
                known
            };
            let top = area.top() + y;
            placed.push(Rect::from_x_y_ranges(area.x_range(), top..=top + height));
            y += height + gap;
        }
        ui.set_min_height((y - gap).max(0.0));
        if shift.abs() > MIN_SHIFT {
            // This pass is discarded (below): keep the focus as it was, so
            // the pass that replaces it sees a move and scrolls to it.
            heights.focus = last.clone();
            return (shift, placed);
        }
        if let Some(i) = reveal {
            // An absolute offset, taken at once on the next frame (which
            // measures the rows around it and may move it again): the same
            // in every pass of a frame.
            // A row taller than the view shows its top.
            let (top, bottom) = (
                placed[i].top() - area.top(),
                placed[i].bottom() - area.top(),
            );
            let target = if top < viewport.top() || bottom - top > viewport.height() {
                top
            } else {
                viewport.top().max(bottom - viewport.height())
            };
            if (target - viewport.top()).abs() < 0.5 {
                heights.reveal = None;
            } else {
                heights.scroll_to = Some(target);
                ui.ctx().request_repaint();
            }
            return (0.0, placed);
        }
        // The focus came to a row of this list (Tab, or a click):
        // show it.
        let moved = last.as_ref().map(|(_, id)| *id) != heights.focus.as_ref().map(|(_, id)| *id);
        if let Some(rect) = focused_rect.filter(|_| moved) {
            ui.scroll_to_rect(rect, None);
        }
        (0.0, placed)
    });
    let (shift, placed) = out.inner;
    if shift != 0.0 {
        // Scroll along with the rows, and lay out this frame again so it
        // never shows them moved. The app's first use of a discarded
        // pass: egui runs the whole UI again in the same frame, and only
        // the second pass is painted. What the first pass changed in the
        // app's own state stays changed, so anything that acts once per
        // pass must check `will_discard` (the map's autoscroll does).
        heights.scroll_to = Some(out.state.unclamped_offset().y + shift);
        ui.ctx()
            .request_discard("virtual rows measured above the view");
    }
    placed
}

#[cfg(test)]
#[path = "virtual_rows_tests.rs"]
mod tests;
