//! The Export ribbon tab: images of the map, text exports and printing.

use eframe::egui::{Button, Ui};

use super::{command, group, large_menu, menu_item, menu_popup};
use crate::actions::{Action, is_enabled};
use crate::app::ReasonCraftApp;
use crate::commands;
use crate::doc_tab::DocTab;
use crate::i18n::t;
use crate::shell::icons::Icon;
use crate::theme::Tokens;

/// A dropdown of text formats: its icon, label key and registry command
/// ids.
struct Menu {
    icon: Icon,
    key: &'static str,
    ids: [&'static str; 3],
}

/// The Outline ▾ and Essay ▾ menus.
const OUTLINE: Menu = Menu {
    icon: Icon::Outline,
    key: "export.outline",
    ids: [
        "export.outline_md",
        "export.outline_txt",
        "export.outline_docx",
    ],
};
const ESSAY: Menu = Menu {
    icon: Icon::Essay,
    key: "export.essay",
    ids: ["export.essay_md", "export.essay_txt", "export.essay_docx"],
};

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
        menu(ui, &tk, tab, &OUTLINE, &mut out);
        menu(ui, &tk, tab, &ESSAY, &mut out);
        command(ui, &tk, "export.argdown", true, false, &mut out);
    });
    group(ui, &tk, Icon::Print, &t("group.print"), |ui| {
        command(ui, &tk, "export.page_setup", true, false, &mut out);
        command(ui, &tk, "export.print", true, false, &mut out);
    });
    app.pending.extend(out);
}

/// Draw `m`, greyed out when none of its formats can run for `tab`
/// (Essay in a grouping map).
fn menu(ui: &mut Ui, tk: &Tokens, tab: &DocTab, m: &Menu, out: &mut Vec<Action>) {
    let enabled = m
        .ids
        .iter()
        .any(|id| is_enabled(tab, &commands::get(id).action));
    let resp = large_menu(ui, tk, m.icon, &t(m.key), enabled);
    menu_popup(ui, &resp, |ui| {
        for id in m.ids {
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
        for id in OUTLINE.ids.into_iter().chain(ESSAY.ids) {
            commands::get(id);
        }
    }
}
