//! The View ribbon tab: zoom, theme, panels, the layout direction and
//! connector style, the minimap and numbering, evaluation display and
//! (debug builds) the sample map.

use eframe::egui::{Button, Popup, Ui};
use reasoncraft_core::{ConnectorStyle, LayoutDirection};

use super::{command, evaluate, group, large_menu, small_command};
use crate::actions::Action;
use crate::app::ReasonCraftApp;
use crate::commands;
use crate::i18n::t;
use crate::settings::MinimapMode;
use crate::shell::icons::{Icon, connector_atom, direction_atom};
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
        command(
            ui,
            &tk,
            Icon::ZoomFitSelection,
            "view.zoom_fit_selection",
            app.active_tab().selection.is_some(),
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
    overview_group(ui, app, &mut out);
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
/// all three (each with its glyph); "Connectors ▾": the connector style
/// (a personal setting), each previewed by its glyph.
fn layout_group(ui: &mut Ui, app: &ReasonCraftApp, out: &mut Vec<Action>) {
    let tk = app.tokens;
    let current = app.active_tab().doc.layout_direction;
    let style = app.settings.connector_style;
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
        let label = t("action.connector_style");
        let resp = large_menu(ui, &tk, Icon::Connector(style), &label, true);
        Popup::menu(&resp).show(|ui| {
            for s in ConnectorStyle::ALL {
                let cmd = commands::get(&format!("view.connector.{}", s.key()));
                let button = Button::new((connector_atom(s, 16.0), t(cmd.label)));
                if ui.add(button.selected(s == style)).clicked() {
                    out.push(cmd.action.clone());
                }
            }
        });
    });
}

/// "Minimap ▾" (automatic, always or never) and the "Numbers" toggle.
fn overview_group(ui: &mut Ui, app: &ReasonCraftApp, out: &mut Vec<Action>) {
    let tk = app.tokens;
    let current = app.settings.minimap;
    group(ui, &tk, &t("group.overview"), |ui| {
        let resp = large_menu(ui, &tk, Icon::Minimap, &t("action.minimap"), true);
        Popup::menu(&resp).show(|ui| {
            for m in MinimapMode::ALL {
                let cmd = commands::get(&format!("view.minimap.{}", m.key()));
                if ui
                    .add(Button::new(t(cmd.label)).selected(m == current))
                    .clicked()
                {
                    out.push(cmd.action.clone());
                }
            }
        });
        let on = app.settings.show_numbers;
        command(ui, &tk, Icon::Numbers, "view.numbers", true, on, out);
    });
}
