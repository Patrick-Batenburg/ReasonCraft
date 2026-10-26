//! Connector style glyphs: a parent box and two children joined by
//! connectors drawn with the style's own geometry.

use std::sync::Arc;

use eframe::egui::{
    Atom, AtomKind, AtomPaintArgs, Color32, IntoSizedResult, Painter, Rect, SizedAtomKind, Ui,
    Vec2, pos2,
};
use reasoncraft_core::connector::{Segment, curved_path, elbow_path, tapered_band};
use reasoncraft_core::{ConnectorStyle, LayoutDirection};

use crate::scene::{PathEl, Shape, ShapePen, paint_shapes};

/// The glyph for `style` in `rect` (a square) in `colour`, as shapes.
pub fn connector_shapes(rect: Rect, style: ConnectorStyle, colour: Color32) -> Vec<Shape> {
    let pen = ShapePen::new(rect, colour, 1.0);
    pen.fill(0.3, 0.06, 0.7, 0.3, colour);
    pen.rect(0.06, 0.7, 0.42, 0.92, colour);
    pen.rect(0.58, 0.7, 0.94, 0.92, colour);
    let mut shapes = pen.finish();
    let size = rect.width();
    let at = |x: f32, y: f32| rect.min + Vec2::new(x, y) * size;
    let width = (size / 14.0).max(1.25);
    let stub = 0.08 * size;
    let dir = LayoutDirection::TopDown;
    for (child, port) in [(0.24, 0.42), (0.76, 0.58)] {
        let (from, to) = (at(child, 0.7), at(port, 0.3));
        let path = match style {
            ConnectorStyle::Curved => curved_path(from, to, stub, dir),
            ConnectorStyle::Elbow => elbow_path(from, to, stub, 0.1 * size, dir),
            ConnectorStyle::Tapered => {
                let (left, right) = tapered_band(from, to, stub, 0.8 * width, 2.5 * width, dir);
                shapes.push(Shape::Strip {
                    left,
                    right,
                    fill: colour,
                });
                vec![Segment::Line([pos2(to.x, to.y + stub), to])]
            }
        };
        let mut points = Vec::with_capacity(path.len() + 1);
        for (i, seg) in path.into_iter().enumerate() {
            let (start, el) = match seg {
                Segment::Cubic([a, b, c, d]) => (a, PathEl::CubicTo(b, c, d)),
                Segment::Line([a, b]) => (a, PathEl::LineTo(b)),
            };
            if i == 0 {
                points.push(PathEl::MoveTo(start));
            }
            points.push(el);
        }
        let w = if style == ConnectorStyle::Tapered {
            0.8 * width
        } else {
            width
        };
        shapes.push(Shape::Path {
            points,
            stroke: Some((w, colour)),
            fill: None,
            dashed: false,
        });
    }
    shapes
}

/// Draw the glyph for `style` in `rect` (a square, screen px) in `colour`.
pub fn paint_connector(painter: &Painter, rect: Rect, style: ConnectorStyle, colour: Color32) {
    paint_shapes(painter, &connector_shapes(rect, style, colour));
}

/// The glyph for `style` as a button atom of `size` px, painted in the
/// button's text colour.
pub fn connector_atom(style: ConnectorStyle, size: f32) -> Atom<'static> {
    AtomKind::closure(move |_ui: &Ui, _args| {
        let size = Vec2::splat(size);
        IntoSizedResult {
            intrinsic_size: size,
            sized: SizedAtomKind::Paint {
                paint: Arc::new(move |ui: &Ui, args: AtomPaintArgs| {
                    let rect = Rect::from_center_size(args.rect.center(), size);
                    paint_connector(ui.painter(), rect, style, args.fallback_text_color);
                }),
                size,
            },
        }
    })
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::Pos2;

    /// Each style's glyph is drawn its own way: curves, corners or bands.
    #[test]
    fn glyphs_differ_by_style() {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::splat(32.0));
        let count = |style, f: fn(&Shape) -> bool| {
            connector_shapes(rect, style, Color32::WHITE)
                .iter()
                .filter(|s| f(s))
                .count()
        };
        let cubics = |s: &Shape| match s {
            Shape::Path { points, .. } => points.iter().any(|p| matches!(p, PathEl::CubicTo(..))),
            _ => false,
        };
        let strip = |s: &Shape| matches!(s, Shape::Strip { .. });
        assert_eq!(count(ConnectorStyle::Curved, cubics), 2);
        assert_eq!(count(ConnectorStyle::Elbow, cubics), 2);
        assert_eq!(count(ConnectorStyle::Tapered, strip), 2);
        assert_eq!(count(ConnectorStyle::Tapered, cubics), 0);
    }
}
