//! The View ribbon tab: zoom, theme, panels, the layout direction,
//! evaluation display and (debug builds) the sample map.

use eframe::egui::{Button, Popup, Ui};
use reasoncraft_core::LayoutDirection;

use super::{command, evaluate, group, large_menu, small_command};
use crate::actions::Action;
use crate::app::ReasonCraftApp;
use crate::commands;
use crate::i18n::t;
use crate::shell::icons::{Icon, direction_atom};
use crate::theme::ThemeChoice;

pub(super) fn show(ui: &mut Ui, app: &mut ReasonCraftApp) {
    let tk = app.tokens;
    let mut out = Vec::new();
    group(ui, &tk, &t("group.zoom"), |ui| {
        command(
            ui,
            &tk,
            Icon::ZoomFit,
            "view.zoom_fit",
            true,
            false,
            &mut out,
        );
        command(
            ui,
            &tk,
            Icon::Zoom100,
            "view.zoom_100",
            true,
            false,
            &mut out,
        );
        ui.vertical(|ui| {
            small_command(ui, &tk, Icon::ZoomIn, "view.zoom_in", true, &mut out);
            small_command(ui, &tk, Icon::ZoomOut, "view.zoom_out", true, &mut out);
        });
    });
    group(ui, &tk, &t("group.theme"), |ui| {
        let themes = [
            (Icon::ThemeSystem, "view.theme.system", ThemeChoice::System),
            (Icon::ThemeLight, "view.theme.light", ThemeChoice::Light),
            (Icon::ThemeDark, "view.theme.dark", ThemeChoice::Dark),
        ];
        for (icon, id, choice) in themes {
            let on = app.settings.theme == choice;
            command(ui, &tk, icon, id, true, on, &mut out);
        }
    });
    group(ui, &tk, &t("group.panels"), |ui| {
        let panels = [
            (Icon::PanelLeft, "view.panel_left", app.left_panel),
            (Icon::PanelRight, "view.panel_right", app.right_panel),
        ];
        for (icon, id, on) in panels {
            command(ui, &tk, icon, id, true, on, &mut out);
        }
    });
    layout_group(ui, app, &mut out);
    group(ui, &tk, &t("group.evaluation"), |ui| {
        evaluate::display_toggles(ui, app, &mut out);
    });
    #[cfg(debug_assertions)]
    group(ui, &tk, &t("group.debug"), |ui| {
        command(
            ui,
            &tk,
            Icon::Open,
            "view.load_sample",
            true,
            false,
            &mut out,
        );
        let on = app.show_source_icons;
        command(
            ui,
            &tk,
            Icon::Source,
            "view.source_icons",
            true,
            on,
            &mut out,
        );
    });
    app.pending.extend(out);
}

/// "Direction ▾": the active document's layout direction, with a menu of
/// all three (each with its glyph).
fn layout_group(ui: &mut Ui, app: &ReasonCraftApp, out: &mut Vec<Action>) {
    let tk = app.tokens;
    let current = app.active_tab().doc.layout_direction;
    group(ui, &tk, &t("group.layout"), |ui| {
        let label = t("action.layout_direction");
        let resp = large_menu(ui, &tk, Icon::Direction(current), &label, true);
        Popup::menu(&resp).show(|ui| {
            for d in LayoutDirection::ALL {
                let cmd = commands::get(&format!("view.direction.{}", d.key()));
                let button = Button::new((direction_atom(d, 16.0), t(cmd.label)));
                if ui.add(button.selected(d == current)).clicked() {
                    out.push(cmd.action.clone());
                }
            }
        });
    });
}
