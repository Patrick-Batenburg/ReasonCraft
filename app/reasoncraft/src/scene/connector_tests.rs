//! Connector styles in scenes, their painting and the overlays that sit
//! on boxes (link glyph, comment badge, rating marker).

use eframe::egui::{self, Color32, Rect, epaint, pos2, vec2};
use reasoncraft_core::{
    Acceptance, BoxKind, CardKind, Comment, ConnectorStyle, Document, Layout, LayoutDirection,
    LayoutParams, Measure, NodeId, Parent, Uuid, layout,
};

use super::*;
use crate::canvas::comments::badge_rect;
use crate::canvas::links::link_rect;
use crate::canvas::scene_paint::paint_shapes;
use crate::scene::rating::marker_circle;

const LINE: f32 = 15.0;

/// One row per "|"-separated part, like the scene tests' measure.
struct Fake;

impl Measure for Fake {
    fn text_height(&self, text: &str, _width: f32, _kind: &BoxKind) -> f32 {
        text.split('|').count() as f32 * LINE
    }
}

impl TextRows for Fake {
    fn rows(&self, text: &str, _width: f32, _kind: &BoxKind) -> Vec<String> {
        text.split('|').map(str::to_owned).collect()
    }
    fn line_height(&self, _size: f32) -> f32 {
        LINE
    }
    fn text_width(&self, text: &str, size: f32) -> f32 {
        text.chars().count() as f32 * size * 0.5
    }
}

/// "Claim" with `n` reason cards, laid out in `dir`; returns the premises.
fn claim_with_reasons(n: usize, dir: LayoutDirection) -> (Document, Vec<NodeId>) {
    let mut doc = Document::new("Claim");
    doc.layout_direction = dir;
    let premises = (0..n)
        .map(|i| {
            let reason = format!("Reason {i}");
            doc.add_card(Parent::Box(doc.root), CardKind::Reason, &reason)
                .unwrap()
                .1
        })
        .collect();
    (doc, premises)
}

fn scene_in(doc: &Document, style: ConnectorStyle) -> (Layout, Scene) {
    let lay = layout(doc, &Fake, &LayoutParams::default());
    let opts = SceneOptions {
        connector_style: style,
        ..SceneOptions::export(crate::theme::Tokens::dark(), "en")
    };
    let scene = build_scene(doc, &lay, &opts, &Fake);
    (lay, scene)
}

/// The cubic Béziers of stroked paths: (start, c1, c2, end).
fn cubics(scene: &Scene) -> Vec<[Pos2; 4]> {
    let mut out = Vec::new();
    for s in &scene.shapes {
        let Shape::Path { points, .. } = s else {
            continue;
        };
        let mut at = Pos2::ZERO;
        for el in points {
            match *el {
                PathEl::MoveTo(p) | PathEl::LineTo(p) => at = p,
                PathEl::CubicTo(a, b, c) => {
                    out.push([at, a, b, c]);
                    at = c;
                }
                PathEl::Close => {}
            }
        }
    }
    out
}

#[test]
fn scene_uses_selected_style() {
    let (doc, _) = claim_with_reasons(3, LayoutDirection::TopDown);
    let (lay, curved) = scene_in(&doc, ConnectorStyle::Curved);
    let n = lay.connectors.len();
    assert_eq!(cubics(&curved).len(), n, "one S-curve each");

    let (_, tapered) = scene_in(&doc, ConnectorStyle::Tapered);
    let strips = tapered
        .shapes
        .iter()
        .filter(|s| matches!(s, Shape::Strip { left, right, .. } if left.len() == right.len()))
        .count();
    assert_eq!(strips, n);
    assert!(cubics(&tapered).is_empty(), "no stroked curves");

    let (_, elbow) = scene_in(&doc, ConnectorStyle::Elbow);
    let corners = cubics(&elbow);
    assert!(!corners.is_empty() && corners.len() <= 2 * n, "{corners:?}");
    for [a, b, c, d] in corners {
        // Quarter circles: handles 0.5523 of the radius.
        let r = (d - a).length() / 2f32.sqrt();
        assert!(((b - a).length() - 0.5523 * r).abs() < 1e-3);
        assert!(((d - c).length() - 0.5523 * r).abs() < 1e-3);
    }
}

