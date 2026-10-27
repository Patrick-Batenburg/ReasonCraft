//! Box sizes: measured text, image and indicator row.
//!
//! A box is also long enough along the cross axis (its width top-down and
//! bottom-up, its height left-to-right) for the ports on its far edge
//! `port_spacing` apart within `port_margin` of its corners, so their
//! arrowheads and bars never overlap. A wider box has its text measured at
//! its new width. The ports are counted per group as `ports` spreads them.

use std::collections::HashMap;

use emath::Vec2;

use super::ports::{WholePorts, whole_ports};
use super::{BOX_TEXT_PAD, LayoutParams, Measure, image_height, indicators};
use crate::ids::NodeId;
use crate::model::{BoxKind, BoxNode, Document, LayoutDirection};

/// Sizes the boxes of one layout.
pub(super) struct Sizer<'a> {
    m: &'a dyn Measure,
    p: &'a LayoutParams,
    /// Each box's indicator row and number.
    pub marks: indicators::Marks,
    /// The cross length each box's far edge needs for its ports (only
    /// boxes that need more than they may have).
    edges: HashMap<NodeId, f32>,
    /// Whether the cross axis is the height (left-to-right).
    cross_is_height: bool,
}

impl<'a> Sizer<'a> {
    pub(super) fn new(doc: &Document, m: &'a dyn Measure, p: &'a LayoutParams) -> Self {
        Self {
            m,
            p,
            marks: indicators::Marks::new(doc, &p.indicators, p.indicator_row),
            edges: edge_needs(doc, p),
            cross_is_height: doc.layout_direction == LayoutDirection::LeftRight,
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
        let edge = self.edges.get(&node.id).copied().unwrap_or(0.0);
        let mut width = node.width.unwrap_or(default_width);
        if !self.cross_is_height {
            width = width.max(edge);
        }
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
        let row = self.marks.rows.get(&node.id).copied().unwrap_or(0.0);
        let mut height = height + row;
        if self.cross_is_height {
            height = height.max(edge);
        }
        Vec2::new(width, height)
    }
}

/// The cross length the far edge of each box with ports needs: room for
/// its group's ports `port_spacing` apart within `port_margin` of each
/// corner. The middle premises of a card with an even number of premises
/// also keep their ports a spacing clear of the whole-card ports spread
/// about the gap between them.
fn edge_needs(doc: &Document, p: &LayoutParams) -> HashMap<NodeId, f32> {
    let mut ports: HashMap<NodeId, usize> = doc
        .boxes
        .iter()
        .filter(|(_, node)| !node.children.is_empty())
        .map(|(&id, node)| (id, node.children.len()))
        .collect();
    // Extra room on each side of a premise beside whole-card ports.
    let mut clear: HashMap<NodeId, f32> = HashMap::new();
    for card in doc.cards.values() {
        let whole = card.children.len();
        if whole == 0 {
            continue;
        }
        match whole_ports(doc, card) {
            WholePorts::Premise(id) => *ports.entry(id).or_default() += whole,
            WholePorts::Between(a, b) => {
                // The outermost whole-card port lies this far from the gap
                // centre, the innermost premise port at least
                // `premise_gap / 2 + port_margin + extra`.
                let half = (whole - 1) as f32 * p.port_spacing / 2.0;
                let extra = half + p.port_spacing - p.premise_gap / 2.0 - p.port_margin;
                for id in [a, b] {
                    clear.insert(id, extra.max(0.0));
                }
            }
            WholePorts::Frame => {}
        }
    }
    ports
        .into_iter()
        .map(|(id, n)| {
            let extra = clear.get(&id).copied().unwrap_or(0.0);
            let need = (n - 1) as f32 * p.port_spacing + 2.0 * (p.port_margin + extra);
            (id, need)
        })
        .collect()
}
