//! A vertical list of rows of any height that lays out only the rows near
//! the view: the others are placed from their cached heights (see
//! [`RowHeights`]), so a large map scrolls smoothly.

use std::fmt::Debug;
use std::hash::Hash;

use eframe::egui::{Rect, ScrollArea, Ui, UiBuilder};

use crate::doc_tab::RowHeights;

/// The rows to show, how to draw one, and what they are measured for.
pub(super) struct Rows<'a, K, D> {
    pub keys: &'a [K],
    /// The height assumed for a row never laid out.
    pub estimate: f32,
    /// The document revision the rows show.
    pub revision: u64,
    /// Rows laid out wherever they are (a field with keyboard focus, so
    /// it keeps the focus while scrolled away).
    pub keep: Option<&'a K>,
    /// Draws the row `keys[i]`, given `i`.
    pub draw: D,
}

/// Shows `rows` in a vertical scroll area. Only rows that meet the
/// viewport grown by one screen above and below are laid out (and
/// measured into `heights`).
pub(super) fn show<K, D>(ui: &mut Ui, heights: &mut RowHeights<K>, rows: Rows<K, D>)
where
    K: Clone + Eq + Hash + Debug,
    D: FnMut(&mut Ui, usize),
{
    let Rows {
        keys,
        estimate,
        revision,
        keep,
        mut draw,
    } = rows;
    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_viewport(ui, |ui, viewport| {
            let area = ui.max_rect();
            heights.start_frame(revision, area.width());
            let gap = ui.spacing().item_spacing.y;
            let screen = viewport.height();
            let near = (viewport.top() - screen)..=(viewport.bottom() + screen);
            let mut y = 0.0;
            for (i, key) in keys.iter().enumerate() {
                let known = heights.get(key).unwrap_or(estimate);
                let shown = (y + known >= *near.start() && y <= *near.end()) || keep == Some(key);
                let height = if shown {
                    let top = area.top() + y;
                    let rect = Rect::from_x_y_ranges(area.x_range(), top..=top + known.max(1.0));
                    let builder = UiBuilder::new().max_rect(rect).id_salt(key);
                    let height = ui
                        .scope_builder(builder, |ui| draw(ui, i))
                        .response
                        .rect
                        .height();
                    heights.measured(key.clone(), height);
                    height
                } else {
                    known
                };
                y += height + gap;
            }
            ui.set_min_height((y - gap).max(0.0));
        });
}
