//! ReasonCraft desktop app entry point: window setup with a custom title
//! bar, trying the renderers in turn (see `reasoncraft::startup`).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::ExitCode;

use eframe::egui;
use reasoncraft::{ReasonCraftApp, startup};

const ICON_PNG: &[u8] = include_bytes!("../assets/icon.png");

fn main() -> ExitCode {
    // Before anything can load a DLL: none from the current folder.
    if !reasoncraft::dll_search::system32_only() {
        eprintln!("warning: could not restrict the DLL search to System32");
    }
    // Before any window or profile: packaging smoke tests run this.
    let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    if reasoncraft::cli::wants_version(&args) {
        reasoncraft::cli::print_version();
        return ExitCode::SUCCESS;
    }
    let profile = reasoncraft::profile::Profile::from_env();
    let outcome = startup::first_that_starts(&startup::RENDERERS, |renderer, started| {
        eframe::run_native(
            "ReasonCraft",
            native_options(renderer, profile.settings_file.clone()),
            Box::new(|cc| {
                started.set(true);
                let recovery_dir = profile.recovery_dir.clone();
                let mut app = ReasonCraftApp::new(cc, recovery_dir, sys_locale::get_locale);
                // Files given on the command line (this is also how a
                // double-clicked `.rcraft` file arrives on Windows and Linux).
                reasoncraft::file_ops::open_paths(&mut app, args.iter().cloned());
                Ok(Box::new(app))
            }),
        )
    });
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(startup::StartError::NoWindow(err)) => {
            eprintln!("error: no renderer opened the window: {err}");
            startup::show_failure(&err);
            ExitCode::FAILURE
        }
        Err(startup::StartError::AfterStart(err)) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

/// The main window, drawn with `renderer`, keeping its settings in
/// `settings_file` (or eframe's per-user default when `None`).
fn native_options(
    renderer: eframe::Renderer,
    settings_file: Option<std::path::PathBuf>,
) -> eframe::NativeOptions {
    let mut viewport = egui::ViewportBuilder::default()
        .with_title(reasoncraft::i18n::t("app.name"))
        .with_app_id("io.github.Patrick_Batenburg.ReasonCraft")
        .with_inner_size([1280.0, 800.0])
        .with_min_inner_size(reasoncraft::app::MIN_WINDOW);
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
    eframe::NativeOptions {
        viewport,
        renderer,
        wgpu_options: startup::wgpu_options(),
        event_loop_builder: event_loop_hook(),
        persistence_path: settings_file,
        ..Default::default()
    }
}

/// On macOS, no default app menu: its Quit item (Cmd+Q) would quit without
/// asking about unsaved changes, while the key itself reaches the app,
/// which asks first.
#[cfg(target_os = "macos")]
fn event_loop_hook() -> Option<eframe::EventLoopBuilderHook> {
    use winit::platform::macos::EventLoopBuilderExtMacOS as _;
    Some(Box::new(|builder| {
        builder.with_default_menu(false);
    }))
}

#[cfg(not(target_os = "macos"))]
fn event_loop_hook() -> Option<eframe::EventLoopBuilderHook> {
    None
}
