//! A backend-neutral picture of a map: the shapes the canvas paints, in
//! world units, so exporters (SVG, PDF, PNG) draw exactly what the screen
//! shows. [`build_scene`] turns a laid-out document into a [`Scene`];
//! [`paint_scene`] draws it with egui.
//!
//! Screen-only overlays (selection ring, "+" button, comment badges, link
//! glyphs, dot grid) are never part of a scene: the canvas paints them on
//! top. A scene for the screen differs from one for export only in what is
//! culled, the zoom it is built for (sizes given in screen px), the hint in
//! an empty claim and the text of the box being edited.

mod boxes;
mod build;
mod egui_paint;
mod pen;

use eframe::egui::{Align, Color32, Pos2, Rect};
use reasoncraft_core::{BoxKind, Item, Measure, NodeId};

pub use boxes::{BOX_RADIUS, SOURCE_HEADER, box_radius};
pub use build::build_scene;
pub use egui_paint::{paint_scene, paint_shapes};
pub(crate) use pen::ShapePen;

use crate::theme::Tokens;

/// A stroke: width (world units) and colour.
pub type StrokeSpec = (f32, Color32);

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
}

impl SceneOptions {
    /// Options for exporting the whole map, parking area excluded.
    pub fn export(tokens: Tokens) -> Self {
        Self {
            tokens,
            show_markers: true,
            eval_shading: true,
            for_export: true,
            include_parking: false,
            only: None,
            view: None,
            zoom: 1.0,
            editing: None,
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
mod tests;
