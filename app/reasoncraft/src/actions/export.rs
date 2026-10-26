//! Exports: the export and page setup dialogs, export jobs, Copy as image
//! and Print.
//!
//! What an export draws from is taken here, on the UI thread: the scene
//! (text is measured with egui) or a copy of the document, with a copy of
//! the assets. The rest runs on a worker (see [`crate::export::ExportJobs`]).

use std::path::PathBuf;

use eframe::egui;
use reasoncraft_core::{Item, layout};

use super::Action;
use crate::app::ReasonCraftApp;
use crate::canvas::measure::{EguiMeasure, MeasureCache};
use crate::export::{
    ExportError, ExportFormat, ExportOptions, ExportRequest, ExportScope, ExportTheme, ExportWork,
    Notice, Outcome, Snapshot, Target,
};
use crate::file_ops::{Modal, with_extension};
use crate::i18n::{FluentArgs, t, t_args};
use crate::scene::{Scene, SceneOptions, build_scene};
use crate::shell::export_dialog::ExportDialog;
use crate::theme::Tokens;

/// Carry out an export action.
pub(super) fn apply(app: &mut ReasonCraftApp, action: Action) {
    match action {
        Action::OpenExport(format) if format.is_image() => {
            let mut options = app.settings.export;
            if app.active_tab().selection.is_none() {
                options.scope = ExportScope::Map;
            }
            let dialog = ExportDialog::export(format, options, app.settings.page_setup);
            app.export_dialog = Some(dialog);
        }
        Action::OpenExport(format) => {
            let request = ExportRequest {
                format,
                options: app.settings.export,
                page: app.settings.page_setup,
            };
            run(app, request);
        }
        Action::RunExport(request) => {
            app.export_dialog = None;
            app.settings.export = request.options;
            if request.format == ExportFormat::Pdf {
                app.settings.page_setup = request.page;
            }
            run(app, request);
        }
        Action::CloseExportDialog => app.export_dialog = None,
        Action::OpenPageSetup => {
            app.export_dialog = Some(ExportDialog::page_setup(app.settings.page_setup));
        }
        Action::SetPageSetup(page) => {
            app.settings.page_setup = page;
            app.export_dialog = None;
        }
        Action::CopyAsImage => {
            let tokens = tokens_for(app, app.settings.export.theme);
            let only = scope_item(app, app.settings.export.scope);
            let snapshot = scene_snapshot(app, only, tokens);
            let target = Target::Clipboard(app.export.clipboard.clone());
            let options = ExportOptions {
                png_scale: 2,
                transparent: false,
                ..ExportOptions::default()
            };
            start(app, ExportFormat::Png, options, snapshot, target);
        }
        Action::Print => {
            let options = app.settings.export;
            let tokens = tokens_for(app, options.theme);
            let snapshot = scene_snapshot(app, None, tokens);
            let target = Target::Print(app.export.print_dir.clone());
            start(app, ExportFormat::Pdf, options, snapshot, target);
        }
        Action::ResolveExportReplace(replace) => {
            let Some(Modal::ConfirmExportReplace { path, request }) = app.modal.take() else {
                return;
            };
            if replace {
                export_to(app, request, path);
            }
        }
        Action::ResolveClosingExports(quit) => {
            if app.modal != Some(Modal::ExportsRunning) {
                return;
            }
            app.modal = None;
            if quit {
                app.export.stop();
                app.confirm_close();
            }
        }
        Action::OpenExportFolder => {
            let Some(Notice::Saved(path)) = &app.export.notice else {
                return;
            };
            let Some(dir) = path.parent() else { return };
            if let Err(e) = app.link_opener.open(&dir.to_string_lossy()) {
                show_error(app, ExportError::Open(e));
            }
        }
        _ => {}
    }
}

/// Ask where `request` goes and start it. A cancelled dialog does nothing.
/// When the extension had to be added and that file exists, ask before
/// replacing it: the dialog only checked the name without it.
fn run(app: &mut ReasonCraftApp, request: ExportRequest) {
    let format = request.format;
    let suggested = format!("{}.{}", app.active_tab().name(), format.extension());
    let Some(chosen) = app.dialogs.pick_export(format, &suggested) else {
        return;
    };
    let path = with_extension(chosen.clone(), format.extension());
    if path != chosen && path.exists() {
        app.modal = Some(Modal::ConfirmExportReplace { path, request });
        return;
    }
    export_to(app, request, path);
}

/// Start `request`, written to `path`.
fn export_to(app: &mut ReasonCraftApp, request: ExportRequest, path: PathBuf) {
    let ExportRequest {
        format,
        options,
        page,
    } = request;
    let snapshot = if format.is_image() {
        let only = scope_item(app, options.scope);
        scene_snapshot(app, only, tokens_for(app, options.theme))
    } else {
        Snapshot::Doc {
            doc: Box::new(app.active_tab().doc.clone()),
            outline: app.settings.outline,
        }
    };
    app.export.start(ExportWork {
        format,
        options,
        page,
        snapshot,
        target: Target::File(path),
    });
}

