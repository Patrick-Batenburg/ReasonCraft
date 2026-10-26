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
        Box::new(|cc| {
            let recovery_dir = reasoncraft::recovery::RecoveryStore::default_dir();
            let mut app = ReasonCraftApp::new(cc, recovery_dir);
            // Files given on the command line (this is also how a
            // double-clicked `.rcraft` file arrives on Windows and Linux).
            reasoncraft::file_ops::open_paths(&mut app, std::env::args_os().skip(1));
            Ok(Box::new(app))
        }),
    )
}
