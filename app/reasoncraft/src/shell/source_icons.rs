//! Original vector glyphs for the 17 source types, shared by the canvas,
//! the "+" menu, the ribbon and the building panel. Each is drawn in one
//! colour with simple strokes and fills, and reads at 14 px and 32 px.

use std::f32::consts::{PI, TAU};
use std::sync::Arc;

use eframe::egui::{
    Atom, AtomKind, AtomPaintArgs, Color32, IntoSizedResult, Painter, Rect, SizedAtomKind, Ui, Vec2,
};
use reasoncraft_core::SourceType;

use crate::scene::{Shape, ShapePen, paint_shapes};

/// Draw the glyph for `t` in `rect` (a square, screen px) in `colour`.
pub fn paint_source_icon(painter: &Painter, rect: Rect, t: &SourceType, colour: Color32) {
    paint_shapes(painter, &source_icon_shapes(rect, t, colour));
}

/// The glyph for `t` in `rect` (a square) in `colour`, as shapes. Unknown
/// types share the generic page of [`SourceType::Source`].
pub fn source_icon_shapes(rect: Rect, t: &SourceType, colour: Color32) -> Vec<Shape> {
    source_icon_shapes_px(rect, t, colour, 1.0)
}

/// [`source_icon_shapes`] for a canvas drawn at a zoom where one screen px
/// is `px` world units: the thinnest stroke stays 1.25 screen px.
pub fn source_icon_shapes_px(rect: Rect, t: &SourceType, colour: Color32, px: f32) -> Vec<Shape> {
    let pen = ShapePen::new(rect, colour, px);
    glyph(&pen, t, colour);
    pen.finish()
}

