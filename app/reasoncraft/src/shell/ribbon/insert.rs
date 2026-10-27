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
        for (icon, id) in [
            (Icon::Reason, "home.add_reason"),
            (Icon::Objection, "home.add_objection"),
            (Icon::CoPremise, "home.add_co_premise"),
        ] {
            command(ui, &tk, icon, id, enabled(id), false, &mut out);
        }
        small_column(ui, |ui| {
            for (icon, id) in [
                (Icon::Category, "home.add_category"),
                (Icon::Note, "home.add_note"),
            ] {
                small_command(ui, &tk, icon, id, enabled(id), &mut out);
            }
        });
    });
    group(ui, &tk, Icon::Objection, &t("group.whole_card"), |ui| {
        small_column(ui, |ui| {
            for (icon, id) in [
                (Icon::Reason, "home.whole_card_reason"),
                (Icon::Objection, "home.whole_card_objection"),
            ] {
                small_command(ui, &tk, icon, id, enabled(id), &mut out);
            }
        });
    });
    group(ui, &tk, Icon::Source, &t("group.sources"), |ui| {
        source_menu(ui, &tk, tab, &mut out);
    });
    group(ui, &tk, Icon::BoxType, &t("group.box"), |ui| {
        small_column(ui, |ui| {
            for (icon, id) in [
                (Icon::Comment, "home.comment"),
                (Icon::Link, "home.link"),
                (Icon::Image, "home.image"),
            ] {
                small_command(ui, &tk, icon, id, enabled(id), &mut out);
            }
        });
    });
    app.pending.extend(out);
}
