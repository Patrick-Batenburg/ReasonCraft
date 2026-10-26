//! The map's numbering: each numbered box shows its outline number at the
//! left of its indicator row, and its link glyph and comment badge move
//! right to make room for it.

use std::collections::HashMap;

use eframe::egui::{Align, Rect, pos2, vec2};
use reasoncraft_core::outline::box_numbers;
use reasoncraft_core::{Document, Item, Layout, NodeId};

use super::build::Builder;
use super::{Shape, TextRows};
use crate::canvas::measure::BOX_FONT_SIZE;
use crate::canvas::paint::screen_font;
use crate::canvas::rating::{CORNER_INSET, indicator_centre};

/// The numbers' text size: 0.8 × the box text size.
pub const NUMBER_SIZE: f32 = 0.8 * BOX_FONT_SIZE;

/// The number each numbered item shows: boxes get their outline number
/// (premises carry their card's, e.g. "1a"); cards get none.
pub fn number_labels(doc: &Document) -> HashMap<Item, String> {
    box_numbers(doc)
        .into_iter()
        .map(|(id, number)| (Item::Box(id), number))
        .collect()
}

/// Where `number` goes in the box at `world` (world units) whose indicator
/// row is `row` high, drawn at `zoom`: at the row's left end,
/// [`CORNER_INSET`] from the box's left edge, centred on the row like the
/// other marks. Measured at the screen font size it is drawn at.
pub fn number_rect(world: Rect, row: f32, number: &str, fonts: &dyn TextRows, zoom: f32) -> Rect {
    let size = screen_font(NUMBER_SIZE, zoom).map_or(NUMBER_SIZE * zoom, |f| f.size);
    let width = fonts.text_width(number, size) / zoom;
    let height = fonts.line_height(size) / zoom;
    let top = indicator_centre(world, row) - height / 2.0;
    Rect::from_min_size(pos2(world.left() + CORNER_INSET, top), vec2(width, height))
}

/// How far right the link glyph and comment badge of each numbered box in
/// `layout` move to clear its number (its width and a gap), at `zoom`.
pub fn marks_leads(
    doc: &Document,
    layout: &Layout,
    fonts: &dyn TextRows,
    zoom: f32,
) -> HashMap<NodeId, f32> {
    box_numbers(doc)
        .into_iter()
        .filter_map(|(id, number)| {
            let world = *layout.boxes.get(&id)?;
            let row = layout.indicator_row(id);
            let lead = number_rect(world, row, &number, fonts, zoom).width() + CORNER_INSET;
            (row > 0.0).then_some((id, lead))
        })
        .collect()
}

impl Builder<'_> {
    /// The muted outline `number` of the box at `world`, in its indicator
    /// row (`row` high). Left out when too small to read.
    pub(super) fn number(&mut self, world: Rect, row: f32, number: &str) {
        let zoom = self.opts.zoom;
        if screen_font(NUMBER_SIZE, zoom).is_none() {
            return;
        }
        let rect = number_rect(world, row, number, self.fonts, zoom);
        if !self.visible(rect) {
            return;
        }
        self.shapes.push(Shape::Text {
            origin: rect.min,
            rows: vec![number.to_owned()],
            size: NUMBER_SIZE,
            bold: false,
            color: self.opts.tokens.text_muted,
            align: Align::Min,
            line_height: self.fonts.line_height(NUMBER_SIZE),
        });
    }
}