fn glyph(pen: &ShapePen, t: &SourceType, colour: Color32) {
    match t {
        SourceType::Source | SourceType::Other(_) => {
            page(pen, 0.2, 0.8);
            for y in [0.46, 0.62, 0.78] {
                pen.line(&[(0.34, y), (0.66, y)]);
            }
        }
        SourceType::MyBasis => {
            // A person with a tick: "on my own account".
            pen.circle(0.36, 0.28, 0.15);
            pen.arc(0.36, 0.9, 0.3, PI, TAU);
            pen.line(&[(0.58, 0.7), (0.7, 0.82), (0.92, 0.56)]);
        }
        SourceType::Assertion => {
            // A speech bubble with "!".
            pen.closed(&[
                (0.12, 0.14),
                (0.88, 0.14),
                (0.88, 0.68),
                (0.46, 0.68),
                (0.24, 0.88),
                (0.28, 0.68),
                (0.12, 0.68),
            ]);
            pen.fill(0.455, 0.24, 0.545, 0.47, colour);
            pen.disc(0.5, 0.57, 0.06, colour);
        }
        SourceType::ByDefinition => {
            // A closed book with "≡" on its cover.
            pen.rect(0.2, 0.1, 0.82, 0.9, colour);
            pen.line(&[(0.34, 0.1), (0.34, 0.9)]);
            for y in [0.38, 0.5, 0.62] {
                pen.line(&[(0.46, y), (0.7, y)]);
            }
        }
        SourceType::CaseStudy => {
            // A page under a magnifying glass.
            pen.line(&[(0.5, 0.1), (0.1, 0.1), (0.1, 0.9), (0.42, 0.9)]);
            pen.line(&[(0.5, 0.1), (0.66, 0.26), (0.66, 0.34)]);
            pen.line(&[(0.22, 0.32), (0.46, 0.32)]);
            pen.circle(0.62, 0.6, 0.18);
            pen.line(&[(0.75, 0.73), (0.92, 0.9)]);
        }
        SourceType::CommonBelief => {
            // Three people: one in front of two others.
            for x in [0.22, 0.78] {
                pen.circle(x, 0.3, 0.11);
                pen.arc(x, 0.74, 0.2, PI, TAU);
            }
            pen.circle(0.5, 0.42, 0.13);
            pen.arc(0.5, 0.95, 0.26, PI, TAU);
        }
        SourceType::Data => {
            // A table: header row and a 3 × 3 grid.
            pen.rect(0.1, 0.16, 0.9, 0.84, colour);
            pen.fill(0.1, 0.16, 0.9, 0.36, colour);
            pen.line(&[(0.1, 0.6), (0.9, 0.6)]);
            for x in [0.37, 0.63] {
                pen.line(&[(x, 0.36), (x, 0.84)]);
            }
        }
        SourceType::Event => {
            // A calendar sheet with one marked day.
            pen.rect(0.12, 0.2, 0.88, 0.88, colour);
            pen.line(&[(0.12, 0.4), (0.88, 0.4)]);
            for x in [0.34, 0.66] {
                pen.line(&[(x, 0.1), (x, 0.28)]);
            }
            pen.fill(0.56, 0.56, 0.74, 0.74, colour);
        }
        SourceType::Example => {
            // A light bulb.
            pen.arc(0.5, 0.38, 0.27, 0.75 * PI, 2.25 * PI);
            let (l, r) = (0.5 - 0.27 * 0.707, 0.5 + 0.27 * 0.707);
            let neck = 0.38 + 0.27 * 0.707;
            pen.line(&[(l, neck), (0.38, 0.72), (0.62, 0.72), (r, neck)]);
            pen.line(&[(0.38, 0.81), (0.62, 0.81)]);
            pen.line(&[(0.43, 0.9), (0.57, 0.9)]);
        }
        SourceType::ExpertOpinion => {
            // A mortarboard with its tassel.
            pen.closed(&[(0.5, 0.18), (0.94, 0.38), (0.5, 0.58), (0.06, 0.38)]);
            pen.line(&[(0.24, 0.5), (0.24, 0.76)]);
            pen.arc(0.5, 0.56, 0.32, 0.22 * PI, 0.78 * PI);
            pen.line(&[(0.76, 0.5), (0.76, 0.76)]);
            pen.line(&[(0.94, 0.38), (0.94, 0.66)]);
            pen.disc(0.94, 0.7, 0.05, colour);
        }
        SourceType::Law => {
            // Scales of justice.
            pen.line(&[(0.5, 0.12), (0.5, 0.86)]);
            pen.line(&[(0.3, 0.86), (0.7, 0.86)]);
            pen.line(&[(0.14, 0.24), (0.86, 0.24)]);
            for x in [0.2, 0.8] {
                pen.line(&[(x - 0.13, 0.54), (x, 0.24), (x + 0.13, 0.54)]);
                pen.arc(x, 0.54, 0.14, 0.0, PI);
            }
        }
        SourceType::Media => {
            // A screen on a stand, showing a play button.
            pen.rect(0.08, 0.16, 0.92, 0.72, colour);
            pen.polygon(&[(0.4, 0.3), (0.64, 0.44), (0.4, 0.58)]);
            pen.line(&[(0.5, 0.72), (0.5, 0.86)]);
            pen.line(&[(0.3, 0.86), (0.7, 0.86)]);
        }
        SourceType::PersonalExperience => {
            // A heart: two lobes meeting in a point. An open path that
            // starts and ends at the point, so the tip has no mitre spike.
            let lobe = |cx: f32, a0: f32, a1: f32| {
                (0..=12).map(move |i| {
                    let a = a0 + (a1 - a0) * i as f32 / 12.0;
                    (cx + 0.19 * a.cos(), 0.38 + 0.19 * a.sin())
                })
            };
            let mut pts = vec![(0.5, 0.88)];
            pts.extend(lobe(0.31, 0.75 * PI, 2.0 * PI));
            pts.extend(lobe(0.69, PI, 2.25 * PI));
            pts.push((0.5, 0.88));
            pen.line(&pts);
        }
        SourceType::Publication => {
            // An open book.
            pen.closed(&[
                (0.5, 0.26),
                (0.08, 0.18),
                (0.08, 0.78),
                (0.5, 0.86),
                (0.92, 0.78),
                (0.92, 0.18),
            ]);
            pen.line(&[(0.5, 0.26), (0.5, 0.86)]);
            for y in [0.38, 0.54] {
                pen.line(&[(0.18, y - 0.02), (0.4, y + 0.02)]);
                pen.line(&[(0.6, y + 0.02), (0.82, y - 0.02)]);
            }
        }
        SourceType::Quote => {
            // Opening quotation marks: two dots with rising tails.
            for x in [0.3, 0.7] {
                pen.disc(x, 0.6, 0.14, colour);
                pen.arc(x + 0.22, 0.6, 0.36, PI, 1.38 * PI);
            }
        }
        SourceType::Statistic => {
            // A bar chart on axes.
            pen.line(&[(0.1, 0.1), (0.1, 0.9), (0.92, 0.9)]);
            pen.fill(0.22, 0.56, 0.38, 0.9, colour);
            pen.fill(0.46, 0.24, 0.62, 0.9, colour);
            pen.fill(0.7, 0.42, 0.86, 0.9, colour);
        }
        SourceType::Web => {
            // A globe: outline, meridian and parallels.
            pen.circle(0.5, 0.5, 0.4);
            let meridian: Vec<(f32, f32)> = (0..=24)
                .map(|i| {
                    let a = PI * i as f32 / 24.0 - PI / 2.0;
                    (0.5 + 0.17 * a.cos(), 0.5 + 0.4 * a.sin())
                })
                .collect();
            pen.line(&meridian);
            let back: Vec<(f32, f32)> = meridian.iter().map(|&(x, y)| (1.0 - x, y)).collect();
            pen.line(&back);
            pen.line(&[(0.1, 0.5), (0.9, 0.5)]);
            pen.line(&[(0.18, 0.3), (0.82, 0.3)]);
            pen.line(&[(0.18, 0.7), (0.82, 0.7)]);
        }
    }
}

