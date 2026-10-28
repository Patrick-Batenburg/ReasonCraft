//! Many connectors into one premise: their arrowheads and bars stay apart
//! in every layout direction and connector style.

use eframe::egui::Rect;
use eframe::egui::{Color32, Pos2, Vec2};
use reasoncraft_core::{
    BoxKind, CardKind, ConnectorStyle, Document, Layout, LayoutDirection, LayoutParams, Measure,
    Parent, layout,
};

use super::*;

/// One text row per "|"-separated part, 15 tall.
struct Fake;

impl Measure for Fake {
    fn text_height(&self, text: &str, _width: f32, _kind: &BoxKind) -> f32 {
        text.split('|').count() as f32 * 15.0
    }
}

impl TextRows for Fake {
    fn rows(&self, text: &str, _width: f32, _kind: &BoxKind) -> Vec<String> {
        text.split('|').map(str::to_owned).collect()
    }
    fn line_height(&self, _size: f32) -> f32 {
        15.0
    }
    fn text_width(&self, text: &str, size: f32) -> f32 {
        text.chars().count() as f32 * size * 0.5
    }
}

/// A claim with one reason, whose premise has `n` child cards of `kind`.
fn crowded_premise(n: usize, kind: CardKind, dir: LayoutDirection) -> Document {
    let mut doc = Document::new("Claim");
    doc.layout_direction = dir;
    let (_, premise) = doc
        .add_card(Parent::Box(doc.root), CardKind::Reason, "Premise")
        .unwrap();
    for i in 0..n {
        doc.add_card(Parent::Box(premise), kind.clone(), &format!("Child {i}"))
            .unwrap();
    }
    doc
}

fn scene_of(doc: &Document, style: ConnectorStyle) -> Scene {
    laid_out(doc, style).1
}

fn laid_out(doc: &Document, style: ConnectorStyle) -> (Layout, Scene) {
    let lay = layout(doc, &Fake, &LayoutParams::default());
    let opts = SceneOptions {
        connector_style: style,
        ..SceneOptions::export(crate::theme::Tokens::dark(), "en")
    };
    let scene = build_scene(doc, &lay, &opts, &Fake);
    (lay, scene)
}

/// The kind of the `i`th child: reasons and objections taking turns.
fn mixed(i: usize) -> CardKind {
    if i.is_multiple_of(2) {
        CardKind::Reason
    } else {
        CardKind::Objection
    }
}

/// The filled triangles of `colour` (arrowheads).
fn triangles(scene: &Scene, colour: Color32) -> Vec<[Pos2; 3]> {
    let corner = |el: &PathEl| match *el {
        PathEl::MoveTo(p) | PathEl::LineTo(p) => Some(p),
        _ => None,
    };
    let mut out = Vec::new();
    for s in &scene.shapes {
        if let Shape::Path {
            points,
            fill: Some(fill),
            stroke: None,
            ..
        } = s
            && *fill == colour
            && let [a, b, c, PathEl::Close] = &points[..]
            && let (Some(a), Some(b), Some(c)) = (corner(a), corner(b), corner(c))
        {
            out.push([a, b, c]);
        }
    }
    out
}

/// Whether two triangles share interior (separating-axis test).
fn triangles_intersect(t: &[Pos2; 3], u: &[Pos2; 3]) -> bool {
    let edges = |tri: &[Pos2; 3]| {
        (0..3)
            .map(|i| tri[(i + 1) % 3] - tri[i])
            .collect::<Vec<_>>()
    };
    let range = |tri: &[Pos2; 3], axis: Vec2| {
        let d = tri.map(|p| p.to_vec2().dot(axis));
        (d[0].min(d[1]).min(d[2]), d[0].max(d[1]).max(d[2]))
    };
    edges(t).into_iter().chain(edges(u)).all(|e| {
        let axis = e.rot90();
        let ((a0, a1), (b0, b1)) = (range(t, axis), range(u, axis));
        a1 > b0 + 1e-3 && b1 > a0 + 1e-3
    })
}

