//! A pen that draws glyphs as scene shapes: unit coordinates on a square.

use std::cell::RefCell;

use eframe::egui::{Color32, Pos2, Rect, Vec2};

use super::{PathEl, Shape};

/// Maps unit coordinates (0..1) onto a glyph's square and records what it
/// draws. Widths and radii the screen keeps in px are multiplied by `px`
/// (world units per screen px; 1.0 on screen and in exports at 100%).
pub(crate) struct ShapePen {
    rect: Rect,
    colour: Color32,
    width: f32,
    px: f32,
    shapes: RefCell<Vec<Shape>>,
}

impl ShapePen {
    /// A pen for `rect` (a square) with the usual icon stroke in `colour`:
    /// a 14th of the size, at least 1.25 px.
    pub(crate) fn new(rect: Rect, colour: Color32, px: f32) -> Self {
        Self {
            rect,
            colour,
            width: (rect.width() / 14.0).max(1.25 * px),
            px,
            shapes: RefCell::new(Vec::new()),
        }
    }

    /// What was drawn, in order.
    pub(crate) fn finish(self) -> Vec<Shape> {
        self.shapes.into_inner()
    }

    fn add(&self, shape: Shape) {
        self.shapes.borrow_mut().push(shape);
    }

    fn p(&self, x: f32, y: f32) -> Pos2 {
        self.rect.min + Vec2::new(x, y) * self.rect.width()
    }

    fn r(&self, x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
        Rect::from_min_max(self.p(x0, y0), self.p(x1, y1))
    }

    fn path(&self, pts: &[(f32, f32)], close: bool) -> Vec<PathEl> {
        let mut els: Vec<PathEl> = pts
            .iter()
            .enumerate()
            .map(|(i, &(x, y))| {
                let p = self.p(x, y);
                if i == 0 {
                    PathEl::MoveTo(p)
                } else {
                    PathEl::LineTo(p)
                }
            })
            .collect();
        if close {
            els.push(PathEl::Close);
        }
        els
    }

    /// An open line through `pts`.
    pub(crate) fn line(&self, pts: &[(f32, f32)]) {
        self.add(Shape::Path {
            points: self.path(pts, false),
            stroke: Some((self.width, self.colour)),
            fill: None,
            dashed: false,
        });
    }

    /// A closed outline through `pts`.
    pub(crate) fn closed(&self, pts: &[(f32, f32)]) {
        self.add(Shape::Path {
            points: self.path(pts, true),
            stroke: Some((self.width, self.colour)),
            fill: None,
            dashed: false,
        });
    }

    /// A filled convex polygon through `pts`.
    pub(crate) fn polygon(&self, pts: &[(f32, f32)]) {
        self.add(Shape::Path {
            points: self.path(pts, true),
            stroke: None,
            fill: Some(self.colour),
            dashed: false,
        });
    }

    /// A rectangle outline (rounded by 2 px), the stroke centred on its edge.
    pub(crate) fn rect(&self, x0: f32, y0: f32, x1: f32, y1: f32, colour: Color32) {
        let half = self.width / 2.0;
        self.add(Shape::Rect {
            rect: self.r(x0, y0, x1, y1).expand(half),
            radius: 2.0 * self.px + half,
            fill: Color32::TRANSPARENT,
            stroke: Some((self.width, colour)),
        });
    }

    /// A filled rectangle (rounded by 1 px).
    pub(crate) fn fill(&self, x0: f32, y0: f32, x1: f32, y1: f32, colour: Color32) {
        self.add(Shape::Rect {
            rect: self.r(x0, y0, x1, y1),
            radius: self.px,
            fill: colour,
            stroke: None,
        });
    }

    /// A circle outline around (x, y).
    pub(crate) fn circle(&self, x: f32, y: f32, r: f32) {
        self.add(Shape::Circle {
            center: self.p(x, y),
            radius: r * self.rect.width(),
            fill: None,
            stroke: Some((self.width, self.colour)),
        });
    }

    /// A filled circle around (x, y).
    pub(crate) fn disc(&self, x: f32, y: f32, r: f32, colour: Color32) {
        self.add(Shape::Circle {
            center: self.p(x, y),
            radius: r * self.rect.width(),
            fill: Some(colour),
            stroke: None,
        });
    }

    /// An arc around (x, y) from angle `a0` to `a1` (radians, y down).
    pub(crate) fn arc(&self, x: f32, y: f32, r: f32, a0: f32, a1: f32) {
        let pts: Vec<(f32, f32)> = (0..=16)
            .map(|i| {
                let a = a0 + (a1 - a0) * i as f32 / 16.0;
                (x + r * a.cos(), y + r * a.sin())
            })
            .collect();
        self.line(&pts);
    }
}
