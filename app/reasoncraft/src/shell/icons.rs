//! Original command icons, drawn with simple vector shapes.

use std::f32::consts::{PI, TAU};

use eframe::egui::{Color32, Painter, Pos2, Rect, Stroke, StrokeKind, Vec2};

use crate::theme::Tokens;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    New,
    Open,
    Save,
    SaveAs,
    Undo,
    Redo,
    Paste,
    Cut,
    Copy,
    Reason,
    Objection,
    CoPremise,
    Category,
    Note,
    Delete,
    DeleteBranch,
    ZoomFit,
    Zoom100,
    ZoomIn,
    ZoomOut,
    ThemeSystem,
    ThemeLight,
    ThemeDark,
    PanelLeft,
    PanelRight,
    Plus,
    Close,
    Minimize,
    Maximize,
    Restore,
    Back,
    Accept,
    Unsure,
    Reject,
    Strong,
    Weak,
    Nil,
    Clear,
    Markers,
    Shading,
}

/// Maps unit coordinates (0..1) onto the icon's square.
struct Pen<'a> {
    painter: &'a Painter,
    rect: Rect,
    stroke: Stroke,
}

impl Pen<'_> {
    fn p(&self, x: f32, y: f32) -> Pos2 {
        self.rect.min + Vec2::new(x, y) * self.rect.width()
    }
    fn r(&self, x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
        Rect::from_min_max(self.p(x0, y0), self.p(x1, y1))
    }
    fn line(&self, pts: &[(f32, f32)]) {
        let pts: Vec<Pos2> = pts.iter().map(|&(x, y)| self.p(x, y)).collect();
        self.painter.line(pts, self.stroke);
    }
    fn line_c(&self, pts: &[(f32, f32)], color: Color32) {
        let pts: Vec<Pos2> = pts.iter().map(|&(x, y)| self.p(x, y)).collect();
        self.painter
            .line(pts, Stroke::new(self.stroke.width, color));
    }
    fn rect(&self, x0: f32, y0: f32, x1: f32, y1: f32, color: Color32) {
        let r = self.r(x0, y0, x1, y1);
        let s = Stroke::new(self.stroke.width, color);
        self.painter.rect_stroke(r, 2.0, s, StrokeKind::Middle);
    }
    fn fill(&self, x0: f32, y0: f32, x1: f32, y1: f32, color: Color32) {
        self.painter.rect_filled(self.r(x0, y0, x1, y1), 1.0, color);
    }
    fn circle(&self, x: f32, y: f32, r: f32) {
        let r = r * self.rect.width();
        self.painter.circle_stroke(self.p(x, y), r, self.stroke);
    }
    fn disc(&self, x: f32, y: f32, r: f32, color: Color32) {
        self.painter
            .circle_filled(self.p(x, y), r * self.rect.width(), color);
    }
    /// An arc around (x, y) from angle `a0` to `a1` (radians, y down).
    fn arc(&self, x: f32, y: f32, r: f32, a0: f32, a1: f32) {
        let pts: Vec<(f32, f32)> = (0..=16)
            .map(|i| {
                let a = a0 + (a1 - a0) * i as f32 / 16.0;
                (x + r * a.cos(), y + r * a.sin())
            })
            .collect();
        self.line(&pts);
    }
    fn arrow_up(&self, x: f32, y: f32, color: Color32) {
        self.line_c(&[(x - 0.12, y + 0.12), (x, y), (x + 0.12, y + 0.12)], color);
    }
}

