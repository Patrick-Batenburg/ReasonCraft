//! Drawing a [`Scene`] with egui, through the canvas camera.

use eframe::egui::{Align, Align2, Color32, Painter, Pos2, Stroke, StrokeKind, epaint, vec2};

use super::{PathEl, Scene, Shape, StrokeSpec};
use crate::canvas::Camera;
use crate::canvas::images::{ImageCache, paint_image};
use crate::canvas::paint::screen_font;
use crate::theme::Tokens;

/// Dash and gap of dashed paths, in screen px.
const DASH: f32 = 6.0;
const GAP: f32 = 4.0;

/// Draw `scene` through `cam`. Shapes outside the painter's clip rect are
/// skipped; text smaller than a few screen px is not drawn. Images come
/// from `images` (a placeholder in `tokens`' colours while one is pending
/// or missing).
pub fn paint_scene(
    painter: &Painter,
    scene: &Scene,
    cam: &Camera,
    images: &ImageCache,
    tokens: &Tokens,
) {
    let clip = painter.clip_rect();
    for shape in &scene.shapes {
        match shape {
            Shape::Rect { .. } | Shape::Circle { .. } => {
                let r = cam.world_rect_to_screen(shape.bounds());
                if clip.intersects(r.expand(4.0)) {
                    paint_shape(painter, shape, cam);
                }
            }
            Shape::Image { rect, asset } => {
                let r = cam.world_rect_to_screen(*rect);
                if clip.intersects(r.expand(4.0)) {
                    paint_image(painter, r, asset, images, tokens, cam.zoom);
                }
            }
            _ => paint_shape(painter, shape, cam),
        }
    }
}

/// Draw `shapes` given in screen px (an icon, say). Images are skipped.
pub fn paint_shapes(painter: &Painter, shapes: &[Shape]) {
    let cam = Camera::default();
    for shape in shapes {
        paint_shape(painter, shape, &cam);
    }
}

fn stroke(spec: &Option<StrokeSpec>, zoom: f32) -> Stroke {
    spec.map_or(Stroke::NONE, |(w, c)| Stroke::new(w * zoom, c))
}

fn paint_shape(painter: &Painter, shape: &Shape, cam: &Camera) {
    let z = cam.zoom;
    match shape {
        Shape::Rect {
            rect,
            radius,
            fill,
            stroke: s,
        } => {
            // Fill and stroke apart, so a translucent stroke blends over
            // the fill.
            let r = cam.world_rect_to_screen(*rect);
            if *fill != Color32::TRANSPARENT {
                painter.rect_filled(r, radius * z, *fill);
            }
            if let Some((w, _)) = s {
                let stroke = stroke(s, z);
                if *fill == Color32::TRANSPARENT {
                    // An outline: the same geometry centred on the inset
                    // edge, which egui snaps symmetrically (crisp icons).
                    let half = w * z / 2.0;
                    let radius = (radius * z - half).max(0.0);
                    painter.rect_stroke(r.shrink(half), radius, stroke, StrokeKind::Middle);
                } else {
                    painter.rect_stroke(r, radius * z, stroke, StrokeKind::Inside);
                }
            }
        }
        Shape::Circle {
            center,
            radius,
            fill,
            stroke: s,
        } => {
            let c = cam.world_to_screen(*center);
            let fill = fill.unwrap_or(Color32::TRANSPARENT);
            painter.circle(c, radius * z, fill, stroke(s, z));
        }
        Shape::Path {
            points,
            stroke: s,
            fill,
            dashed,
        } => path(painter, points, stroke(s, z), *fill, *dashed, cam),
        Shape::Text {
            origin,
            rows,
            size,
            bold,
            color,
            align,
            line_height,
        } => {
            let Some(font) = screen_font(*size, z) else {
                return;
            };
            let anchor = match align {
                Align::Min => Align2::LEFT_TOP,
                Align::Center => Align2::CENTER_TOP,
                Align::Max => Align2::RIGHT_TOP,
            };
            // Faux bold: the default fonts have no bold face.
            let bold = bold.then(|| vec2(0.45 * z.min(1.5), 0.0));
            for (i, row) in rows.iter().enumerate() {
                if row.is_empty() {
                    continue;
                }
                let at = cam.world_to_screen(*origin + vec2(0.0, i as f32 * line_height));
                let galley = painter.layout_no_wrap(row.clone(), font.clone(), *color);
                let rect = anchor.anchor_size(at, galley.size());
                if let Some(offset) = bold {
                    painter.galley(rect.min + offset, galley.clone(), *color);
                }
                painter.galley(rect.min, galley, *color);
            }
        }
        Shape::Image { .. } => {}
    }
}

/// A path: runs of lines become polylines, a lone stroked curve a Bézier
/// shape; filled and dashed paths are flattened first.
fn path(
    painter: &Painter,
    els: &[PathEl],
    stroke: Stroke,
    fill: Option<Color32>,
    dashed: bool,
    cam: &Camera,
) {
    let flatten = fill.is_some() || dashed;
    let mut run: Vec<Pos2> = Vec::new();
    let mut start = None;
    let mut closed = false;
    let flush = |run: &mut Vec<Pos2>, closed: bool| {
        line(painter, std::mem::take(run), stroke, fill, closed, dashed);
    };
    for el in els {
        match *el {
            PathEl::MoveTo(p) => {
                flush(&mut run, closed);
                closed = false;
                let p = cam.world_to_screen(p);
                start = Some(p);
                run.push(p);
            }
            PathEl::LineTo(p) => run.push(cam.world_to_screen(p)),
            PathEl::CubicTo(a, b, c) => {
                let from = run.last().copied().unwrap_or(Pos2::ZERO);
                let pts = [
                    from,
                    cam.world_to_screen(a),
                    cam.world_to_screen(b),
                    cam.world_to_screen(c),
                ];
                let curve = epaint::CubicBezierShape::from_points_stroke(
                    pts,
                    false,
                    Color32::TRANSPARENT,
                    stroke,
                );
                if flatten {
                    run.extend(curve.flatten(None).into_iter().skip(1));
                } else {
                    flush(&mut run, false);
                    painter.add(curve);
                    run.push(pts[3]);
                }
            }
            PathEl::Close => {
                closed = true;
                if dashed && let Some(s) = start {
                    run.push(s);
                }
            }
        }
    }
    flush(&mut run, closed);
}

fn line(
    painter: &Painter,
    pts: Vec<Pos2>,
    stroke: Stroke,
    fill: Option<Color32>,
    closed: bool,
    dashed: bool,
) {
    if pts.len() < 2 {
        return;
    }
    if let Some(fill) = fill {
        painter.add(epaint::Shape::convex_polygon(pts, fill, stroke));
    } else if dashed {
        painter.extend(epaint::Shape::dashed_line(&pts, stroke, DASH, GAP));
    } else if closed {
        painter.add(epaint::Shape::closed_line(pts, stroke));
    } else if pts.len() == 2 {
        painter.line_segment([pts[0], pts[1]], stroke);
    } else {
        painter.add(epaint::Shape::line(pts, stroke));
    }
}
