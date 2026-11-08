//! Elbow connectors: no two run on top of each other, in any layout
//! direction.

use reasoncraft_core::{ConnectorStyle, LayoutDirection};

use super::connector_tests::scene_in;
use super::*;

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
