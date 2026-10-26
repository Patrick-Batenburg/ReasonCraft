//! Automatic compact tidy-tree layout of a [`Document`].
//!
//! [`layout`] places the main tree (from the root claim), then each parking
//! item as its own mini-tree, in a column after the main tree along the
//! cross axis (to the right top-down and bottom-up, below left-to-right).
//! Subtrees are packed against each other's contours. The document's
//! [`LayoutDirection`] picks the world axes; the main tree's top-left
//! corner ends up at the origin. Coordinates grow downward like the screen.

mod axis;
mod card;
mod contour;
mod indicators;
mod ports;
mod row;
mod size;
mod tree;

pub use indicators::{IndicatorOptions, needs_indicator_row};
pub(crate) use tree::display_children;

use emath::{Pos2, Rect, Vec2};
use indexmap::IndexMap;

use crate::connector::Strength;
use crate::ids::{CardId, Item, NodeId};
use crate::model::{BoxKind, CardStrength, Document, ImageRef, LayoutDirection, Role};

/// Horizontal and vertical padding between a box's edge and its text.
/// A [`Measure`] is asked for the height of text wrapped at
/// `box width - 2 * BOX_TEXT_PAD`; painters must wrap at the same width.
pub const BOX_TEXT_PAD: f32 = 6.0;

/// Vertical gap between a box's image and its text.
pub const IMAGE_GAP: f32 = 6.0;

/// The height a box `width` wide reserves above its text for `image`:
/// padding, the image at the text width, and [`IMAGE_GAP`]. Painters draw
/// the image at `(BOX_TEXT_PAD, BOX_TEXT_PAD)` from the box's top-left,
/// `width - 2 * BOX_TEXT_PAD` wide, and the box's content below this block.
pub fn image_height(width: f32, image: &ImageRef) -> f32 {
    (width - 2.0 * BOX_TEXT_PAD) * image.aspect.clamp(0.05, 4.0) + BOX_TEXT_PAD + IMAGE_GAP
}

/// Measures wrapped text. Implemented by the UI with its real fonts.
pub trait Measure {
    /// Height of `text` wrapped to `width`, breaking words that do not fit.
    fn text_height(&self, text: &str, width: f32, kind: &BoxKind) -> f32;
}

/// Sizes and gaps used by [`layout`].
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutParams {
    pub box_width: f32,
    pub source_width: f32,
    pub min_box_height: f32,
    pub card_pad: f32,
    pub card_label_height: f32,
    pub premise_gap: f32,
    pub h_gap: f32,
    pub v_gap: f32,
    pub port_spacing: f32,
    pub port_margin: f32,
    pub parking_gap: f32,
    /// Height of the indicator row a box with marks gets above its
    /// content (see [`needs_indicator_row`]).
    pub indicator_row: f32,
    /// Which marks boxes show.
    pub indicators: IndicatorOptions,
}

impl Default for LayoutParams {
    fn default() -> Self {
        Self {
            box_width: 156.0,
            source_width: 128.0,
            min_box_height: 40.0,
            card_pad: 8.0,
            card_label_height: 24.0,
            premise_gap: 8.0,
            h_gap: 18.0,
            v_gap: 70.0,
            port_spacing: 16.0,
            port_margin: 15.0,
            parking_gap: 120.0,
            indicator_row: 16.0,
            indicators: IndicatorOptions::default(),
        }
    }
}

/// How a connector is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectorKind {
    Supports,
    Opposes,
    Rebuts,
    /// Grouping lines (category to category or claim).
    Neutral,
    /// Source and note lines.
    Attachment,
}

/// What a connector points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    /// A box (including a premise inside a card).
    Box,
    /// A whole card frame.
    WholeCard,
}

/// A line from a child item up to its parent.
#[derive(Debug, Clone, PartialEq)]
pub struct Connector {
    pub child: Item,
    /// The centre of the child's edge toward its parent (a card's frame
    /// edge): its top centre top-down.
    pub from: Pos2,
    /// The parent port.
    pub to: Pos2,
    pub kind: ConnectorKind,
    pub target: TargetKind,
    /// Evaluated strength of a card's connector; `Normal` for other items.
    pub strength: Strength,
    /// The card is rated nil (drawn weak, flagged for a distinct look).
    pub nil: bool,
    /// The layout direction: the connector leaves its parent along
    /// `-dir.toward_parent()`.
    pub dir: LayoutDirection,
}

