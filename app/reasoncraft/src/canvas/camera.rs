//! Pan and zoom state of a map view.

use eframe::egui;

/// Where the map view looks: a screen offset and a zoom factor (1.0 = 100%).
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
