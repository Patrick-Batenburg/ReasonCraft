//! The Insert ribbon tab: everything that adds to the map in one place,
//! including the whole-card items and the box extras (comment, link,
//! image) that the Home tab spreads over its groups.

use eframe::egui::Ui;

use super::{command, group, home::source_menu, small_column, small_command};
use crate::actions::is_enabled;
use crate::app::ReasonCraftApp;
use crate::commands;
use crate::i18n::t;
use crate::shell::icons::Icon;

pub(super) fn show(ui: &mut Ui, app: &mut ReasonCraftApp) {
    let tk = app.tokens;
    let tab = app.active_tab();
    let enabled = |id: &str| is_enabled(tab, &commands::get(id).action);
    let mut out = Vec::new();

    group(ui, &tk, Icon::Reason, &t("group.boxes"), |ui| {
        for id in [
            "home.add_reason",
            "home.add_objection",
            "home.add_co_premise",
        ] {
            command(ui, &tk, id, enabled(id), false, &mut out);
        }
        small_column(ui, |ui| {
            for id in ["home.add_category", "home.add_note"] {
                small_command(ui, &tk, id, enabled(id), &mut out);
            }
        });
    });
    group(ui, &tk, Icon::Objection, &t("group.whole_card"), |ui| {
        small_column(ui, |ui| {
            for id in ["home.whole_card_reason", "home.whole_card_objection"] {
                small_command(ui, &tk, id, enabled(id), &mut out);
            }
        });
    });
    group(ui, &tk, Icon::Source, &t("group.sources"), |ui| {
        source_menu(ui, &tk, tab, &mut out);
    });
    group(ui, &tk, Icon::BoxType, &t("group.box"), |ui| {
        small_column(ui, |ui| {
            for id in ["home.comment", "home.link", "home.image"] {
                small_command(ui, &tk, id, enabled(id), &mut out);
            }
        });
    });
    app.pending.extend(out);
}