fn start(
    app: &mut ReasonCraftApp,
    format: ExportFormat,
    options: ExportOptions,
    snapshot: Snapshot,
    target: Target,
) {
    let page = app.settings.page_setup;
    app.export.start(ExportWork {
        format,
        options,
        page,
        snapshot,
        target,
    });
}

/// The item an image export in `scope` shows with what is under it:
/// `None` (the whole map) for the map scope or when nothing is selected.
fn scope_item(app: &ReasonCraftApp, scope: ExportScope) -> Option<Item> {
    match scope {
        ExportScope::Map => None,
        ExportScope::Selection => app.active_tab().selection,
    }
}

/// The colours an export in `theme` uses.
fn tokens_for(app: &ReasonCraftApp, theme: ExportTheme) -> Tokens {
    match theme {
        ExportTheme::Light => Tokens::light(),
        ExportTheme::Current => app.tokens,
    }
}

/// The active map (or `only` and what is under it) as the canvas shows
/// it, at zoom 1 in the colours of `tokens`, with a copy of its images.
fn scene_snapshot(app: &mut ReasonCraftApp, only: Option<Item>, tokens: Tokens) -> Snapshot {
    let ReasonCraftApp {
        tabs,
        active,
        settings,
        measure_cache,
        egui_ctx,
        ..
    } = app;
    let tab = &tabs[*active];
    let params = settings.layout_params();
    let opts = SceneOptions {
        show_markers: settings.show_markers,
        numbers: settings.show_numbers,
        eval_shading: settings.eval_shading,
        connector_style: settings.connector_style,
        only,
        ..SceneOptions::export(tokens)
    };
    let build = |ctx: &egui::Context, cache: &mut MeasureCache| -> Scene {
        let measure = EguiMeasure::new(ctx, cache);
        let lay = layout(&tab.doc, &measure, &params);
        build_scene(&tab.doc, &lay, &opts, &measure)
    };
    let scene = match egui_ctx {
        Some(ctx) => build(ctx, measure_cache),
        // Outside a frame (actions applied directly): measure with a
        // context of our own, which has the same fonts.
        None => {
            let ctx = egui::Context::default();
            let mut scene = None;
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                scene = Some(build(ui.ctx(), &mut MeasureCache::default()));
            });
            // Nothing is drawn: the font atlas this frame made is not needed.
            output.textures_delta.clear();
            scene.unwrap_or(Scene {
                shapes: Vec::new(),
                bounds: egui::Rect::ZERO,
            })
        }
    };
    Snapshot::Scene {
        scene,
        assets: tab.assets.clone(),
        tokens,
    }
}

/// The window is closing and every tab is resolved: close now, or keep
/// the window open with a waiting prompt until the running exports finish.
/// An export error not shown yet (it failed behind another prompt) stops
/// the close: the next [`poll_exports`] shows it.
pub(crate) fn close_after_exports(app: &mut ReasonCraftApp) {
    if app.export.busy() {
        app.modal = Some(Modal::ExportsRunning);
        return;
    }
    app.modal = None;
    if app.export.waiting_errors.is_empty() {
        app.confirm_close();
    }
}

/// Picks up the exports that ended: a notice for a written file or a
/// copied image, the PDF viewer for a print, an error message otherwise.
/// A close waiting for the exports goes ahead once they are done, unless
/// one failed: then its message shows and the window stays open.
pub fn poll_exports(app: &mut ReasonCraftApp) {
    for result in app.export.finished() {
        match result {
            Ok(Outcome::Saved(path)) => app.export.notice = Some(Notice::Saved(path)),
            Ok(Outcome::Copied) => app.export.notice = Some(Notice::Copied),
            Ok(Outcome::ReadyToPrint(path)) => {
                if let Err(e) = app.link_opener.open(&path.to_string_lossy()) {
                    show_error(app, ExportError::Open(e));
                }
            }
            Err(e) => show_error(app, e),
        }
    }
    if app.modal == Some(Modal::ExportsRunning) && !app.export.busy() {
        app.modal = None;
        if app.export.waiting_errors.is_empty() {
            app.confirm_close();
            return;
        }
    }
    if app.modal.is_none()
        && let Some(err) = app.export.waiting_errors.pop_front()
    {
        show_error(app, err);
    }
}

/// Shows `err`, or keeps it for later when another message is showing.
fn show_error(app: &mut ReasonCraftApp, err: ExportError) {
    if app.modal.is_some() {
        app.export.waiting_errors.push_back(err);
        return;
    }
    let mut args = FluentArgs::new();
    args.set("detail", err.detail());
    app.modal = Some(Modal::Error {
        title: t("export.error.title"),
        message: t_args(err.key(), &args),
    });
}