/// The end bars of `colour`: thick straight lines.
fn bars(scene: &Scene, colour: Color32, thickness: f32) -> Vec<(Pos2, Pos2)> {
    let mut out = Vec::new();
    for s in &scene.shapes {
        if let Shape::Path {
            points,
            stroke: Some((w, c)),
            fill: None,
            ..
        } = s
            && *c == colour
            && (*w - thickness).abs() < 1e-4
            && let [PathEl::MoveTo(a), PathEl::LineTo(b)] = points[..]
        {
            out.push((a, b));
        }
    }
    out
}

/// The shortest distance between segments `p` and `q` (they do not cross).
fn segment_distance(p: (Pos2, Pos2), q: (Pos2, Pos2)) -> f32 {
    let to_segment = |x: Pos2, (a, b): (Pos2, Pos2)| {
        let ab = b - a;
        let t = ((x - a).dot(ab) / ab.length_sq()).clamp(0.0, 1.0);
        (x - (a + ab * t)).length()
    };
    [
        to_segment(p.0, q),
        to_segment(p.1, q),
        to_segment(q.0, p),
        to_segment(q.1, p),
    ]
    .into_iter()
    .fold(f32::MAX, f32::min)
}

#[test]
fn nine_arrowheads_do_not_overlap_any_direction() {
    let reason = crate::theme::Tokens::dark().reason;
    for dir in LayoutDirection::ALL {
        let doc = crowded_premise(9, CardKind::Reason, dir);
        for style in ConnectorStyle::ALL {
            let scene = scene_of(&doc, style);
            let heads = triangles(&scene, reason);
            assert_eq!(heads.len(), 10, "{dir:?} {style:?}: nine and the reason's");
            for (i, t) in heads.iter().enumerate() {
                for u in &heads[i + 1..] {
                    assert!(
                        !triangles_intersect(t, u),
                        "{dir:?} {style:?}: {t:?} and {u:?} overlap"
                    );
                }
            }
        }
    }
}

/// Opposes bars into one premise keep clear of each other too (they once
/// overlapped left-to-right, about 5.7 units apart).
#[test]
fn opposes_bars_do_not_overlap_any_direction() {
    let objection = crate::theme::Tokens::dark().objection;
    for dir in LayoutDirection::ALL {
        for n in [2, 3, 6] {
            let doc = crowded_premise(n, CardKind::Objection, dir);
            let scene = scene_of(&doc, ConnectorStyle::Elbow);
            let found = bars(&scene, objection, super::connectors::BAR_THICKNESS);
            assert_eq!(found.len(), n, "{dir:?}: {n} bars");
            for (i, &p) in found.iter().enumerate() {
                for &q in &found[i + 1..] {
                    let d = segment_distance(p, q);
                    assert!(
                        d > super::connectors::BAR_THICKNESS,
                        "{dir:?}, {n} bars: {p:?} and {q:?} only {d} apart"
                    );
                }
            }
        }
    }
}

/// In every style: no two arrowheads of `doc` intersect, no two bars come
/// within a bar's thickness, and no arrowhead or bar corner lies inside a
/// box or frame whose edge holds its port.
fn assert_markers_clear(doc: &Document, what: &str) {
    let tk = crate::theme::Tokens::dark();
    let thick = super::connectors::BAR_THICKNESS;
    for style in ConnectorStyle::ALL {
        let (lay, scene) = laid_out(doc, style);
        let heads = triangles(&scene, tk.reason);
        let found = bars(&scene, tk.objection, thick);
        // Each reason ends in an arrowhead, each objection in a bar: the
        // checks below see every marker.
        let cards = |kind: CardKind| doc.cards.values().filter(|c| c.kind == kind).count();
        assert_eq!(heads.len(), cards(CardKind::Reason), "{what} {style:?}");
        assert_eq!(found.len(), cards(CardKind::Objection), "{what} {style:?}");
        for (i, t) in heads.iter().enumerate() {
            for u in &heads[i + 1..] {
                assert!(!triangles_intersect(t, u), "{what} {style:?}: {t:?} {u:?}");
            }
        }
        for (i, &p) in found.iter().enumerate() {
            for &q in &found[i + 1..] {
                let d = segment_distance(p, q);
                assert!(d > thick, "{what} {style:?}: bars {p:?} {q:?} {d} apart");
            }
        }
        let rects: Vec<Rect> = lay
            .boxes
            .values()
            .chain(lay.cards.values())
            .copied()
            .collect();
        let corners = heads
            .iter()
            .map(|t| t.to_vec())
            .chain(found.iter().map(|&(a, b)| vec![a, b]));
        for corners in corners {
            let mid = corners
                .iter()
                .fold(Pos2::ZERO, |m, c| m + c.to_vec2() / corners.len() as f32);
            let port = lay
                .connectors
                .iter()
                .map(|c| c.to)
                .min_by(|a, b| (*a - mid).length().total_cmp(&(*b - mid).length()))
                .unwrap();
            let on_edge =
                |r: &Rect| r.expand(0.01).contains(port) && !r.shrink(0.01).contains(port);
            for r in rects.iter().filter(|r| on_edge(r)) {
                for c in &corners {
                    assert!(
                        !r.shrink(1e-3).contains(*c),
                        "{what} {style:?}: {c:?} inside {r:?} (port {port:?})"
                    );
                }
            }
        }
    }
}