/// The result of [`layout`].
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    /// The direction the layout was made in.
    pub dir: LayoutDirection,
    /// Every box, premises included.
    pub boxes: IndexMap<NodeId, Rect>,
    /// Card frame rects.
    pub cards: IndexMap<CardId, Rect>,
    /// Top-right anchor inside each frame for the right-aligned rating label.
    pub card_labels: IndexMap<CardId, Pos2>,
    /// Role of every card.
    pub card_roles: IndexMap<CardId, Role>,
    /// Rating of every card (`None` when unrated).
    pub card_strength: IndexMap<CardId, Option<CardStrength>>,
    /// The height of the indicator row at the top of each box that has
    /// one; its image and text sit below it.
    pub indicator_rows: IndexMap<NodeId, f32>,
    pub connectors: Vec<Connector>,
    pub main_bounds: Rect,
    pub parking_bounds: Option<Rect>,
    pub bounds: Rect,
}

/// Lay out `doc` in its layout direction, packed compactly. Iterative throughout, so very
/// deep maps are safe.
pub fn layout(doc: &Document, m: &dyn Measure, p: &LayoutParams) -> Layout {
    let mut out = Layout {
        dir: doc.layout_direction,
        boxes: IndexMap::with_capacity(doc.boxes.len()),
        cards: IndexMap::with_capacity(doc.cards.len()),
        card_labels: IndexMap::with_capacity(doc.cards.len()),
        card_roles: IndexMap::with_capacity(doc.cards.len()),
        card_strength: IndexMap::with_capacity(doc.cards.len()),
        indicator_rows: IndexMap::new(),
        connectors: Vec::new(),
        main_bounds: Rect::NOTHING,
        parking_bounds: None,
        bounds: Rect::NOTHING,
    };
    let mut pending = Vec::new();
    tree::card_roles(doc, &mut out);

    let frame = axis::Frame::new(doc.layout_direction);
    let sizer = size::Sizer::new(doc, m, p);
    let main = tree::Tree::build(doc, Item::Box(doc.root), &sizer, p, frame);
    out.main_bounds = main.place(doc, (0.0, 0.0), p, &mut out, &mut pending);
    if out.main_bounds == Rect::NOTHING {
        // Only possible for a document whose root box is missing.
        out.main_bounds = Rect::from_min_size(Pos2::ZERO, Vec2::ZERO);
    }

    let cross = frame.cross_range(out.main_bounds).1 + p.parking_gap;
    let mut main_at = frame.main_range(out.main_bounds).0;
    for &item in &doc.parking {
        let t = tree::Tree::build(doc, item, &sizer, p, frame);
        let r = t.place(doc, (cross, main_at), p, &mut out, &mut pending);
        if r != Rect::NOTHING {
            main_at = frame.main_range(r).1 + p.v_gap;
            out.parking_bounds = Some(out.parking_bounds.map_or(r, |b| b.union(r)));
        }
    }

    out.connectors = ports::assign(doc, pending, &out, p, frame);
    out.indicator_rows = out
        .boxes
        .keys()
        .filter_map(|id| Some((*id, *sizer.rows.get(id)?)))
        .collect();
    out.bounds = out
        .parking_bounds
        .map_or(out.main_bounds, |b| b.union(out.main_bounds));
    // Bottom-up grows toward negative y.
    out.translate(Pos2::ZERO - out.main_bounds.min);
    out
}

impl Layout {
    /// The height of box `id`'s indicator row (0 without one).
    pub fn indicator_row(&self, id: NodeId) -> f32 {
        self.indicator_rows.get(&id).copied().unwrap_or(0.0)
    }

    /// Box `id`'s rect below its indicator row: where its image and text go.
    pub fn body(&self, id: NodeId) -> Option<Rect> {
        let mut r = *self.boxes.get(&id)?;
        r.min.y += self.indicator_row(id);
        Some(r)
    }

    /// Move everything by `d`.
    fn translate(&mut self, d: Vec2) {
        if d == Vec2::ZERO {
            return;
        }
        let rects = self.boxes.values_mut().chain(self.cards.values_mut());
        for r in rects.chain([&mut self.main_bounds, &mut self.bounds]) {
            *r = r.translate(d);
        }
        if let Some(r) = &mut self.parking_bounds {
            *r = r.translate(d);
        }
        for a in self.card_labels.values_mut() {
            *a += d;
        }
        for c in &mut self.connectors {
            c.from += d;
            c.to += d;
        }
    }
}
