//! ReasonCraft desktop app entry point: window setup with a custom title bar.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use reasoncraft::ReasonCraftApp;

const ICON_PNG: &[u8] = include_bytes!("../assets/icon.png");

fn main() -> eframe::Result {
    let mut viewport = egui::ViewportBuilder::default()
        .with_title(reasoncraft::i18n::t("app.name"))
        .with_app_id("nl.reasoncraft.app")
        .with_inner_size([1280.0, 800.0])
        .with_min_inner_size([800.0, 500.0]);
    viewport = if cfg!(target_os = "macos") {
        // Keep the native traffic lights; our title bar draws under them.
        viewport
            .with_fullsize_content_view(true)
            .with_titlebar_shown(false)
            .with_title_shown(false)
    } else {
        viewport.with_decorations(false)
    };
    match eframe::icon_data::from_png_bytes(ICON_PNG) {
        Ok(icon) => viewport = viewport.with_icon(icon),
        Err(err) => eprintln!("warning: app icon not loaded: {err}"),
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "ReasonCraft",
        options,
        Box::new(|cc| Ok(Box::new(ReasonCraftApp::new(cc)))),
    )
}