/// Draw `icon` in `rect` (a square). `fg` is the main colour, `bg` the
/// surface it sits on (used to cut shapes such as the moon).
pub fn paint(painter: &Painter, rect: Rect, icon: Icon, fg: Color32, bg: Color32, t: &Tokens) {
    let width = (rect.width() / 14.0).max(1.25);
    let pen = Pen {
        painter,
        rect,
        stroke: Stroke::new(width, fg),
    };
    let muted = |c: Color32| if fg == t.text_muted { fg } else { c };
    match icon {
        Icon::New => {
            pen.line(&[
                (0.62, 0.1),
                (0.2, 0.1),
                (0.2, 0.9),
                (0.8, 0.9),
                (0.8, 0.28),
                (0.62, 0.1),
                (0.62, 0.28),
                (0.8, 0.28),
            ]);
            pen.line_c(&[(0.5, 0.45), (0.5, 0.75)], muted(t.accent));
            pen.line_c(&[(0.35, 0.6), (0.65, 0.6)], muted(t.accent));
        }
        Icon::Open => {
            pen.line(&[
                (0.08, 0.82),
                (0.08, 0.2),
                (0.38, 0.2),
                (0.46, 0.3),
                (0.82, 0.3),
                (0.82, 0.42),
            ]);
            pen.line(&[
                (0.08, 0.82),
                (0.22, 0.42),
                (0.95, 0.42),
                (0.8, 0.82),
                (0.08, 0.82),
            ]);
        }
        Icon::Save | Icon::SaveAs => {
            pen.line(&[
                (0.12, 0.12),
                (0.72, 0.12),
                (0.88, 0.28),
                (0.88, 0.88),
                (0.12, 0.88),
                (0.12, 0.12),
            ]);
            pen.rect(0.3, 0.12, 0.64, 0.36, fg);
            pen.rect(0.28, 0.58, 0.72, 0.88, fg);
            if icon == Icon::SaveAs {
                pen.line_c(&[(0.55, 0.98), (0.98, 0.55)], muted(t.accent));
            }
        }
        Icon::Undo | Icon::Redo => {
            let flip = |x: f32| if icon == Icon::Undo { x } else { 1.0 - x };
            let pts: Vec<(f32, f32)> = (0..=16)
                .map(|i| {
                    let a = -PI / 2.0 + PI * i as f32 / 16.0;
                    (flip(0.5 + 0.3 * a.cos()), 0.55 + 0.25 * a.sin())
                })
                .collect();
            pen.line(&pts);
            pen.line(&[(flip(0.5), 0.8), (flip(0.3), 0.8)]);
            pen.line(&[(flip(0.38), 0.16), (flip(0.24), 0.3), (flip(0.38), 0.44)]);
            pen.line(&[(flip(0.24), 0.3), (flip(0.5), 0.3)]);
        }
        Icon::Paste => {
            pen.rect(0.18, 0.18, 0.82, 0.92, fg);
            pen.fill(0.36, 0.1, 0.64, 0.26, fg);
            pen.line(&[(0.32, 0.5), (0.68, 0.5)]);
            pen.line(&[(0.32, 0.68), (0.6, 0.68)]);
        }
        Icon::Cut => {
            pen.circle(0.3, 0.75, 0.13);
            pen.circle(0.7, 0.75, 0.13);
            pen.line(&[(0.38, 0.64), (0.75, 0.1)]);
            pen.line(&[(0.62, 0.64), (0.25, 0.1)]);
        }
        Icon::Copy => {
            pen.rect(0.12, 0.12, 0.62, 0.68, fg);
            pen.rect(0.38, 0.34, 0.88, 0.9, fg);
        }
        Icon::Reason | Icon::Objection => {
            let role = muted(if icon == Icon::Reason {
                t.reason
            } else {
                t.objection
            });
            pen.rect(0.2, 0.06, 0.8, 0.3, fg);
            pen.rect(0.16, 0.66, 0.84, 0.94, role);
            pen.line_c(&[(0.5, 0.66), (0.5, 0.33)], role);
            if icon == Icon::Reason {
                pen.arrow_up(0.5, 0.33, role);
            } else {
                pen.line_c(&[(0.36, 0.36), (0.64, 0.36)], role);
            }
        }
        Icon::CoPremise => {
            let role = muted(t.reason);
            pen.rect(0.04, 0.24, 0.96, 0.76, role);
            pen.rect(0.12, 0.36, 0.46, 0.64, fg);
            pen.rect(0.54, 0.36, 0.88, 0.64, fg);
            pen.line_c(&[(0.71, 0.82), (0.71, 0.98)], muted(t.accent));
            pen.line_c(&[(0.63, 0.9), (0.79, 0.9)], muted(t.accent));
        }
        Icon::Category => {
            pen.rect(0.3, 0.06, 0.7, 0.28, fg);
            pen.line(&[(0.5, 0.28), (0.5, 0.48)]);
            pen.line(&[(0.25, 0.62), (0.25, 0.48), (0.75, 0.48), (0.75, 0.62)]);
            pen.rect(0.06, 0.64, 0.44, 0.9, fg);
            pen.rect(0.56, 0.64, 0.94, 0.9, fg);
        }
        Icon::Note => {
            pen.line(&[
                (0.15, 0.12),
                (0.85, 0.12),
                (0.85, 0.65),
                (0.6, 0.9),
                (0.15, 0.9),
                (0.15, 0.12),
            ]);
            pen.line(&[(0.6, 0.9), (0.6, 0.65), (0.85, 0.65)]);
            pen.line(&[(0.3, 0.32), (0.7, 0.32)]);
            pen.line(&[(0.3, 0.48), (0.62, 0.48)]);
        }
        Icon::Delete | Icon::Close => {
            let c = if icon == Icon::Delete {
                muted(t.objection)
            } else {
                fg
            };
            pen.line_c(&[(0.22, 0.22), (0.78, 0.78)], c);
            pen.line_c(&[(0.78, 0.22), (0.22, 0.78)], c);
        }
        Icon::DeleteBranch => {
            pen.rect(0.3, 0.04, 0.7, 0.24, fg);
            pen.line(&[(0.5, 0.24), (0.5, 0.4)]);
            pen.line(&[(0.25, 0.52), (0.25, 0.4), (0.75, 0.4), (0.75, 0.52)]);
            let c = muted(t.objection);
            pen.line_c(&[(0.1, 0.6), (0.4, 0.92)], c);
            pen.line_c(&[(0.4, 0.6), (0.1, 0.92)], c);
            pen.line_c(&[(0.6, 0.6), (0.9, 0.92)], c);
            pen.line_c(&[(0.9, 0.6), (0.6, 0.92)], c);
        }
        Icon::ZoomFit => {
            for (x, y, dx, dy) in [
                (0.1, 0.1, 1.0, 1.0),
                (0.9, 0.1, -1.0, 1.0),
                (0.1, 0.9, 1.0, -1.0),
                (0.9, 0.9, -1.0, -1.0),
            ] {
                pen.line(&[(x, y + 0.25 * dy), (x, y), (x + 0.25 * dx, y)]);
            }
            pen.rect(0.32, 0.36, 0.68, 0.64, fg);
        }
        Icon::Zoom100 => {
            pen.line(&[(0.18, 0.32), (0.28, 0.24), (0.28, 0.76)]);
            pen.disc(0.5, 0.4, 0.045, fg);
            pen.disc(0.5, 0.64, 0.045, fg);
            pen.line(&[(0.66, 0.32), (0.76, 0.24), (0.76, 0.76)]);
        }
        Icon::ZoomIn | Icon::ZoomOut => {
            pen.circle(0.42, 0.42, 0.28);
            pen.line(&[(0.62, 0.62), (0.9, 0.9)]);
            pen.line(&[(0.3, 0.42), (0.54, 0.42)]);
            if icon == Icon::ZoomIn {
                pen.line(&[(0.42, 0.3), (0.42, 0.54)]);
            }
        }
        Icon::ThemeSystem => {
            pen.circle(0.5, 0.5, 0.36);
            let pts: Vec<Pos2> = (0..=16)
                .map(|i| {
                    let a = -PI / 2.0 + PI * i as f32 / 16.0;
                    pen.p(0.5 + 0.36 * a.cos(), 0.5 + 0.36 * a.sin())
                })
                .collect();
            painter.add(eframe::egui::Shape::convex_polygon(pts, fg, Stroke::NONE));
        }
        Icon::ThemeLight => {
            pen.circle(0.5, 0.5, 0.2);
            for i in 0..8 {
                let a = TAU * i as f32 / 8.0;
                let (c, s) = (a.cos(), a.sin());
                pen.line(&[
                    (0.5 + 0.3 * c, 0.5 + 0.3 * s),
                    (0.5 + 0.42 * c, 0.5 + 0.42 * s),
                ]);
            }
        }
        Icon::ThemeDark => {
            pen.disc(0.48, 0.52, 0.34, fg);
            pen.disc(0.66, 0.38, 0.3, bg);
        }
        Icon::PanelLeft | Icon::PanelRight => {
            pen.rect(0.08, 0.16, 0.92, 0.84, fg);
            if icon == Icon::PanelLeft {
                pen.fill(0.08, 0.16, 0.36, 0.84, fg);
            } else {
                pen.fill(0.64, 0.16, 0.92, 0.84, fg);
            }
        }
        Icon::Plus => {
            pen.line(&[(0.5, 0.2), (0.5, 0.8)]);
            pen.line(&[(0.2, 0.5), (0.8, 0.5)]);
        }
        Icon::Minimize => pen.line(&[(0.25, 0.55), (0.75, 0.55)]),
        Icon::Maximize => pen.rect(0.25, 0.25, 0.75, 0.75, fg),
        Icon::Restore => {
            pen.rect(0.25, 0.35, 0.65, 0.75, fg);
            pen.line(&[
                (0.35, 0.35),
                (0.35, 0.25),
                (0.75, 0.25),
                (0.75, 0.65),
                (0.65, 0.65),
            ]);
        }
        Icon::Back => {
            pen.arc(0.5, 0.5, 0.4, 0.0, TAU);
            pen.line(&[(0.3, 0.5), (0.7, 0.5)]);
            pen.line(&[(0.46, 0.34), (0.3, 0.5), (0.46, 0.66)]);
        }
        Icon::Accept => {
            pen.circle(0.5, 0.5, 0.4);
            pen.line_c(&[(0.3, 0.52), (0.44, 0.66), (0.7, 0.36)], muted(t.reason));
        }
        Icon::Unsure => {
            pen.circle(0.5, 0.5, 0.4);
            pen.arc(0.5, 0.4, 0.12, PI, 2.4 * PI);
            pen.line(&[(0.5, 0.52), (0.5, 0.58)]);
            pen.disc(0.5, 0.7, 0.035, fg);
        }
        Icon::Reject => {
            pen.circle(0.5, 0.5, 0.4);
            let c = muted(t.objection);
            pen.line_c(&[(0.36, 0.36), (0.64, 0.64)], c);
            pen.line_c(&[(0.64, 0.36), (0.36, 0.64)], c);
        }
        // A link that carries its conclusion firmly, weakly or not at all.
        Icon::Strong => {
            let thick = Stroke::new(width * 2.2, fg);
            painter.line_segment([pen.p(0.5, 0.88), pen.p(0.5, 0.2)], thick);
            painter.line(
                vec![pen.p(0.3, 0.38), pen.p(0.5, 0.16), pen.p(0.7, 0.38)],
                thick,
            );
        }
        Icon::Weak => {
            for i in 0..3 {
                let y = 0.88 - 0.2 * i as f32;
                pen.line(&[(0.5, y), (0.5, y - 0.1)]);
            }
            pen.line(&[(0.34, 0.34), (0.5, 0.18), (0.66, 0.34)]);
        }
        Icon::Nil => {
            pen.circle(0.5, 0.5, 0.32);
            pen.line(&[(0.22, 0.86), (0.78, 0.14)]);
        }
        Icon::Clear => {
            // An eraser on a line.
            pen.line(&[
                (0.18, 0.56),
                (0.5, 0.24),
                (0.8, 0.54),
                (0.48, 0.86),
                (0.18, 0.56),
            ]);
            pen.line(&[(0.34, 0.4), (0.64, 0.7)]);
            pen.line_c(&[(0.48, 0.9), (0.92, 0.9)], muted(t.accent));
        }
        Icon::Markers => {
            pen.rect(0.1, 0.3, 0.78, 0.82, fg);
            pen.disc(0.78, 0.3, 0.14, muted(t.reason));
        }
        Icon::Shading => {
            pen.rect(0.12, 0.2, 0.88, 0.8, fg);
            pen.fill(0.12, 0.2, 0.5, 0.8, muted(t.accent).gamma_multiply(0.6));
        }
    }
}
