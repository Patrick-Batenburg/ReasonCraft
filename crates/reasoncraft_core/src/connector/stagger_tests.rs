use emath::{Pos2, pos2};

use super::*;
use crate::connector::{ELBOW_RADIUS, STUB_PX, Segment, elbow_path_at};

/// An axis-parallel line piece: which axis it runs along (0 = x, 1 = y),
/// its fixed coordinate, and its range along the axis.
fn piece(a: Pos2, b: Pos2) -> Option<(usize, f32, f32, f32)> {
    let d = b - a;
    if d.length() < 1e-3 {
        None
    } else if d.y.abs() < 1e-4 {
        Some((0, a.y, a.x.min(b.x), a.x.max(b.x)))
    } else {
        Some((1, a.x, a.y.min(b.y), a.y.max(b.y)))
    }
}

/// A parent with `n` children fanned out on both sides of its ports, in
/// `dir`: (child edge, port, stub) for each.
fn fan(n: usize, dir: LayoutDirection) -> Vec<(Pos2, Pos2, f32)> {
    let toward = dir.toward_parent();
    let side = toward.rot90();
    let centre = pos2(500.0, 500.0);
    (0..n)
        .map(|i| {
            let k = i as f32 - (n - 1) as f32 / 2.0;
            let port = centre + side * (k * 16.0);
            let child = centre - toward * 150.0 + side * (k * 120.0 + 10.0);
            (child, port, STUB_PX)
        })
        .collect()
}

#[test]
fn elbow_runs_never_overlap_or_cross() {
    for dir in LayoutDirection::ALL {
        for n in [2, 5, 8] {
            let ends = fan(n, dir);
            let turns = elbow_turns(&ends, dir);
            let pieces: Vec<Vec<_>> = ends
                .iter()
                .zip(&turns)
                .map(|(&(from, to, stub), &turn)| {
                    elbow_path_at(from, to, stub, ELBOW_RADIUS, turn, dir)
                        .into_iter()
                        .filter_map(|s| match s {
                            Segment::Line([a, b]) => piece(a, b),
                            Segment::Cubic(_) => None,
                        })
                        .collect()
                })
                .collect();
            for (i, a) in pieces.iter().enumerate() {
                for b in &pieces[i + 1..] {
                    for &(axis, at, lo, hi) in a {
                        for &(axis2, at2, lo2, hi2) in b {
                            if axis == axis2 {
                                let apart = (at - at2).abs() > 1e-3;
                                let overlap = hi.min(hi2) - lo.max(lo2);
                                assert!(apart || overlap <= 1e-3, "{dir:?} {n}: collinear");
                            } else {
                                // Perpendicular pieces cross when each one's
                                // fixed coordinate lies inside the other.
                                let crosses = at2 > lo + 1e-3
                                    && at2 < hi - 1e-3
                                    && at > lo2 + 1e-3
                                    && at < hi2 - 1e-3;
                                assert!(
                                    !crosses,
                                    "{dir:?} {n}: runs cross {:?} {:?} {turns:?}",
                                    (axis, at, lo, hi),
                                    (axis2, at2, lo2, hi2)
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn turns_stay_between_child_and_stub() {
    for dir in LayoutDirection::ALL {
        let main = |p: Pos2| p.to_vec2().dot(dir.toward_parent().abs());
        let ends = fan(12, dir);
        for (&(from, to, stub), turn) in ends.iter().zip(elbow_turns(&ends, dir)) {
            let start = main(to - dir.toward_parent() * stub);
            let (lo, hi) = (main(from).min(start), main(from).max(start));
            assert!(turn > lo && turn < hi, "{dir:?}: {turn} not in {lo}..{hi}");
        }
    }
}

#[test]
fn a_lone_connector_turns_halfway() {
    let dir = LayoutDirection::TopDown;
    let ends = [(pos2(0.0, 200.0), pos2(60.0, 0.0), STUB_PX)];
    assert_eq!(elbow_turns(&ends, dir), [(200.0 + STUB_PX) / 2.0]);
    // Runs that do not overlap share the halfway level.
    let apart = [
        (pos2(0.0, 200.0), pos2(40.0, 0.0), STUB_PX),
        (pos2(300.0, 200.0), pos2(260.0, 0.0), STUB_PX),
    ];
    let turns = elbow_turns(&apart, dir);
    assert_eq!(turns[0], turns[1]);
}