/// A tapered band carries on along its stub at its full width, into an
/// arrowhead wider than the band, instead of narrowing to a thin line.
#[test]
fn tapered_stub_keeps_the_band_width() {
    let (doc, _) = claim_with_reasons(2, LayoutDirection::TopDown);
    let (_, scene) = scene_in(&doc, ConnectorStyle::Tapered);
    let mut checked = 0;
    for (i, shape) in scene.shapes.iter().enumerate() {
        let Shape::Strip { left, right, .. } = shape else {
            continue;
        };
        let wide = (left[left.len() - 1] - right[right.len() - 1]).length();
        // The band's stub, then its arrowhead.
        let Some(Shape::Path {
            stroke: Some((w, _)),
            ..
        }) = scene.shapes.get(i + 1)
        else {
            panic!("no stub after band {i}");
        };
        assert!((w - wide).abs() < 1e-3, "stub {w} vs band end {wide}");
        let Some(Shape::Path {
            points,
            fill: Some(_),
            ..
        }) = scene.shapes.get(i + 2)
        else {
            panic!("no arrowhead after band {i}");
        };
        let corners: Vec<Pos2> = points
            .iter()
            .filter_map(|p| match p {
                PathEl::MoveTo(q) | PathEl::LineTo(q) => Some(*q),
                _ => None,
            })
            .collect();
        let head = (corners[1] - corners[2]).length();
        assert!(head > 1.2 * wide, "arrowhead {head} vs band {wide}");
        checked += 1;
    }
    assert_eq!(checked, 2);
}

#[test]
fn screen_px_widths_follow_the_zoom_with_a_floor() {
    use reasoncraft_core::connector::{MIN_VISIBLE_PX, Strength, stroke_px};
    let (doc, _) = claim_with_reasons(1, LayoutDirection::TopDown);
    let lay = layout(&doc, &Fake, &LayoutParams::default());
    for zoom in [0.1, 0.5, 1.0, 2.0] {
        let mut o = SceneOptions::export(crate::theme::Tokens::dark(), "en");
        o.zoom = zoom;
        let scene = build_scene(&doc, &lay, &o, &Fake);
        let width = scene
            .shapes
            .iter()
            .find_map(|s| match s {
                Shape::Path {
                    points,
                    stroke: Some((w, _)),
                    ..
                } if points.iter().any(|p| matches!(p, PathEl::CubicTo(..))) => Some(*w),
                _ => None,
            })
            .unwrap();
        let px = stroke_px(
            reasoncraft_core::ConnectorKind::Supports,
            Strength::Normal,
            zoom,
        );
        assert!(
            (width * zoom - px.max(MIN_VISIBLE_PX)).abs() < 1e-5,
            "{zoom}"
        );
    }
}

#[test]
fn strip_paints_as_mesh() {
    // A U-turn: concave, so no convex polygon could fill it.
    let left = vec![
        pos2(0.0, 0.0),
        pos2(0.0, 40.0),
        pos2(40.0, 40.0),
        pos2(40.0, 0.0),
    ];
    let right = vec![
        pos2(8.0, 0.0),
        pos2(8.0, 32.0),
        pos2(32.0, 32.0),
        pos2(32.0, 0.0),
    ];
    let strip = Shape::Strip {
        left,
        right,
        fill: Color32::RED,
    };
    let ctx = egui::Context::default();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        let painter = ui.ctx().layer_painter(egui::LayerId::background());
        paint_shapes(&painter, std::slice::from_ref(&strip));
    });
    out.textures_delta.clear();
    let mesh = out.shapes.iter().find_map(|c| match &c.shape {
        epaint::Shape::Mesh(m) => Some(m.clone()),
        _ => None,
    });
    let mesh = mesh.expect("a mesh");
    assert_eq!(mesh.vertices.len(), 8);
    assert_eq!(mesh.indices.len(), 3 * 6, "two triangles per step");
}

