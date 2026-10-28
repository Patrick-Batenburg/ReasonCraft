//! The Export ribbon tab: images of the map, text exports and printing.

use eframe::egui::{Button, Ui};

use super::{command, group, large_menu, menu_item, menu_popup};
use crate::actions::{Action, is_enabled};
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
    group(ui, &tk, Icon::ExportPng, &t("group.image"), |ui| {
        let images = [
            "export.png",
            "export.svg",
            "export.pdf",
            "export.copy_image",
        ];
        for id in images {
            command(ui, &tk, id, true, false, &mut out);
        }
    });
    group(ui, &tk, Icon::Essay, &t("group.text"), |ui| {
        let tab = app.active_tab();
        let on = |id: &str| is_enabled(tab, &commands::get(id).action);
        let (outline, essay) = (on(OUTLINE[0]), on(ESSAY[0]));
        menu(ui, &tk, Icon::Outline, "export.outline", &OUTLINE, outline, &mut out);
        menu(ui, &tk, Icon::Essay, "export.essay", &ESSAY, essay, &mut out);
        command(ui, &tk, "export.argdown", true, false, &mut out);
    });
    group(ui, &tk, Icon::Print, &t("group.print"), |ui| {
        command(ui, &tk, "export.page_setup", true, false, &mut out);
        command(ui, &tk, "export.print", true, false, &mut out);
    });
    app.pending.extend(out);
}

/// A dropdown of text formats (registry command ids), greyed out when
/// they are not `enabled` (Essay in a grouping map).
fn menu(
    ui: &mut Ui,
    tk: &Tokens,
    icon: Icon,
    key: &str,
    ids: &[&str],
    enabled: bool,
    out: &mut Vec<Action>,
) {
    let resp = large_menu(ui, tk, icon, &t(key), enabled);
    menu_popup(ui, &resp, |ui| {
        for &id in ids {
            let cmd = commands::get(id);
            if menu_item(ui, true, Button::new(t(cmd.label))) {
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
