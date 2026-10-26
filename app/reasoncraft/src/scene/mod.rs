//! A backend-neutral picture of a map: the shapes the canvas paints, in
//! world units, so exporters (SVG, PDF, PNG) draw exactly what the screen
//! shows. [`build_scene`] turns a laid-out document into a [`Scene`];
//! [`paint_scene`] draws it with egui.
//!
//! Screen-only overlays (selection ring, "+" button, comment badges, link
//! glyphs, dot grid, minimap) are never part of a scene: the canvas paints them on
//! top. A scene for the screen differs from one for export only in what is
//! culled, the zoom it is built for (sizes given in screen px), the hint in
//! an empty claim and the text of the box being edited.

mod boxes;
mod build;
mod connectors;
mod egui_paint;
mod numbers;
mod pen;

use eframe::egui::{Align, Color32, Pos2, Rect};
use reasoncraft_core::{BoxKind, ConnectorStyle, Item, Measure, NodeId};

pub use boxes::{BOX_RADIUS, SOURCE_HEADER, box_radius};
pub use build::build_scene;
pub use egui_paint::{paint_scene, paint_shapes};
pub use numbers::{NUMBER_SIZE, marks_leads, number_rect};
pub(crate) use pen::ShapePen;

use crate::theme::Tokens;

/// A stroke: width (world units) and colour.
pub type StrokeSpec = (f32, Color32);

/// Dash length of dashed paths, in screen px (world units in an export,
/// which is drawn at zoom 1).
pub const DASH_LEN: f32 = 6.0;
/// Gap between the dashes of dashed paths, like [`DASH_LEN`].
pub const GAP_LEN: f32 = 4.0;

/// How far bold text is smeared to the right, in screen px, at `zoom`:
/// the default fonts have no bold face, so bold is faked by drawing the
/// text twice this far apart.
pub fn bold_offset(zoom: f32) -> f32 {
    0.45 * zoom.min(1.5)
}

/// The placeholder drawn for an image whose asset is missing or broken: a
/// dashed outline with a centred label, in the muted text colour. Sizes in
/// screen px (world units in an export).
pub mod missing_image {
    /// Outline width.
    pub const STROKE: f32 = 1.0;
    /// Outline dash and gap.
    pub const DASH: f32 = 5.0;
    pub const GAP: f32 = 4.0;
    /// Size of the label's text (in world units, like box text).
    pub const TEXT_SIZE: f32 = 11.0;

    /// The label in the middle of the outline.
    pub fn label() -> String {
        crate::i18n::t("image.missing")
    }
}

/// One step of a [`Shape::Path`], in world units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathEl {
    MoveTo(Pos2),
    LineTo(Pos2),
    /// A cubic Bézier: two control points, then the end point.
    CubicTo(Pos2, Pos2, Pos2),
    /// Back to the last `MoveTo`.
    Close,
}

/// One drawing primitive, in world units.
#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    /// A rounded rectangle. Its stroke lies inside `rect`.
    Rect {
        rect: Rect,
        radius: f32,
        fill: Color32,
        stroke: Option<StrokeSpec>,
    },
    /// Lines and curves. A filled path is convex. The stroke is centred on
    /// the path; `dashed` paths dash it.
    Path {
        points: Vec<PathEl>,
        stroke: Option<StrokeSpec>,
        fill: Option<Color32>,
        dashed: bool,
    },
    /// A circle; its stroke is centred on the radius.
    Circle {
        center: Pos2,
        radius: f32,
        fill: Option<Color32>,
        stroke: Option<StrokeSpec>,
    },
    /// Rows of text, one under the other, `line_height` apart. `origin` is
    /// the top of the first row, at its left (`Align::Min`), centre
    /// (`Align::Center`) or right (`Align::Max`) end. The rows are already
    /// wrapped and are drawn as they are. Empty rows only take up space.
    Text {
        origin: Pos2,
        rows: Vec<String>,
        size: f32,
        bold: bool,
        color: Color32,
        align: Align,
        line_height: f32,
    },
    /// A box image, from the document's asset `asset`.
    Image { rect: Rect, asset: String },
    /// A filled band between two edges with the same number of points:
    /// `left[i]` and `right[i]` lie across the band from each other. It
    /// may bend any way (a tapered connector), unlike a filled path.
    Strip {
        left: Vec<Pos2>,
        right: Vec<Pos2>,
        fill: Color32,
    },
}

