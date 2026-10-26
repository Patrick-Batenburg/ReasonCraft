//! The Export ribbon tab: images of the map, text exports and printing.

use eframe::egui::{Button, Popup, Ui};

use super::{command, group, large_menu};
use crate::actions::Action;
use crate::app::ReasonCraftApp;
use crate::commands;
use crate::i18n::t;
use crate::shell::icons::Icon;
use crate::theme::Tokens;

/// The Outline ▾ and Essay ▾ menus' commands.
const OUTLINE: [&str; 3] = [
    "export.outline_md",
    "export.outline_txt",
    "export.outline_docx",
];
const ESSAY: [&str; 3] = ["export.essay_md", "export.essay_txt", "export.essay_docx"];

pub(super) fn show(ui: &mut Ui, app: &mut ReasonCraftApp) {
    let tk = app.tokens;
    let mut out = Vec::new();
    group(ui, &tk, &t("group.image"), |ui| {
        let images = [
            (Icon::ExportPng, "export.png"),
            (Icon::ExportSvg, "export.svg"),
            (Icon::ExportPdf, "export.pdf"),
            (Icon::CopyImage, "export.copy_image"),
        ];
        for (icon, id) in images {
            command(ui, &tk, icon, id, true, false, &mut out);
        }
    });
    group(ui, &tk, &t("group.text"), |ui| {
        menu(ui, &tk, Icon::Outline, "export.outline", &OUTLINE, &mut out);
        menu(ui, &tk, Icon::Essay, "export.essay", &ESSAY, &mut out);
        command(
            ui,
            &tk,
            Icon::Argdown,
            "export.argdown",
            true,
            false,
            &mut out,
        );
    });
    group(ui, &tk, &t("group.print"), |ui| {
        command(
            ui,
            &tk,
            Icon::PageSetup,
            "export.page_setup",
            true,
            false,
            &mut out,
        );
        command(ui, &tk, Icon::Print, "export.print", true, false, &mut out);
    });
    app.pending.extend(out);
}

/// A dropdown of text formats (registry command ids).
fn menu(ui: &mut Ui, tk: &Tokens, icon: Icon, key: &str, ids: &[&str], out: &mut Vec<Action>) {
    let resp = large_menu(ui, tk, icon, &t(key), true);
    Popup::menu(&resp).show(|ui| {
        for &id in ids {
            let cmd = commands::get(id);
            if ui.add(Button::new(t(cmd.label))).clicked() {
                out.push(cmd.action.clone());
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The menus only open on a click, so the ribbon test never reaches
    /// these ids.
    #[test]
    fn menu_ids_exist() {
        for id in OUTLINE.into_iter().chain(ESSAY) {
            commands::get(id);
        }
    }
}
