//! The window's sizes: the smallest one, and the first. On the
//! first run the window opens at [`FIRST_SIZE`], centred (see `main.rs`),
//! and is maximised when that leaves too little room around it (a
//! 1366 × 768 laptop, or 1920 × 1080 at 150 %), so the status bar never
//! ends up under the taskbar. Later runs get the place and size the
//! window had (eframe keeps them).

use eframe::egui;

use super::ReasonCraftApp;

/// The smallest window at 100 % interface size, in screen points.
pub const MIN_WINDOW: [f32; 2] = [800.0, 500.0];

/// The smallest window, in the interface's own points: [`MIN_WINDOW`], so
/// a larger interface keeps the room it is laid out for, but never more
/// than 90 % of the `monitor` (when known). egui-winit turns these points
/// into pixels with the interface size (egui's zoom factor times the
/// display scale) and gives the monitor's size in the same points, so no
/// scale is applied here.
pub fn min_window_size(monitor: Option<egui::Vec2>) -> egui::Vec2 {
    let base = egui::Vec2::from(MIN_WINDOW);
    monitor.map_or(base, |m| base.min(m * 0.9))
}

/// The window's inner size on the first run, in points.
pub const FIRST_SIZE: [f32; 2] = [1280.0, 800.0];

/// How much of the monitor the window may take before it is maximised
/// instead: what is left is for the taskbar or dock and some margin.
const ROOM: f32 = 0.9;

/// Whether a window of `window` points leaves enough of a `monitor`'s
/// room around it.
pub fn fits_on(window: egui::Vec2, monitor: egui::Vec2) -> bool {
    window.x <= monitor.x * ROOM && window.y <= monitor.y * ROOM
}

impl ReasonCraftApp {
    /// The whole interface at the size the settings ask for (egui's zoom
    /// factor). The map's zoom works in points, so it comes on top. The
    /// window's minimum size, in these points, grows with it on screen
    /// (see [`min_window_size`]), once the screen's size is known.
    pub(super) fn sync_ui_scale(&self, ctx: &egui::Context) {
        let scale = self.settings.ui_scale_factor();
        if ctx.zoom_factor() != scale {
            ctx.set_zoom_factor(scale);
        }
        let Some(monitor) = ctx.input(|i| i.viewport().monitor_size) else {
            return;
        };
        // Sent again once the interface size changes (egui applies a new
        // zoom factor from the next pass on, and egui-winit turns the
        // points into pixels with the zoom factor in force).
        let min = (min_window_size(Some(monitor)), ctx.zoom_factor());
        let sent = egui::Id::unique("min_window_sent");
        if ctx.data(|d| d.get_temp::<(egui::Vec2, f32)>(sent)) != Some(min) {
            ctx.data_mut(|d| d.insert_temp(sent, min));
            ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(min.0));
        }
    }

    /// On the first run, maximise the window once the monitor's size is
    /// known, if the window does not fit on it.
    pub(super) fn fit_first_window(&mut self, ctx: &egui::Context) {
        if !self.fit_window {
            return;
        }
        let (monitor, inner) = ctx.input(|i| (i.viewport().monitor_size, i.viewport().inner_rect));
        let (Some(monitor), Some(inner)) = (monitor, inner) else {
            return;
        };
        self.fit_window = false;
        if !fits_on(inner.size(), monitor) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_size_fits_a_common_screen_but_not_a_small_one() {
        let first = egui::Vec2::from(FIRST_SIZE);
        assert!(fits_on(first, egui::vec2(1920.0, 1080.0)));
        assert!(fits_on(first, egui::vec2(1600.0, 900.0)));
        // 1366 × 768 at 100 %; 1920 × 1080 at 150 % (eframe has already
        // made the window no larger than the monitor).
        assert!(!fits_on(
            egui::vec2(1280.0, 768.0),
            egui::vec2(1366.0, 768.0)
        ));
        assert!(!fits_on(
            egui::vec2(1280.0, 720.0),
            egui::vec2(1280.0, 720.0)
        ));
    }

    /// Whether the first frames on a `monitor` maximise a window of the
    /// first size (clamped to the monitor, as eframe does).
    fn maximises_on(monitor: egui::Vec2, first_run: bool) -> bool {
        let mut app = ReasonCraftApp::new_for_test();
        app.fit_window = first_run;
        let size = egui::Vec2::from(FIRST_SIZE).min(monitor);
        let mut h = egui_kittest::Harness::builder()
            .with_size(size)
            .build_ui_state(|ui, app: &mut ReasonCraftApp| app.show(ui), app);
        let viewport = h
            .input_mut()
            .viewports
            .entry(egui::ViewportId::ROOT)
            .or_default();
        viewport.monitor_size = Some(monitor);
        viewport.inner_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size));
        let mut maximised = false;
        for _ in 0..3 {
            h.step();
            let commands = &h.output().viewport_output[&egui::ViewportId::ROOT].commands;
            maximised |= commands.contains(&egui::ViewportCommand::Maximized(true));
        }
        maximised
    }

    #[test]
    fn a_first_window_too_large_for_the_monitor_is_maximised() {
        assert!(maximises_on(egui::vec2(1366.0, 768.0), true));
        assert!(!maximises_on(egui::vec2(1920.0, 1080.0), true));
        // Later runs keep the size the user gave the window.
        assert!(!maximises_on(egui::vec2(1366.0, 768.0), false));
    }
}
