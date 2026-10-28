//! Pan and zoom state of a map view.

use eframe::egui::{self, Pos2, Rect, Vec2};

/// Low enough to fit maps of a hundred-odd boxes on one screen. Strokes,
/// arrowheads and grid dots keep their own screen-px minimums.
pub const ZOOM_MIN: f32 = 0.03;
pub const ZOOM_MAX: f32 = 4.0;
/// Free space kept around the map on each side when fitting, as a fraction
/// of the viewport size.
const FIT_MARGIN: f32 = 0.1;
/// Screen px kept between a revealed item and the edge of the view.
pub const REVEAL_MARGIN: f32 = 24.0;
/// The most Fit selection zooms in.
pub const FIT_SELECTION_MAX: f32 = 2.0;

/// Where the map view looks. A world point `p` is drawn at
/// `p * zoom + offset` in screen points (1.0 zoom = 100%).
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub offset: egui::Vec2,
    pub zoom: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            offset: egui::Vec2::ZERO,
            zoom: 1.0,
        }
    }
}

impl Camera {
    pub fn world_to_screen(&self, p: Pos2) -> Pos2 {
        (p.to_vec2() * self.zoom + self.offset).to_pos2()
    }

    pub fn screen_to_world(&self, p: Pos2) -> Pos2 {
        ((p.to_vec2() - self.offset) / self.zoom).to_pos2()
    }

    pub fn world_rect_to_screen(&self, r: Rect) -> Rect {
        Rect::from_min_max(self.world_to_screen(r.min), self.world_to_screen(r.max))
    }

    /// Multiply the zoom by `factor` (clamped to the zoom range), keeping the
    /// world point under `screen_pt` where it is.
    pub fn zoom_at(&mut self, screen_pt: Pos2, factor: f32) {
        self.zoom_to(screen_pt, self.zoom * factor);
    }

    /// The view went from `old` to `new` (a window or panel resize): keep
    /// the world point in the middle of the view in the middle, so the
    /// map does not slide out of a shrinking window.
    pub fn follow_resize(&mut self, old: Rect, new: Rect) {
        self.offset += new.center() - old.center();
    }

    /// Set the zoom to exactly `zoom` (clamped), keeping the world point
    /// under `screen_pt` in place.
    pub fn zoom_to(&mut self, screen_pt: Pos2, zoom: f32) {
        let world = self.screen_to_world(screen_pt);
        self.zoom = zoom.clamp(ZOOM_MIN, ZOOM_MAX);
        self.offset = screen_pt.to_vec2() - world.to_vec2() * self.zoom;
    }

    /// Show all of `bounds` centred in `viewport` with a 10% margin on each
    /// side. Small maps are shown at 100% rather than enlarged.
    pub fn fit(&mut self, bounds: Rect, viewport: Rect) {
        let room = viewport.size() * (1.0 - 2.0 * FIT_MARGIN);
        let size = bounds.size().max(Vec2::splat(1.0));
        let zoom = (room.x / size.x).min(room.y / size.y);
        self.zoom = zoom.clamp(ZOOM_MIN, 1.0);
        self.offset = viewport.center().to_vec2() - bounds.center().to_vec2() * self.zoom;
    }

    /// Show all of world rect `item` centred in `viewport`, with
    /// [`REVEAL_MARGIN`] screen px around it, zooming in up to
    /// [`FIT_SELECTION_MAX`].
    pub fn fit_selection(&mut self, item: Rect, viewport: Rect) {
        let room = (viewport.size() - Vec2::splat(2.0 * REVEAL_MARGIN)).max(Vec2::splat(1.0));
        let size = item.size().max(Vec2::splat(1.0));
        let zoom = (room.x / size.x).min(room.y / size.y);
        self.zoom = zoom.clamp(ZOOM_MIN, FIT_SELECTION_MAX);
        self.offset = viewport.center().to_vec2() - item.center().to_vec2() * self.zoom;
    }

