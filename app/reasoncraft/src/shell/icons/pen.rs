//! The pen icons are drawn with: unit coordinates on a square.

use eframe::egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, Vec2};

/// Maps unit coordinates (0..1) onto the icon's square.
pub(in crate::shell) struct Pen<'a> {
    pub(in crate::shell) painter: &'a Painter,
    pub(in crate::shell) rect: Rect,
    pub(in crate::shell) stroke: Stroke,
}

impl<'a> Pen<'a> {
    /// A pen for `rect` (a square) with the usual icon stroke in `colour`.
    pub(in crate::shell) fn new(painter: &'a Painter, rect: Rect, colour: Color32) -> Self {
        let width = (rect.width() / 14.0).max(1.25);
        Pen {
            painter,
            rect,
            stroke: Stroke::new(width, colour),
        }
    }
    /// A closed outline through `pts`.
    pub(in crate::shell) fn closed(&self, pts: &[(f32, f32)]) {
        let pts: Vec<Pos2> = pts.iter().map(|&(x, y)| self.p(x, y)).collect();
        self.painter.add(Shape::closed_line(pts, self.stroke));
    }
    /// A filled convex polygon through `pts`.
    pub(in crate::shell) fn polygon(&self, pts: &[(f32, f32)]) {
        let pts: Vec<Pos2> = pts.iter().map(|&(x, y)| self.p(x, y)).collect();
        self.painter
            .add(Shape::convex_polygon(pts, self.stroke.color, Stroke::NONE));
    }
    pub(in crate::shell) fn p(&self, x: f32, y: f32) -> Pos2 {
        self.rect.min + Vec2::new(x, y) * self.rect.width()
    }
    pub(in crate::shell) fn r(&self, x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
        Rect::from_min_max(self.p(x0, y0), self.p(x1, y1))
    }
    pub(in crate::shell) fn line(&self, pts: &[(f32, f32)]) {
        let pts: Vec<Pos2> = pts.iter().map(|&(x, y)| self.p(x, y)).collect();
        self.painter.line(pts, self.stroke);
    }
    pub(in crate::shell) fn line_c(&self, pts: &[(f32, f32)], color: Color32) {
        let pts: Vec<Pos2> = pts.iter().map(|&(x, y)| self.p(x, y)).collect();
        self.painter
            .line(pts, Stroke::new(self.stroke.width, color));
    }
    pub(in crate::shell) fn rect(&self, x0: f32, y0: f32, x1: f32, y1: f32, color: Color32) {
        let r = self.r(x0, y0, x1, y1);
        let s = Stroke::new(self.stroke.width, color);
        self.painter.rect_stroke(r, 2.0, s, StrokeKind::Middle);
    }
    pub(in crate::shell) fn fill(&self, x0: f32, y0: f32, x1: f32, y1: f32, color: Color32) {
        self.painter.rect_filled(self.r(x0, y0, x1, y1), 1.0, color);
    }
    pub(in crate::shell) fn circle(&self, x: f32, y: f32, r: f32) {
        let r = r * self.rect.width();
        self.painter.circle_stroke(self.p(x, y), r, self.stroke);
    }
    pub(in crate::shell) fn disc(&self, x: f32, y: f32, r: f32, color: Color32) {
        self.painter
            .circle_filled(self.p(x, y), r * self.rect.width(), color);
    }
    /// An arc around (x, y) from angle `a0` to `a1` (radians, y down).
    pub(in crate::shell) fn arc(&self, x: f32, y: f32, r: f32, a0: f32, a1: f32) {
        let pts: Vec<(f32, f32)> = (0..=16)
            .map(|i| {
                let a = a0 + (a1 - a0) * i as f32 / 16.0;
                (x + r * a.cos(), y + r * a.sin())
            })
            .collect();
        self.line(&pts);
    }
    pub(in crate::shell) fn arrow_up(&self, x: f32, y: f32, color: Color32) {
        self.line_c(&[(x - 0.12, y + 0.12), (x, y), (x + 0.12, y + 0.12)], color);
    }
}
