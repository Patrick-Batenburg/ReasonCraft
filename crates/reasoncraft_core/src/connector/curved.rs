//! The Curved style: an S-curve along the layout's main axis.

use emath::Pos2;

use super::Segment;
use crate::model::LayoutDirection;

/// Path from the child's near edge `from` to the parent port `to`: an
/// S-curve that ends `stub` px beyond `to` (away from the parent), followed
/// by a straight stub along `dir.toward_parent()` to `to`.
pub fn curved_path(from: Pos2, to: Pos2, stub: f32, dir: LayoutDirection) -> Vec<Segment> {
    let curve_end = to - dir.toward_parent() * stub;
    vec![
        Segment::Cubic(s_curve(from, curve_end, dir)),
        Segment::Line([curve_end, to]),
    ]
}

/// The control points of the S-curve from `from` to `end`: it leaves and
/// arrives along the main axis, turning halfway.
pub(super) fn s_curve(from: Pos2, end: Pos2, dir: LayoutDirection) -> [Pos2; 4] {
    let (c1, c2) = if dir == LayoutDirection::LeftRight {
        let mid = (from.x + end.x) / 2.0;
        (Pos2::new(mid, from.y), Pos2::new(mid, end.y))
    } else {
        let mid = (from.y + end.y) / 2.0;
        (Pos2::new(from.x, mid), Pos2::new(end.x, mid))
    };
    [from, c1, c2, end]
}

#[cfg(test)]
mod tests {
    use super::super::STUB_PX;
    use super::*;

    #[test]
    fn curved_path_ends_with_vertical_stub() {
        let to = Pos2::new(100.0, 50.0);
        let path = curved_path(
            Pos2::new(40.0, 200.0),
            to,
            STUB_PX,
            LayoutDirection::TopDown,
        );
        assert_eq!(
            path.last(),
            Some(&Segment::Line([Pos2::new(100.0, 66.0), to]))
        );
    }

    #[test]
    fn path_valid_when_child_not_below_parent() {
        let path = curved_path(
            Pos2::new(0.0, 10.0),
            Pos2::new(5.0, 20.0),
            STUB_PX,
            LayoutDirection::TopDown,
        );
        assert_eq!(path.len(), 2);
    }

    #[test]
    fn curved_path_ends_with_stub_along_direction() {
        let to = Pos2::new(100.0, 50.0);
        for dir in LayoutDirection::ALL {
            let from = to - dir.toward_parent() * 150.0 + emath::vec2(30.0, 30.0);
            let path = curved_path(from, to, STUB_PX, dir);
            let Some(&Segment::Line([a, b])) = path.last() else {
                panic!("{dir:?}: the path ends in a curve");
            };
            assert_eq!(b, to, "{dir:?}");
            let along = (b - a).normalized();
            assert!(
                (along - dir.toward_parent()).length() < 1e-5,
                "{dir:?}: {along:?}"
            );
            assert!(((b - a).length() - STUB_PX).abs() < 1e-4, "{dir:?}");
            let Segment::Cubic(c) = path[0] else {
                panic!("{dir:?}: the path starts with a line");
            };
            assert_eq!((c[0], c[3]), (from, a), "{dir:?}");
        }
    }
}