    /// Zoom to `zoom` (clamped to the zoom range) with the centre of world
    /// rect `item` in the middle of `viewport`.
    pub fn center_on(&mut self, item: Rect, viewport: Rect, zoom: f32) {
        self.zoom = zoom.clamp(ZOOM_MIN, ZOOM_MAX);
        self.offset = viewport.center().to_vec2() - item.center().to_vec2() * self.zoom;
    }

    /// Pan by the smallest amount that brings world rect `item`, with
    /// `margin` screen px around it, into `viewport`. The zoom is kept. An
    /// item too big for the view is aligned to its top-left.
    pub fn reveal(&mut self, item: Rect, viewport: Rect, margin: f32) {
        let s = self.world_rect_to_screen(item).expand(margin);
        let shift = |lo: f32, hi: f32, view_lo: f32, view_hi: f32| {
            if lo < view_lo || hi - lo > view_hi - view_lo {
                view_lo - lo
            } else if hi > view_hi {
                view_hi - hi
            } else {
                0.0
            }
        };
        self.offset += Vec2::new(
            shift(s.left(), s.right(), viewport.left(), viewport.right()),
            shift(s.top(), s.bottom(), viewport.top(), viewport.bottom()),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Rect, pos2, vec2};

    #[test]
    fn zoom_at_keeps_point_fixed() {
        let mut cam = Camera {
            offset: vec2(40.0, -25.0),
            zoom: 0.8,
        };
        let cursor = pos2(310.0, 220.0);
        let world = cam.screen_to_world(cursor);
        cam.zoom_at(cursor, 1.7);
        assert!((cam.zoom - 1.36).abs() < 1e-5);
        assert!((cam.world_to_screen(world) - cursor).length() < 1e-3);
        // Clamped at the limits, still around the same point.
        cam.zoom_at(cursor, 1000.0);
        assert_eq!(cam.zoom, ZOOM_MAX);
        assert!((cam.world_to_screen(world) - cursor).length() < 1e-3);
        cam.zoom_at(cursor, 1e-6);
        assert_eq!(cam.zoom, ZOOM_MIN);
    }

    #[test]
    fn fit_contains_bounds() {
        let viewport = Rect::from_min_size(pos2(200.0, 120.0), vec2(900.0, 600.0));
        for bounds in [
            Rect::from_min_size(pos2(0.0, 0.0), vec2(3000.0, 800.0)),
            Rect::from_min_size(pos2(-500.0, 40.0), vec2(400.0, 2600.0)),
        ] {
            let mut cam = Camera::default();
            cam.fit(bounds, viewport);
            let on_screen = Rect::from_min_max(
                cam.world_to_screen(bounds.min),
                cam.world_to_screen(bounds.max),
            );
            assert!(
                viewport
                    .shrink2(viewport.size() * 0.1 - vec2(0.01, 0.01))
                    .contains_rect(on_screen)
            );
            assert!((on_screen.center() - viewport.center()).length() < 1e-3);
            // Tight along one axis: the 10% margin is used, not more.
            let used = on_screen.size() / (viewport.size() * 0.8);
            assert!((used.max_elem() - 1.0).abs() < 1e-3);
        }
    }

    #[test]
    fn fit_never_enlarges_past_100_percent() {
        let viewport = Rect::from_min_size(pos2(0.0, 0.0), vec2(900.0, 600.0));
        let mut cam = Camera::default();
        cam.fit(
            Rect::from_min_size(pos2(0.0, 0.0), vec2(156.0, 40.0)),
            viewport,
        );
        assert_eq!(cam.zoom, 1.0);
        assert!((cam.world_to_screen(pos2(78.0, 20.0)) - viewport.center()).length() < 1e-3);
    }

    #[test]
    fn reveal_pans_the_least_and_keeps_zoom() {
        let viewport = Rect::from_min_size(pos2(100.0, 50.0), vec2(800.0, 600.0));
        let start = Camera {
            offset: vec2(100.0, 50.0),
            zoom: 0.5,
        };
        let shown = |cam: &Camera, r: Rect| cam.world_rect_to_screen(r).expand(REVEAL_MARGIN);
        // Already in view: nothing moves.
        let mut cam = start;
        let inside = Rect::from_min_size(pos2(200.0, 200.0), vec2(156.0, 40.0));
        cam.reveal(inside, viewport, REVEAL_MARGIN);
        assert_eq!(cam.offset, start.offset);
        // Out of view: the smallest pan puts it flush with the margin on
        // the side it was beyond; an axis already in view does not move.
        for (item, pan) in [
            (pos2(3000.0, 2000.0), vec2(-802.0, -444.0)),
            (pos2(-900.0, -400.0), vec2(474.0, 224.0)),
            (pos2(1500.0, 300.0), vec2(-52.0, 0.0)),
        ] {
            let item = Rect::from_min_size(item, vec2(156.0, 40.0));
            let mut cam = start;
            cam.reveal(item, viewport, REVEAL_MARGIN);
            assert_eq!(cam.zoom, start.zoom);
            assert!(((cam.offset - start.offset) - pan).length() < 1e-3);
            assert!(viewport.expand(1e-3).contains_rect(shown(&cam, item)));
        }
        // Too big for the view: its top-left corner is shown.
        let mut cam = start;
        let huge = Rect::from_min_size(pos2(0.0, 0.0), vec2(5000.0, 5000.0));
        cam.reveal(huge, viewport, REVEAL_MARGIN);
        assert!((shown(&cam, huge).min - viewport.min).length() < 1e-3);
    }

    #[test]
    fn fit_shows_the_whole_sample_map() {
        use reasoncraft_core::fixtures::{FixedMeasure, sample_iot_like};
        use reasoncraft_core::{LayoutParams, layout};
        let doc = sample_iot_like(117);
        let l = layout(&doc, &FixedMeasure, &LayoutParams::default());
        let viewport = Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 600.0));
        let mut cam = Camera::default();
        cam.fit(l.bounds, viewport);
        assert!(cam.zoom > ZOOM_MIN, "fit is not clamped: {}", cam.zoom);
        assert!(viewport.contains_rect(cam.world_rect_to_screen(l.bounds)));
    }

    #[test]
    fn fit_selection_zooms_to_item() {
        let viewport = Rect::from_min_size(pos2(100.0, 50.0), vec2(1000.0, 600.0));
        // A small box is enlarged, but not past 200%.
        let mut cam = Camera::default();
        let small = Rect::from_min_size(pos2(500.0, 300.0), vec2(156.0, 40.0));
        cam.fit_selection(small, viewport);
        assert_eq!(cam.zoom, FIT_SELECTION_MAX);
        assert!((cam.world_to_screen(small.center()) - viewport.center()).length() < 1e-3);
        // A big subtree fits, tight along one axis with the 24 px margin.
        let big = Rect::from_min_size(pos2(-800.0, 40.0), vec2(4000.0, 900.0));
        cam.fit_selection(big, viewport);
        let shown = cam.world_rect_to_screen(big);
        assert!(viewport.shrink(REVEAL_MARGIN - 0.01).contains_rect(shown));
        assert!((shown.width() - (1000.0 - 2.0 * REVEAL_MARGIN)).abs() < 1e-2);
        assert!((shown.center() - viewport.center()).length() < 1e-3);
    }

    #[test]
    fn resize_keeps_the_middle_in_the_middle() {
        let mut cam = Camera {
            offset: vec2(30.0, -20.0),
            zoom: 0.5,
        };
        let old = Rect::from_min_size(pos2(220.0, 120.0), vec2(900.0, 600.0));
        let new = Rect::from_min_size(pos2(120.0, 120.0), vec2(300.0, 320.0));
        let middle = cam.screen_to_world(old.center());
        cam.follow_resize(old, new);
        assert!((cam.world_to_screen(middle) - new.center()).length() < 1e-3);
        assert_eq!(cam.zoom, 0.5);
    }

    #[test]
    fn zoom_min_is_three_percent() {
        assert_eq!(ZOOM_MIN, 0.03);
    }
}