/// A page from `x0` to `x1`, full height, with a folded top-right corner.
fn page(pen: &ShapePen, x0: f32, x1: f32) {
    let fold = 0.22;
    pen.closed(&[
        (x0, 0.1),
        (x1 - fold, 0.1),
        (x1, 0.1 + fold),
        (x1, 0.9),
        (x0, 0.9),
    ]);
    pen.line(&[(x1 - fold, 0.1), (x1 - fold, 0.1 + fold), (x1, 0.1 + fold)]);
}

/// The glyph for `t` as a button atom of `size` px, painted in the
/// button's text colour (so it dims with a disabled entry).
pub fn source_atom(t: &SourceType, size: f32) -> Atom<'static> {
    let t = t.clone();
    AtomKind::closure(move |_ui: &Ui, _args| {
        let size = Vec2::splat(size);
        IntoSizedResult {
            intrinsic_size: size,
            sized: SizedAtomKind::Paint {
                paint: Arc::new(move |ui: &Ui, args: AtomPaintArgs| {
                    let rect = Rect::from_center_size(args.rect.center(), size);
                    paint_source_icon(ui.painter(), rect, &t, args.fallback_text_color);
                }),
                size,
            },
        }
    })
    .into()
}

/// A window with every glyph at 14 px and 32 px, on a box and on the
/// ribbon, for checking them by eye (debug builds only).
#[cfg(debug_assertions)]
pub fn preview(ctx: &eframe::egui::Context, app: &crate::app::ReasonCraftApp) {
    use eframe::egui::{Grid, Sense, Window};
    if !app.show_source_icons {
        return;
    }
    let tk = app.tokens;
    let mut types = SourceType::ALL.to_vec();
    types.push(SourceType::Other("hologram".to_owned()));
    Window::new(crate::i18n::t("action.source_icons"))
        .resizable(false)
        .show(ctx, |ui| {
            Grid::new("source_icons")
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    for t in &types {
                        ui.label(crate::i18n::t(&format!("source.{}", t.key())));
                        for (px, fill, colour) in [
                            (14.0, tk.box_fill, tk.text_muted),
                            (14.0, tk.box_fill, tk.box_text),
                            (32.0, tk.box_fill, tk.box_text),
                            (32.0, tk.ribbon, tk.text_muted),
                        ] {
                            let (rect, _) =
                                ui.allocate_exact_size(Vec2::splat(40.0), Sense::hover());
                            ui.painter().rect_filled(rect, 3.0, fill);
                            let icon = Rect::from_center_size(rect.center(), Vec2::splat(px));
                            paint_source_icon(ui.painter(), icon, t, colour);
                        }
                        ui.end_row();
                    }
                });
        });
}