/// Many connectors into one premise: the premise grows to fit them.
#[test]
fn crowded_premise_markers_stay_clear() {
    for dir in LayoutDirection::ALL {
        for n in [13, 20] {
            let mut doc = Document::new("Claim");
            doc.layout_direction = dir;
            let (_, premise) = doc
                .add_card(Parent::Box(doc.root), CardKind::Reason, "Premise")
                .unwrap();
            for i in 0..n {
                doc.add_card(Parent::Box(premise), mixed(i), &format!("Child {i}"))
                    .unwrap();
            }
            assert_markers_clear(&doc, &format!("{dir:?} {n}"));
        }
    }
}

/// Whole-card connectors join the middle premise's ports (an odd number of
/// premises) or spread about the gap between the middle two (even), and
/// stay clear of the premises' own.
#[test]
fn whole_card_ports_stay_clear_of_premise_ports() {
    for dir in LayoutDirection::ALL {
        let cases = [
            (3, 1, 8),
            (2, 9, 5),
            (2, 1, 3),
            (4, 3, 2),
            (2, 0, 12),
            (2, 0, 24),
            (4, 0, 6),
        ];
        for (premises, own, whole) in cases {
            let mut doc = Document::new("Claim");
            doc.layout_direction = dir;
            let (card, first) = doc
                .add_card(Parent::Box(doc.root), CardKind::Reason, "Premise 0")
                .unwrap();
            let mut ids = vec![first];
            for i in 1..premises {
                ids.push(
                    doc.add_premise(card, None, &format!("Premise {i}"))
                        .unwrap(),
                );
            }
            for &p in &ids[(premises - 1) / 2..=premises / 2] {
                for i in 0..own {
                    doc.add_card(Parent::Box(p), mixed(i), &format!("Own {i}"))
                        .unwrap();
                }
            }
            for i in 0..whole {
                doc.add_card(Parent::Card(card), mixed(i + 1), &format!("Whole {i}"))
                    .unwrap();
            }
            let what = format!("{dir:?} {premises}/{own}/{whole}");
            assert_markers_clear(&doc, &what);
            assert_ports_spaced(&doc, &what);
        }
    }
}

/// Every stub of `doc` runs straight along the main axis (none fans out),
/// and ports on one line are `port_spacing` apart or more.
fn assert_ports_spaced(doc: &Document, what: &str) {
    let p = LayoutParams::default();
    let lay = layout(doc, &Fake, &p);
    let toward = doc.layout_direction.toward_parent();
    let main = |v: Pos2| v.to_vec2().dot(toward.abs());
    let cross = |v: Pos2| v.to_vec2().dot(toward.rot90().abs());
    for (i, c) in lay.connectors.iter().enumerate() {
        assert_eq!(c.approach, toward, "{what}: a fanned stub");
        for d in &lay.connectors[i + 1..] {
            if (main(c.to) - main(d.to)).abs() < 0.01 {
                let gap = (cross(c.to) - cross(d.to)).abs();
                assert!(gap >= p.port_spacing - 1e-3, "{what}: ports {gap} apart");
            }
        }
    }
}
