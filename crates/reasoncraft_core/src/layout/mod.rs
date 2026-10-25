//! Automatic top-down tidy-tree layout of a [`Document`].
//!
//! [`layout`] places the main tree (from the root claim) with its top-left
//! slot at the origin, then each parking item as its own mini-tree, stacked
//! to the right of the main tree. Coordinates grow downward like the screen.

mod ports;
mod tree;

pub(crate) use tree::display_children;

use emath::{Pos2, Rect, Vec2};
use indexmap::IndexMap;

use crate::connector::Strength;
use crate::ids::{CardId, Item, NodeId};
use crate::model::{BoxKind, CardStrength, Document, ImageRef, Role};

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
    /// The child's top centre (a card's frame top centre).
    pub from: Pos2,
    /// The parent port.
    pub to: Pos2,
    pub kind: ConnectorKind,
    pub target: TargetKind,
    /// Evaluated strength of a card's connector; `Normal` for other items.
    pub strength: Strength,
    /// The card is rated nil (drawn weak, flagged for a distinct look).
    pub nil: bool,
}

/// The result of [`layout`].
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
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
    pub connectors: Vec<Connector>,
    pub main_bounds: Rect,
    pub parking_bounds: Option<Rect>,
    pub bounds: Rect,
}

/// Lay out `doc` top-down. Iterative throughout, so very deep maps are safe.
pub fn layout(doc: &Document, m: &dyn Measure, p: &LayoutParams) -> Layout {
    let mut out = Layout {
        boxes: IndexMap::with_capacity(doc.boxes.len()),
        cards: IndexMap::with_capacity(doc.cards.len()),
        card_labels: IndexMap::with_capacity(doc.cards.len()),
        card_roles: IndexMap::with_capacity(doc.cards.len()),
        card_strength: IndexMap::with_capacity(doc.cards.len()),
        connectors: Vec::new(),
        main_bounds: Rect::NOTHING,
        parking_bounds: None,
        bounds: Rect::NOTHING,
    };
    let mut pending = Vec::new();
    tree::card_roles(doc, &mut out);

    let main = tree::Tree::build(doc, Item::Box(doc.root), m, p);
    out.main_bounds = main.place(doc, Pos2::ZERO, p, &mut out, &mut pending);
    if out.main_bounds == Rect::NOTHING {
        // Only possible for a document whose root box is missing.
        out.main_bounds = Rect::from_min_size(Pos2::ZERO, Vec2::ZERO);
    }

    let left = out.main_bounds.right() + p.parking_gap;
    let mut top = out.main_bounds.top();
    for &item in &doc.parking {
        let t = tree::Tree::build(doc, item, m, p);
        let r = t.place(doc, Pos2::new(left, top), p, &mut out, &mut pending);
        if r != Rect::NOTHING {
            top = r.bottom() + p.v_gap;
            out.parking_bounds = Some(out.parking_bounds.map_or(r, |b| b.union(r)));
        }
    }

    out.connectors = ports::assign(doc, pending, &out, p);
    out.bounds = out
        .parking_bounds
        .map_or(out.main_bounds, |b| b.union(out.main_bounds));
    out
}
