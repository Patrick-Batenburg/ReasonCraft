//! The export dialog (scope, theme and the format's own options) and the
//! page setup dialog. Esc or a click outside cancels.

use eframe::egui::{self, DragValue, RichText, Ui};

use crate::actions::Action;
use crate::app::ReasonCraftApp;
use crate::export::{
    ExportFormat, ExportOptions, ExportRequest, ExportScope, ExportTheme, Fit, Orientation,
    PageSetup, Paper,
};
use crate::i18n::{FluentArgs, t, t_args};
use crate::theme::Tokens;

/// The dialog's choices while it is open: kept apart from the settings
/// until the user exports (or confirms the page setup).
#[derive(Clone, Debug, PartialEq)]
pub struct ExportDialog {
    /// The export to make; `None` is the page setup dialog.
    pub format: Option<ExportFormat>,
    pub options: ExportOptions,
    pub page: PageSetup,
}

impl ExportDialog {
    /// The export dialog for `format`, starting from the remembered choices.
    pub fn export(format: ExportFormat, options: ExportOptions, page: PageSetup) -> Self {
        Self {
            format: Some(format),
            options,
            page,
        }
    }

    /// The page setup dialog.
    pub fn page_setup(page: PageSetup) -> Self {
        Self {
            format: None,
            options: ExportOptions::default(),
            page,
        }
    }

    /// What the confirm button does.
    fn confirm(&self) -> Action {
        match self.format {
            Some(format) => Action::RunExport(ExportRequest {
                format,
                options: self.options,
                page: self.page,
            }),
            None => Action::SetPageSetup(self.page),
        }
    }
}

/// Lowest and highest tile scale offered, and margins in mm.
const TILE_SCALE: std::ops::RangeInclusive<f32> = 0.25..=4.0;
const MARGIN_MM: std::ops::RangeInclusive<f32> = 0.0..=40.0;

/// Draw the open export or page setup dialog, if any, and queue the answer.
pub fn show(ctx: &egui::Context, app: &mut ReasonCraftApp) {
    let Some(mut dialog) = app.export_dialog.clone() else {
        return;
    };
    let tk = app.tokens;
    let has_selection = app.active_tab().selection.is_some();
    let frame = egui::Frame::popup(&ctx.global_style())
        .fill(tk.panel)
        .inner_margin(egui::Margin::same(20));
    let response = egui::Modal::new(egui::Id::unique("export_dialog"))
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_width(380.0);
            ui.spacing_mut().item_spacing.y = 8.0;
            let title = match dialog.format {
                Some(format) => {
                    let mut args = FluentArgs::new();
                    args.set("format", t(format.name_key()));
                    t_args("export.dialog.title", &args)
                }
                None => t("export.page_setup.title"),
            };
            ui.label(RichText::new(title).strong().size(16.0).color(tk.box_text));
            match dialog.format {
                Some(format) => image_options(ui, &tk, format, &mut dialog, has_selection),
                None => page_options(ui, &tk, &mut dialog.page),
            }
            ui.add_space(6.0);
            let mut answer = None;
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let confirm = match dialog.format {
                    Some(_) => t("export.dialog.export"),
                    None => t("export.dialog.ok"),
                };
                if ui.button(confirm).clicked() {
                    answer = Some(dialog.confirm());
                }
                if ui.button(t("export.dialog.cancel")).clicked() {
                    answer = Some(Action::CloseExportDialog);
                }
            });
            answer
        });
    let cancel = response.should_close().then_some(Action::CloseExportDialog);
    match response.inner.or(cancel) {
        Some(action) => app.push(action),
        None => app.export_dialog = Some(dialog),
    }
}

/// Scope, theme and the options of image format `format`.
fn image_options(
    ui: &mut Ui,
    tk: &Tokens,
    format: ExportFormat,
    dialog: &mut ExportDialog,
    has_selection: bool,
) {
    let opts = &mut dialog.options;
    heading(ui, tk, "export.scope");
    ui.horizontal(|ui| {
        ui.radio_value(&mut opts.scope, ExportScope::Map, t("export.scope.map"));
        ui.add_enabled_ui(has_selection, |ui| {
            let label = t("export.scope.selection");
            ui.radio_value(&mut opts.scope, ExportScope::Selection, label);
        });
    });
    heading(ui, tk, "export.theme");
    ui.horizontal(|ui| {
        ui.radio_value(&mut opts.theme, ExportTheme::Light, t("export.theme.light"));
        let label = t("export.theme.current");
        ui.radio_value(&mut opts.theme, ExportTheme::Current, label);
    });
    match format {
        ExportFormat::Png => {
            heading(ui, tk, "export.png.scale");
            ui.horizontal(|ui| {
                for scale in 1..=4u8 {
                    let mut args = FluentArgs::new();
                    args.set("scale", scale);
                    let label = t_args("export.png.scale_value", &args);
                    ui.selectable_value(&mut opts.png_scale, scale, label);
                }
            });
            ui.checkbox(&mut opts.transparent, t("export.png.transparent"));
        }
        ExportFormat::Svg => {
            ui.checkbox(&mut opts.svg_outlines, t("export.svg.outlines"));
            if opts.svg_outlines {
                ui.label(
                    RichText::new(t("export.note.emoji"))
                        .size(12.0)
                        .color(tk.text_muted),
                );
            }
        }
        ExportFormat::Pdf => page_options(ui, tk, &mut dialog.page),
        _ => {}
    }
}

/// Paper, orientation, fit and margin.
fn page_options(ui: &mut Ui, tk: &Tokens, page: &mut PageSetup) {
    heading(ui, tk, "export.page.paper");
    ui.horizontal(|ui| {
        ui.radio_value(&mut page.paper, Paper::A4, t("export.page.a4"));
        ui.radio_value(&mut page.paper, Paper::Letter, t("export.page.letter"));
    });
    heading(ui, tk, "export.page.orientation");
    ui.horizontal(|ui| {
        let (portrait, landscape) = (Orientation::Portrait, Orientation::Landscape);
        ui.radio_value(&mut page.orientation, portrait, t("export.page.portrait"));
        ui.radio_value(&mut page.orientation, landscape, t("export.page.landscape"));
    });
    heading(ui, tk, "export.page.fit");
    ui.horizontal(|ui| {
        let tiled = matches!(page.fit, Fit::Tile { .. });
        if ui.radio(!tiled, t("export.page.one_page")).clicked() {
            page.fit = Fit::OnePage;
        }
        if ui.radio(tiled, t("export.page.tile")).clicked() && !tiled {
            page.fit = Fit::Tile { scale: 1.0 };
        }
        if let Fit::Tile { scale } = &mut page.fit {
            let mut percent = *scale * 100.0;
            let range = TILE_SCALE.start() * 100.0..=TILE_SCALE.end() * 100.0;
            let field = DragValue::new(&mut percent)
                .range(range)
                .speed(5.0)
                .suffix(" %");
            if ui.add(field).changed() {
                *scale = percent / 100.0;
            }
        }
    });
    ui.horizontal(|ui| {
        ui.label(t("export.page.margin"));
        let field = DragValue::new(&mut page.margin_mm)
            .range(MARGIN_MM)
            .speed(0.5)
            .suffix(" mm");
        ui.add(field);
    });
}

fn heading(ui: &mut Ui, tk: &Tokens, key: &str) {
    ui.label(RichText::new(t(key)).size(12.0).color(tk.text_muted));
}
