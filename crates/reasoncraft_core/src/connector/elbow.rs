//! The Elbow style: segments along the axes with rounded corners, like an
//! org chart.

use emath::{Pos2, Vec2};

use super::Segment;
use crate::model::LayoutDirection;

/// Corner radius of elbow connectors, in world units. Each corner is
/// clamped to half the shorter segment next to it.
pub const ELBOW_RADIUS: f32 = 8.0;

/// The handle length of a quarter circle drawn as a cubic, per unit radius.
const QUARTER_K: f32 = 0.5523;

/// Below this cross offset (world units) the path is one straight line.
const STRAIGHT_BELOW: f32 = 0.5;

/// Path from the child's near edge `from` to the parent port `to`: along
/// the main axis to halfway between `from` and the stub start, across to
/// the port, along the main axis to the stub start, then the straight stub
/// (`stub` long, along `dir.toward_parent()`). Corners are quarter circles
/// of `radius`. When `from` is (nearly) in line with the port, one line.
pub fn elbow_path(
    from: Pos2,
    to: Pos2,
    stub: f32,
    radius: f32,
    dir: LayoutDirection,
) -> Vec<Segment> {
    let main = |p: Pos2| p.to_vec2().dot(dir.toward_parent().abs());
    let stub_start = to - dir.toward_parent() * stub;
    let mid = (main(from) + main(stub_start)) / 2.0;
    elbow_path_at(from, to, stub, radius, mid, dir)
}

/// Like [`elbow_path`], but the run across goes at main coordinate `turn`
/// (world y top-down and bottom-up, x left-to-right), which should lie
/// between `from` and the stub start (see [`elbow_turns`](super::elbow_turns)).
pub fn elbow_path_at(
    from: Pos2,
    to: Pos2,
    stub: f32,
    radius: f32,
    turn: f32,
    dir: LayoutDirection,
) -> Vec<Segment> {
    let toward = dir.toward_parent();
    let stub_start = to - toward * stub;
    // The main and cross axes, with unit length and positive.
    let main_axis = toward.abs();
    let cross = toward.rot90().abs();
    let along = |p: Pos2| p.to_vec2().dot(main_axis);
    let offset = (stub_start - from).dot(cross);
    if offset.abs() < STRAIGHT_BELOW {
        return vec![Segment::Line([from, to])];
    }
    let first = turn - along(from);
    let last = along(stub_start) - turn;
    let turn_a = from + main_axis * first;
    let turn_b = turn_a + cross * offset;
    let r = radius
        .min(first.abs() / 2.0)
        .min(last.abs() / 2.0)
        .min(offset.abs() / 2.0)
        .max(0.0);
    let mut out = Vec::with_capacity(6);
    let mut at = from;
    for (corner, next) in [(turn_a, turn_b), (turn_b, stub_start)] {
        let into = unit(corner - at);
        let out_dir = unit(next - corner);
        let (enter, leave) = (corner - into * r, corner + out_dir * r);
        line(&mut out, at, enter);
        if r > 0.0 {
            let k = QUARTER_K * r;
            out.push(Segment::Cubic([
                enter,
                enter + into * k,
                leave - out_dir * k,
                leave,
            ]));
        }
        at = leave;
    }
    line(&mut out, at, stub_start);
    out.push(Segment::Line([stub_start, to]));
    out
}

/// `v` scaled to unit length, or zero.
fn unit(v: Vec2) -> Vec2 {
    if v.length() > 0.0 {
        v.normalized()
    } else {
        Vec2::ZERO
    }
}

/// A line from `a` to `b`, unless they coincide.
fn line(out: &mut Vec<Segment>, a: Pos2, b: Pos2) {
    if (b - a).length() > 1e-4 {
        out.push(Segment::Line([a, b]));
    }
}

#[cfg(test)]
mod tests {
    use super::super::STUB_PX;
    use super::*;

    fn ends(s: &Segment) -> (Pos2, Pos2) {
        match *s {
            Segment::Line([a, b]) => (a, b),
            Segment::Cubic([a, _, _, d]) => (a, d),
        }
    }

    #[test]
    fn elbow_segments_axis_aligned_except_corners() {
        let to = Pos2::new(100.0, 50.0);
        for dir in LayoutDirection::ALL {
            for side in [-1.0, 1.0] {
                let shift = dir.toward_parent().rot90() * 40.0 * side;
                let from = to - dir.toward_parent() * 150.0 + shift;
                let path = elbow_path(from, to, STUB_PX, ELBOW_RADIUS, dir);
                assert_eq!(ends(&path[0]).0, from, "{dir:?}");
                assert_eq!(ends(path.last().unwrap()).1, to, "{dir:?}");
                for w in path.windows(2) {
                    let gap = ends(&w[1]).0 - ends(&w[0]).1;
                    assert!(gap.length() < 1e-4, "{dir:?}: broken path");
                }
                let mut corners = 0;
                for s in &path {
                    match *s {
                        Segment::Line([a, b]) => {
                            let d = b - a;
                            assert!(d.x.abs() < 1e-4 || d.y.abs() < 1e-4, "{dir:?}: {d:?}");
                        }
                        Segment::Cubic([a, b, c, d]) => {
                            corners += 1;
                            // A quarter circle: handles 0.5523 r long.
                            let r = (d - a).length() / 2f32.sqrt();
                            assert!((r - ELBOW_RADIUS).abs() < 1e-3, "{dir:?}: {r}");
                            let k = QUARTER_K * r;
                            assert!(((b - a).length() - k).abs() < 1e-3);
                            assert!(((d - c).length() - k).abs() < 1e-3);
                        }
                    }
                }
                assert_eq!(corners, 2, "{dir:?}");
                let Some(&Segment::Line([a, b])) = path.last() else {
                    panic!("{dir:?}: no stub");
                };
                assert!(((b - a) - dir.toward_parent() * STUB_PX).length() < 1e-4);
            }
        }
    }

    #[test]
    fn elbow_straight_when_aligned() {
        let to = Pos2::new(100.0, 50.0);
        for dir in LayoutDirection::ALL {
            let from = to - dir.toward_parent() * 150.0 + dir.toward_parent().rot90() * 0.3;
            let path = elbow_path(from, to, STUB_PX, ELBOW_RADIUS, dir);
            assert_eq!(path, vec![Segment::Line([from, to])], "{dir:?}");
        }
    }

    #[test]
    fn elbow_corners_shrink_to_half_the_shorter_segment() {
        let to = Pos2::new(0.0, 0.0);
        // 6 across: the corners are 3, not 8.
        let path = elbow_path(
            Pos2::new(6.0, 100.0),
            to,
            STUB_PX,
            ELBOW_RADIUS,
            LayoutDirection::TopDown,
        );
        for s in &path {
            if let Segment::Cubic([a, _, _, d]) = *s {
                assert!(((d - a).length() / 2f32.sqrt() - 3.0).abs() < 1e-3);
            }
        }
        // A child above its parent still draws a connected path.
        let path = elbow_path(
            Pos2::new(40.0, -30.0),
            to,
            STUB_PX,
            ELBOW_RADIUS,
            LayoutDirection::TopDown,
        );
        assert_eq!(ends(&path[0]).0, Pos2::new(40.0, -30.0));
        assert_eq!(ends(path.last().unwrap()).1, to);
    }
}
