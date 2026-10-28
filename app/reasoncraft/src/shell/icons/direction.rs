//! Layout direction glyphs: a parent box and two children, the children
//! below (top-down), above (bottom-up) or to the right (left-to-right).

use std::sync::Arc;

use eframe::egui::{
    Atom, AtomKind, AtomPaintArgs, Color32, IntoSizedResult, Painter, Rect, SizedAtomKind, Ui, Vec2,
};
use reasoncraft_core::LayoutDirection;

use crate::canvas::scene_paint::paint_shapes;
use crate::scene::{Shape, ShapePen};

/// The glyph for `dir` in `rect` (a square) in `colour`, as shapes.
pub fn direction_shapes(rect: Rect, dir: LayoutDirection, colour: Color32) -> Vec<Shape> {
    let pen = ShapePen::new(rect, colour, 1.0);
    // Drawn top-down, then mirrored or turned for the direction.
    let at = |x: f32, y: f32| match dir {
        LayoutDirection::TopDown => (x, y),
        LayoutDirection::BottomUp => (x, 1.0 - y),
        LayoutDirection::LeftRight => (y, x),
    };
    let rect = |x0: f32, y0: f32, x1: f32, y1: f32, filled: bool| {
        let ((ax, ay), (bx, by)) = (at(x0, y0), at(x1, y1));
        let (x0, x1, y0, y1) = (ax.min(bx), ax.max(bx), ay.min(by), ay.max(by));
        if filled {
            pen.fill(x0, y0, x1, y1, colour);
        } else {
            pen.rect(x0, y0, x1, y1, colour);
        }
    };
    let line = |pts: &[(f32, f32)]| {
        let pts: Vec<(f32, f32)> = pts.iter().map(|&(x, y)| at(x, y)).collect();
        pen.line(&pts);
    };
    rect(0.3, 0.06, 0.7, 0.3, true);
    rect(0.06, 0.7, 0.42, 0.92, false);
    rect(0.58, 0.7, 0.94, 0.92, false);
    line(&[(0.5, 0.3), (0.5, 0.5)]);
    line(&[(0.24, 0.7), (0.24, 0.5), (0.76, 0.5), (0.76, 0.7)]);
    pen.finish()
}

/// Draw the glyph for `dir` in `rect` (a square, screen px) in `colour`.
pub fn paint_direction(painter: &Painter, rect: Rect, dir: LayoutDirection, colour: Color32) {
    paint_shapes(painter, &direction_shapes(rect, dir, colour));
}

/// The glyph for `dir` as a button atom of `size` px, painted in the
/// button's text colour.
pub fn direction_atom(dir: LayoutDirection, size: f32) -> Atom<'static> {
    AtomKind::closure(move |_ui: &Ui, _args| {
        let size = Vec2::splat(size);
        IntoSizedResult {
            intrinsic_size: size,
            sized: SizedAtomKind::Paint {
                paint: Arc::new(move |ui: &Ui, args: AtomPaintArgs| {
                    let rect = Rect::from_center_size(args.rect.center(), size);
                    paint_direction(ui.painter(), rect, dir, args.fallback_text_color);
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
    use eframe::egui::{Pos2, pos2};

    /// The filled parent box sits at the edge the children point away from.
    #[test]
    fn parent_box_follows_direction() {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::splat(100.0));
        for dir in LayoutDirection::ALL {
            let shapes = direction_shapes(rect, dir, Color32::WHITE);
            let Some(Shape::Rect { rect: parent, .. }) = shapes.first() else {
                panic!("{dir:?}: the parent box comes first");
            };
            let toward = dir.toward_parent();
            let offset = parent.center() - pos2(50.0, 50.0);
            assert!(offset.dot(toward) > 20.0, "{dir:?}: {offset:?}");
        }
    }
}
