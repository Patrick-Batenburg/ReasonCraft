//! Main and cross axes.
//!
//! The tree is packed in axis terms: the main axis runs from a parent to its
//! children and the cross axis runs along a row of siblings. A [`Frame`]
//! maps those coordinates to world space; it is the only place that knows
//! which world axis is which. Today it maps top-down only (cross = x,
//! main = y).

use emath::{Pos2, Rect, Vec2};

/// The size of a box along the axes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Ext {
    pub cross: f32,
    pub main: f32,
}

/// Maps `(cross, main)` coordinates to world coordinates and back.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Frame {}

impl Frame {
    /// The extent of a world-space `size` along the axes.
    pub(crate) fn ext(self, size: Vec2) -> Ext {
        Ext {
            cross: size.x,
            main: size.y,
        }
    }

    /// The world rect of extent `ext` whose low corner along both axes is at
    /// `(cross, main)`.
    pub(crate) fn to_world(self, cross: f32, main: f32, ext: Ext) -> Rect {
        Rect::from_min_size(Pos2::new(cross, main), Vec2::new(ext.cross, ext.main))
    }

    /// The world point at `(cross, main)`.
    pub(crate) fn point(self, cross: f32, main: f32) -> Pos2 {
        Pos2::new(cross, main)
    }

    /// The cross coordinate of a world point.
    pub(crate) fn cross_of(self, p: Pos2) -> f32 {
        p.x
    }

    /// The `(low, high)` cross coordinates of a world rect.
    pub(crate) fn cross_range(self, r: Rect) -> (f32, f32) {
        (r.left(), r.right())
    }

    /// The `(low, high)` main coordinates of a world rect.
    pub(crate) fn main_range(self, r: Rect) -> (f32, f32) {
        (r.top(), r.bottom())
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
}
