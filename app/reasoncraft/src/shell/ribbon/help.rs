//! The Help ribbon tab: the help topics, the keyboard shortcuts, About
//! and the example maps.

use eframe::egui::Ui;

use super::{command, group};
use crate::app::ReasonCraftApp;
use crate::i18n::t;
use crate::shell::icons::Icon;

pub(super) fn show(ui: &mut Ui, app: &mut ReasonCraftApp) {
    let tk = app.tokens;
    let mut out = Vec::new();
    group(ui, &tk, Icon::Help, &t("group.help"), |ui| {
        let buttons = [
            (Icon::Help, "help.topics"),
            (Icon::Keyboard, "help.keyboard"),
            (Icon::About, "help.about"),
        ];
        for (icon, id) in buttons {
            command(ui, &tk, icon, id, true, false, &mut out);
        }
    });
    group(ui, &tk, Icon::Reason, &t("group.examples"), |ui| {
        let buttons = [
            (Icon::Reason, "help.example_reasoning"),
            (Icon::Category, "help.example_grouping"),
        ];
        for (icon, id) in buttons {
            command(ui, &tk, icon, id, true, false, &mut out);
        }
    });
    app.pending.extend(out);
}