/// Everything drawn, in paint order, and the world rect it covers.
#[derive(Clone, Debug, PartialEq)]
pub struct Scene {
    pub shapes: Vec<Shape>,
    pub bounds: Rect,
}

/// What a scene shows.
#[derive(Clone, Copy, Debug)]
pub struct SceneOptions {
    pub tokens: Tokens,
    /// Draw ✓ / ? / × markers on rated boxes.
    pub show_markers: bool,
    /// Draw each numbered box's outline number ([`reasoncraft_core::Layout::numbers`], in
    /// the indicator row the layout gave it).
    pub numbers: bool,
    /// Tint and weight cards and connectors by their rating.
    pub eval_shading: bool,
    /// For a file or the clipboard: no screen hints.
    pub for_export: bool,
    /// Show the parking area: its loose items and its frame.
    pub include_parking: bool,
    /// Show only this item and everything under it (a selection export).
    pub only: Option<Item>,
    /// World rect to cull to (the visible part of the canvas); `None`
    /// keeps everything.
    pub view: Option<Rect>,
    /// The zoom the scene is drawn at. Sizes the screen keeps constant in
    /// px (strokes, arrowheads, markers' minimums) are divided by it, and
    /// details too small to see at it are left out. 1.0 for exports.
    pub zoom: f32,
    /// The box whose text the inline editor shows instead (screen only).
    pub editing: Option<NodeId>,
    /// How connectors are drawn.
    pub connector_style: ConnectorStyle,
}

impl SceneOptions {
    /// Options for exporting the whole map, parking area excluded.
    pub fn export(tokens: Tokens) -> Self {
        Self {
            tokens,
            show_markers: true,
            numbers: false,
            eval_shading: true,
            for_export: true,
            include_parking: false,
            only: None,
            view: None,
            zoom: 1.0,
            editing: None,
            connector_style: ConnectorStyle::Curved,
        }
    }
}

/// Text measurement for scenes: the measure the layout used (so text is
/// centred in the height the layout reserved), and its wrapping.
pub trait TextRows: Measure {
    /// The rows `text` wraps into at `width` in the box font for `kind`.
    fn rows(&self, text: &str, width: f32, kind: &BoxKind) -> Vec<String>;
    /// The distance between rows of text of `size`.
    fn line_height(&self, size: f32) -> f32;
    /// The width of `text` on one row at `size`.
    fn text_width(&self, text: &str, size: f32) -> f32;
}

impl Shape {
    /// A rough world bounding rect, strokes included. Text is taken as its
    /// origin column, as tall as its rows (it never sticks out of what it
    /// labels).
    pub fn bounds(&self) -> Rect {
        let half = |s: &Option<StrokeSpec>| s.map_or(0.0, |(w, _)| w / 2.0);
        match self {
            Shape::Rect { rect, .. } | Shape::Image { rect, .. } => *rect,
            Shape::Circle {
                center,
                radius,
                stroke,
                ..
            } => Rect::from_center_size(*center, eframe::egui::Vec2::splat(2.0 * radius))
                .expand(half(stroke)),
            Shape::Path { points, stroke, .. } => {
                let mut r = Rect::NOTHING;
                for el in points {
                    match *el {
                        PathEl::MoveTo(p) | PathEl::LineTo(p) => r.extend_with(p),
                        PathEl::CubicTo(a, b, c) => {
                            r.extend_with(a);
                            r.extend_with(b);
                            r.extend_with(c);
                        }
                        PathEl::Close => {}
                    }
                }
                r.expand(half(stroke))
            }
            Shape::Strip { left, right, .. } => left
                .iter()
                .chain(right)
                .fold(Rect::NOTHING, |r, p| r.union(Rect::from_pos(*p))),
            Shape::Text {
                origin,
                rows,
                line_height,
                ..
            } => Rect::from_min_size(
                *origin,
                eframe::egui::vec2(0.0, rows.len() as f32 * line_height),
            ),
        }
    }
}

#[cfg(test)]
mod connector_tests;
#[cfg(test)]
mod tests;
