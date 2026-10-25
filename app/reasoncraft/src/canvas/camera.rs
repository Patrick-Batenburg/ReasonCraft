//! Pan and zoom state of a map view.

use eframe::egui::{self, Pos2, Rect, Vec2};

pub const ZOOM_MIN: f32 = 0.1;
pub const ZOOM_MAX: f32 = 4.0;
/// Free space kept around the map on each side when fitting, as a fraction
/// of the viewport size.
const FIT_MARGIN: f32 = 0.1;

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
        let world = self.screen_to_world(screen_pt);
        self.zoom = (self.zoom * factor).clamp(ZOOM_MIN, ZOOM_MAX);
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
}
