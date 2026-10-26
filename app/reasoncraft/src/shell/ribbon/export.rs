//! The Export ribbon tab: images of the map, text exports and printing.

use eframe::egui::{Button, Popup, Ui};

use super::{group, large, large_menu};
use crate::actions::Action;
use crate::app::ReasonCraftApp;
use crate::export::ExportFormat;
use crate::i18n::t;
use crate::shell::icons::Icon;
use crate::theme::Tokens;

pub(super) fn show(ui: &mut Ui, app: &mut ReasonCraftApp) {
    let tk = app.tokens;
    let mut out = Vec::new();
    group(ui, &tk, &t("group.image"), |ui| {
        let images = [
            (Icon::ExportPng, "export.png", ExportFormat::Png),
            (Icon::ExportSvg, "export.svg", ExportFormat::Svg),
            (Icon::ExportPdf, "export.pdf", ExportFormat::Pdf),
        ];
        for (icon, key, format) in images {
            if large(ui, &tk, icon, &t(key), true, false).clicked() {
                out.push(Action::OpenExport(format));
            }
        }
        let label = t("action.copy_image");
        if large(ui, &tk, Icon::CopyImage, &label, true, false).clicked() {
            out.push(Action::CopyAsImage);
        }
    });
    group(ui, &tk, &t("group.text"), |ui| {
        let outline = [
            ExportFormat::OutlineMarkdown,
            ExportFormat::OutlineText,
            ExportFormat::OutlineDocx,
        ];
        menu(ui, &tk, Icon::Outline, "export.outline", &outline, &mut out);
        let essay = [
            ExportFormat::EssayMarkdown,
            ExportFormat::EssayText,
            ExportFormat::EssayDocx,
        ];
        menu(ui, &tk, Icon::Essay, "export.essay", &essay, &mut out);
    });
    group(ui, &tk, &t("group.print"), |ui| {
        let label = t("export.page_setup");
        if large(ui, &tk, Icon::PageSetup, &label, true, false).clicked() {
            out.push(Action::OpenPageSetup);
        }
        if large(ui, &tk, Icon::Print, &t("export.print"), true, false).clicked() {
            out.push(Action::Print);
        }
    });
    app.pending.extend(out);
}

/// A dropdown of text formats.
fn menu(
    ui: &mut Ui,
    tk: &Tokens,
    icon: Icon,
    key: &str,
    formats: &[ExportFormat],
    out: &mut Vec<Action>,
) {
    let resp = large_menu(ui, tk, icon, &t(key), true);
    Popup::menu(&resp).show(|ui| {
        for &format in formats {
            if ui.add(Button::new(t(format.name_key()))).clicked() {
                out.push(Action::OpenExport(format));
            }
        }
    });
}