#[test]
fn svg_strip_is_filled_path() {
    let scene = Scene {
        shapes: vec![Shape::Strip {
            left: vec![pos2(0.0, 0.0), pos2(10.0, 20.0)],
            right: vec![pos2(4.0, 0.0), pos2(16.0, 20.0)],
            fill: Color32::from_rgb(0x12, 0x34, 0x56),
        }],
        bounds: Rect::from_min_max(pos2(0.0, 0.0), pos2(16.0, 20.0)),
    };
    let tokens = crate::theme::Tokens::dark();
    let opts = crate::export::SvgOptions {
        text_as_paths: false,
        ..crate::export::SvgOptions::new(&tokens)
    };
    let svg = crate::export::scene_to_svg(&scene, &Default::default(), &opts, "x");
    assert!(
        svg.contains(r##"<path d="M0 0 L10 20 L16 20 L4 0 Z" fill="#123456"/>"##),
        "{svg}"
    );
}

#[test]
fn link_glyph_clear_of_arrowheads() {
    for (dir, n) in LayoutDirection::ALL
        .into_iter()
        .flat_map(|d| [(d, 5), (d, 9)])
    {
        let (mut doc, _) = claim_with_reasons(n, dir);
        let root = doc.root;
        doc.boxes.get_mut(&root).unwrap().link = Some("https://example.org".into());
        for style in ConnectorStyle::ALL {
            let (lay, scene) = scene_in(&doc, style);
            let glyph = link_rect(lay.boxes[&root], lay.indicator_row(root), 0.0);
            let arrowheads: Vec<Rect> = scene
                .shapes
                .iter()
                .filter(|s| {
                    matches!(
                        s,
                        Shape::Path {
                            fill: Some(_),
                            stroke: None,
                            ..
                        }
                    )
                })
                .map(Shape::bounds)
                .collect();
            assert_eq!(arrowheads.len(), n, "{dir:?} {style:?}");
            for a in arrowheads {
                assert!(!a.intersects(glyph), "{dir:?} {style:?}: {a:?} {glyph:?}");
            }
        }
    }
}

/// The origin and row count of each text shape that starts inside `world`.
fn text_rows(scene: &Scene, world: Rect) -> Vec<(Pos2, usize)> {
    let rows: Vec<(Pos2, usize)> = scene
        .shapes
        .iter()
        .filter_map(|s| match s {
            Shape::Text { origin, rows, .. } if world.contains(*origin) => {
                Some((*origin, rows.len()))
            }
            _ => None,
        })
        .collect();
    assert!(!rows.is_empty(), "text in {world:?}");
    rows
}

#[test]
fn marker_and_badge_inside_box() {
    for dir in LayoutDirection::ALL {
        let mut doc = Document::new("Claim");
        doc.layout_direction = dir;
        let (card, first) = doc
            .add_card(Parent::Box(doc.root), CardKind::Reason, "A")
            .unwrap();
        let mut premises = vec![first];
        for text in ["B|two rows", "C|three|rows"] {
            premises.push(doc.add_premise(card, None, text).unwrap());
        }
        for &p in &premises {
            let node = doc.boxes.get_mut(&p).unwrap();
            node.acceptance = Some(Acceptance::Accept);
            node.link = Some("https://example.org".into());
            for _ in 0..100 {
                node.comments.push(Comment {
                    id: Uuid::new_v4(),
                    author: "A".into(),
                    text: "T".into(),
                    verdict: None,
                    created: String::new(),
                    extra: Default::default(),
                });
            }
        }
        let (lay, scene) = scene_in(&doc, ConnectorStyle::Curved);
        for &p in &premises {
            let world = lay.boxes[&p];
            let row = lay.indicator_row(p);
            assert!(row > 0.0, "{dir:?}: an indicator row");
            let (c, r) = marker_circle(world, row);
            let marker = Rect::from_center_size(c, vec2(2.0 * r, 2.0 * r));
            let badge = badge_rect(world, row, 0.0, 100);
            let glyph = link_rect(world, row, 0.0);
            for (name, rect) in [("marker", marker), ("badge", badge), ("glyph", glyph)] {
                assert!(
                    world.contains_rect(rect),
                    "{dir:?} {name}: {rect:?} in {world:?}"
                );
                for &q in premises.iter().filter(|&&q| q != p) {
                    assert!(
                        !rect.intersects(lay.boxes[&q]),
                        "{dir:?} {name} on a neighbour"
                    );
                }
            }
            assert!(
                !marker.intersects(badge) && !marker.intersects(glyph),
                "{dir:?}"
            );
            assert!(
                badge.left() >= glyph.right(),
                "{dir:?}: badge after the glyph"
            );
            // No mark covers a row of the box's text (taken as wide as the
            // box, since rows are centred).
            for (origin, rows) in text_rows(&scene, world) {
                for i in 0..rows {
                    let top = origin.y + i as f32 * LINE;
                    let text = Rect::from_x_y_ranges(world.x_range(), top..=top + LINE);
                    for mark in [marker, badge.expand2(vec2(0.0, 1.75)), glyph] {
                        assert!(!mark.intersects(text), "{dir:?}: {mark:?} on {text:?}");
                    }
                }
            }
            // The scene draws the marker where `marker_circle` says.
            let drawn = scene
                .shapes
                .iter()
                .any(|s| matches!(s, Shape::Circle { center, .. } if *center == c));
            assert!(drawn, "{dir:?}: marker in the scene");
        }
    }
}

/// The axis-parallel straight pieces of each elbow body (a stroked path
/// with corners; stubs and their markers are drawn apart) of `scene`:
/// (axis, fixed coordinate, range start, range end); axis 0 runs along x.
fn straight_pieces(scene: &Scene) -> Vec<Vec<(usize, f32, f32, f32)>> {
    let mut out = Vec::new();
    for s in &scene.shapes {
        let Shape::Path {
            points,
            stroke: Some(_),
            fill: None,
            ..
        } = s
        else {
            continue;
        };
        if !points.iter().any(|p| matches!(p, PathEl::CubicTo(..))) {
            continue;
        }
        let mut pieces = Vec::new();
        let mut at = Pos2::ZERO;
        for el in points {
            match *el {
                PathEl::MoveTo(p) => at = p,
                PathEl::LineTo(p) => {
                    let d = p - at;
                    if d.length() > 1e-3 && d.y.abs() < 1e-4 {
                        pieces.push((0, at.y, at.x.min(p.x), at.x.max(p.x)));
                    } else if d.length() > 1e-3 && d.x.abs() < 1e-4 {
                        pieces.push((1, at.x, at.y.min(p.y), at.y.max(p.y)));
                    }
                    at = p;
                }
                PathEl::CubicTo(_, _, p) => at = p,
                PathEl::Close => {}
            }
        }
        out.push(pieces);
    }
    out
}

#[test]
fn elbow_connectors_never_run_on_top_of_each_other() {
    for dir in LayoutDirection::ALL {
        let mut doc = reasoncraft_core::fixtures::sample_iot_like(40);
        doc.layout_direction = dir;
        let (_, scene) = scene_in(&doc, ConnectorStyle::Elbow);
        let paths = straight_pieces(&scene);
        for (i, a) in paths.iter().enumerate() {
            for b in &paths[i + 1..] {
                for &(axis, at, lo, hi) in a {
                    for &(axis2, at2, lo2, hi2) in b {
                        let collinear = axis == axis2 && (at - at2).abs() < 1e-3;
                        let overlap = hi.min(hi2) - lo.max(lo2);
                        assert!(
                            !collinear || overlap <= 1e-3,
                            "{dir:?}: {:?} on {:?}",
                            (axis, at, lo, hi),
                            (axis2, at2, lo2, hi2)
                        );
                    }
                }
            }
        }
    }
}
