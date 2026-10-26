//! Box sizes: measured text, image and indicator row.

use std::collections::HashMap;

use emath::Vec2;

use super::{BOX_TEXT_PAD, LayoutParams, Measure, image_height, indicators};
use crate::ids::NodeId;
use crate::model::{BoxKind, BoxNode, Document};

/// Sizes the boxes of one layout.
pub(super) struct Sizer<'a> {
    m: &'a dyn Measure,
    p: &'a LayoutParams,
    /// The indicator row height of each box that has one.
    pub rows: HashMap<NodeId, f32>,
}

impl<'a> Sizer<'a> {
    pub(super) fn new(doc: &Document, m: &'a dyn Measure, p: &'a LayoutParams) -> Self {
        Self {
            m,
            p,
            rows: indicators::rows(doc, &p.indicators, p.indicator_row),
        }
    }

    /// Size of a box from its measured text, its image and its indicator row.
    pub(super) fn box_size(&self, node: &BoxNode) -> Vec2 {
        let p = self.p;
        let is_source = matches!(node.kind, BoxKind::Source { .. });
        let default_width = if is_source {
            p.source_width
        } else {
            p.box_width
        };
        let width = node.width.unwrap_or(default_width);
        let text_height = self
            .m
            .text_height(&node.text, width - 2.0 * BOX_TEXT_PAD, &node.kind);
        let image_height = node
            .image
            .as_ref()
            .map_or(0.0, |img| image_height(width, img));
        let height = if is_source {
            // Room for the icon and the source type line.
            text_height + 30.0 + image_height
        } else {
            p.min_box_height
                .max(text_height + 2.0 * BOX_TEXT_PAD + image_height)
        };
        let row = self.rows.get(&node.id).copied().unwrap_or(0.0);
        Vec2::new(width, height + row)
    }
}
