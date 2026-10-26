//! Main and cross axes.
//!
//! The tree is packed in axis terms: the main axis runs from a parent to its
//! children and the cross axis runs along a row of siblings. A [`Frame`]
//! maps those coordinates to world space; it is the only place that knows
//! which world axis is which:
//!
//! - top-down: cross = x, main = y;
//! - bottom-up: cross = x, main = −y (children above their parent);
//! - left-to-right: cross = y, main = x.

use emath::{Pos2, Rect, Vec2};

use crate::model::LayoutDirection;

/// The size of a box along the axes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Ext {
    pub cross: f32,
    pub main: f32,
}

/// Maps `(cross, main)` coordinates to world coordinates and back.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Frame {
    pub dir: LayoutDirection,
}

impl Frame {
    pub(crate) fn new(dir: LayoutDirection) -> Self {
        Self { dir }
    }

    /// Whether the cross axis is world y.
    pub(crate) fn cross_is_y(self) -> bool {
        self.dir == LayoutDirection::LeftRight
    }

    /// The extent of a world-space `size` along the axes.
    pub(crate) fn ext(self, size: Vec2) -> Ext {
        if self.cross_is_y() {
            Ext {
                cross: size.y,
                main: size.x,
            }
        } else {
            Ext {
                cross: size.x,
                main: size.y,
            }
        }
    }

    /// The world rect of extent `ext` whose low corner along both axes is at
    /// `(cross, main)`.
    pub(crate) fn to_world(self, cross: f32, main: f32, ext: Ext) -> Rect {
        Rect::from_two_pos(
            self.point(cross, main),
            self.point(cross + ext.cross, main + ext.main),
        )
    }

    /// The world point at `(cross, main)`.
    pub(crate) fn point(self, cross: f32, main: f32) -> Pos2 {
        match self.dir {
            LayoutDirection::TopDown => Pos2::new(cross, main),
            LayoutDirection::BottomUp => Pos2::new(cross, -main),
            LayoutDirection::LeftRight => Pos2::new(main, cross),
        }
    }

    /// The cross coordinate of a world point.
    pub(crate) fn cross_of(self, p: Pos2) -> f32 {
        if self.cross_is_y() { p.y } else { p.x }
    }

    /// The `(low, high)` cross coordinates of a world rect.
    pub(crate) fn cross_range(self, r: Rect) -> (f32, f32) {
        if self.cross_is_y() {
            (r.top(), r.bottom())
        } else {
            (r.left(), r.right())
        }
    }

    /// The `(low, high)` main coordinates of a world rect: its near edge
    /// (toward the parent) and its far edge.
    pub(crate) fn main_range(self, r: Rect) -> (f32, f32) {
        match self.dir {
            LayoutDirection::TopDown => (r.top(), r.bottom()),
            LayoutDirection::BottomUp => (-r.bottom(), -r.top()),
            LayoutDirection::LeftRight => (r.left(), r.right()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_down_round_trips() {
        let f = Frame::default();
        let ext = f.ext(Vec2::new(30.0, 20.0));
        assert_eq!(
            ext,
            Ext {
                cross: 30.0,
                main: 20.0
            }
        );
        let r = f.to_world(5.0, 7.0, ext);
        assert_eq!(
            r,
            Rect::from_min_size(Pos2::new(5.0, 7.0), Vec2::new(30.0, 20.0))
        );
        assert_eq!(f.cross_range(r), (5.0, 35.0));
        assert_eq!(f.main_range(r), (7.0, 27.0));
        assert_eq!(f.cross_of(f.point(3.0, 4.0)), 3.0);
    }

    #[test]
    fn every_direction_round_trips() {
        let size = Vec2::new(30.0, 20.0);
        for dir in LayoutDirection::ALL {
            let f = Frame::new(dir);
            let ext = f.ext(size);
            let r = f.to_world(5.0, 7.0, ext);
            assert_eq!(r.size(), size, "{dir:?}");
            assert_eq!(f.cross_range(r), (5.0, 5.0 + ext.cross), "{dir:?}");
            assert_eq!(f.main_range(r), (7.0, 7.0 + ext.main), "{dir:?}");
            assert_eq!(f.cross_of(f.point(3.0, 4.0)), 3.0, "{dir:?}");
            // Growing along main moves away from the parent.
            let step = f.point(0.0, 1.0) - f.point(0.0, 0.0);
            assert_eq!(step, -dir.toward_parent(), "{dir:?}");
        }
        let r = Frame::new(LayoutDirection::BottomUp).to_world(
            5.0,
            7.0,
            Ext {
                cross: 30.0,
                main: 20.0,
            },
        );
        assert_eq!(
            r,
            Rect::from_min_max(Pos2::new(5.0, -27.0), Pos2::new(35.0, -7.0))
        );
    }
}
