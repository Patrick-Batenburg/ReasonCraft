//! The Tapered style: a filled band along the curved path that widens
//! toward the parent; the scene carries it on at that width along the stub.

use emath::{Pos2, Vec2};

use super::curved::s_curve;
use super::{MIN_VISIBLE_PX, Strength, line_width};
use crate::layout::ConnectorKind;
use crate::model::LayoutDirection;

/// Points along each edge of a band: enough that its tight bends near the
/// ends stay smooth.
const SAMPLES: usize = 48;
/// How close the inner edge may come to a bend's centre, as a share of its
/// radius: an edge offset past the centre would fold back on itself.
const INNER_REACH: f32 = 0.9;
/// The width of a band at its parent end: this many times its child end,
/// and at least [`MIN_PARENT_PX`] screen px.
const PARENT_FACTOR: f32 = 3.0;
const MIN_PARENT_PX: f32 = 6.0;

/// The widths of a tapered connector at its child end (also its stub's
/// stroke) and at its parent end, in world units at `zoom`. Neither is
/// ever thinner than [`MIN_VISIBLE_PX`] on screen.
pub fn tapered_widths(kind: ConnectorKind, strength: Strength, zoom: f32) -> (f32, f32) {
    let child = line_width(kind, strength, zoom).max(MIN_VISIBLE_PX / zoom);
    (child, (PARENT_FACTOR * child).max(MIN_PARENT_PX / zoom))
}

/// The two edges of the band from the child's near edge `from` to the stub
/// start `stub` before the parent port `to`: its centre line follows
/// [`curved_path`](super::curved_path), and it widens from `w_child` to
/// `w_parent`. On a bend tighter than half the band, the inner edge keeps
/// short of the bend's centre, so the band never folds over itself. The
/// edges have the same number of points (at least 16); `left[i]` and
/// `right[i]` lie across the band from each other.
pub fn tapered_band(
    from: Pos2,
    to: Pos2,
    stub: f32,
    w_child: f32,
    w_parent: f32,
    dir: LayoutDirection,
) -> (Vec<Pos2>, Vec<Pos2>) {
    let toward = dir.toward_parent();
    let [p0, p1, p2, p3] = s_curve(from, to - toward * stub, dir);
    let mut left = Vec::with_capacity(SAMPLES);
    let mut right = Vec::with_capacity(SAMPLES);
    let [q0, q1, q2, q3] = [p0, p1, p2, p3].map(|p| p.to_vec2());
    for i in 0..SAMPLES {
        let t = i as f32 / (SAMPLES - 1) as f32;
        let u = 1.0 - t;
        let at =
            (q0 * (u * u * u) + q1 * (3.0 * u * u * t) + q2 * (3.0 * u * t * t) + q3 * (t * t * t))
                .to_pos2();
        let speed =
            (q1 - q0) * (3.0 * u * u) + (q2 - q1) * (6.0 * u * t) + (q3 - q2) * (3.0 * t * t);
        let accel = (q2 - q1 * 2.0 + q0) * (6.0 * u) + (q3 - q2 * 2.0 + q1) * (6.0 * t);
        // Where the curve has no direction (it has no length), use the axis.
        let tangent = if speed.length() > 1e-6 {
            speed.normalized()
        } else {
            toward
        };
        let side: Vec2 = tangent.rot90();
        let half = (w_child + (w_parent - w_child) * smoothstep(t)) / 2.0;
        // On a tight bend the inner edge stays short of the bend's centre.
        let (mut l_half, mut r_half) = (half, half);
        let bend = speed.x * accel.y - speed.y * accel.x;
        if bend.abs() > 1e-6 {
            let inner = (INNER_REACH * speed.length().powi(3) / bend.abs()).min(half);
            if accel.dot(side) > 0.0 {
                l_half = inner;
            } else {
                r_half = inner;
            }
        }
        left.push(at + side * l_half);
        right.push(at - side * r_half);
    }
    (left, right)
}

/// 0 at 0, 1 at 1, flat at both ends and never decreasing.
fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::super::STUB_PX;
    use super::*;

    fn widths(left: &[Pos2], right: &[Pos2]) -> Vec<f32> {
        left.iter()
            .zip(right)
            .map(|(l, r)| (*l - *r).length())
            .collect()
    }

    #[test]
    fn tapered_band_never_thinner_than_floor() {
        let to = Pos2::new(100.0, 50.0);
        for zoom in [0.05, 4.0] {
            for kind in [ConnectorKind::Attachment, ConnectorKind::Supports] {
                let (wc, wp) = tapered_widths(kind, Strength::Weak, zoom);
                for dir in LayoutDirection::ALL {
                    let from =
                        to - dir.toward_parent() * 150.0 + dir.toward_parent().rot90() * 60.0;
                    let (l, r) = tapered_band(from, to, STUB_PX / zoom, wc, wp, dir);
                    assert!(l.len() >= 16 && l.len() == r.len(), "{dir:?}");
                    let min = widths(&l, &r).into_iter().fold(f32::MAX, f32::min);
                    assert!(
                        min >= MIN_VISIBLE_PX / zoom - 1e-3,
                        "{zoom} {kind:?} {dir:?}: {min}"
                    );
                }
            }
        }
    }

    #[test]
    fn tapered_widens_toward_parent() {
        let to = Pos2::new(100.0, 50.0);
        let (wc, wp) = tapered_widths(ConnectorKind::Supports, Strength::Normal, 1.0);
        assert_eq!((wc, wp), (2.0, 6.0));
        for dir in LayoutDirection::ALL {
            let from = to - dir.toward_parent() * 150.0 + dir.toward_parent().rot90() * 60.0;
            let (l, r) = tapered_band(from, to, STUB_PX, wc, wp, dir);
            let w = widths(&l, &r);
            for pair in w.windows(2) {
                assert!(pair[1] >= pair[0] - 1e-4, "{dir:?}: {w:?}");
            }
            assert!((w[0] - wc).abs() < 1e-4 && (w[w.len() - 1] - wp).abs() < 1e-3);
            // The band's centre ends at the stub start, not at the port.
            let last = l.len() - 1;
            let centre = l[last] + (r[last] - l[last]) / 2.0;
            let stub_start = to - dir.toward_parent() * STUB_PX;
            assert!((centre - stub_start).length() < 1e-3, "{dir:?}");
            assert!(((l[0] + (r[0] - l[0]) / 2.0) - from).length() < 1e-3);
        }
    }

    /// A wide, shallow connector bends tightly near both ends; neither edge
    /// may run backwards there (a fold shows as a notch in the band).
    #[test]
    fn tapered_band_never_folds() {
        let to = Pos2::new(100.0, 50.0);
        let (wc, wp) = tapered_widths(ConnectorKind::Supports, Strength::Strong, 1.0);
        for dir in LayoutDirection::ALL {
            let toward = dir.toward_parent();
            for lateral in [-600.0, 600.0] {
                let from = to - toward * 40.0 + toward.rot90() * lateral;
                let (l, r) = tapered_band(from, to, STUB_PX, wc, wp, dir);
                for edge in [&l, &r] {
                    for w in edge.windows(3) {
                        let (a, b) = (w[1] - w[0], w[2] - w[1]);
                        assert!(
                            a.dot(b) >= 0.0,
                            "{dir:?} {lateral}: edge turns back at {:?}",
                            w[1]
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn tapered_widths_have_floors() {
        let (wc, wp) = tapered_widths(ConnectorKind::Attachment, Strength::Normal, 1.0);
        assert_eq!((wc, wp), (MIN_VISIBLE_PX, MIN_PARENT_PX));
    }
}
