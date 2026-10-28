//! The View ribbon tab: zoom, theme, the interface language, panels, the
//! layout direction and connector style, the minimap and numbering,
//! evaluation display and (debug builds) the sample map.

use eframe::egui::{Button, Ui};
use reasoncraft_core::{ConnectorStyle, LayoutDirection};

use super::{
    command, evaluate, group, large_menu, menu_item, menu_popup, small_column, small_command,
};
use crate::actions::Action;
use crate::app::ReasonCraftApp;
use crate::commands;
use crate::i18n::{UI_LANGUAGES, t};
use crate::settings::MinimapMode;
use crate::shell::icons::{Icon, connector_atom, direction_atom};
use crate::theme::ThemeChoice;

pub(super) fn show(ui: &mut Ui, app: &mut ReasonCraftApp) {
    let tk = app.tokens;
    let mut out = Vec::new();
    group(ui, &tk, Icon::ZoomFit, &t("group.zoom"), |ui| {
        command(ui, &tk, "view.zoom_fit", true, false, &mut out);
        command(ui, &tk, "view.zoom_100", true, false, &mut out);
        command(
            ui,
            &tk,
            "view.zoom_fit_selection",
            app.active_tab().selection.is_some(),
            false,
            &mut out,
        );
        small_column(ui, |ui| {
            small_command(ui, &tk, "view.zoom_in", true, &mut out);
            small_command(ui, &tk, "view.zoom_out", true, &mut out);
        });
    });
    group(ui, &tk, Icon::ThemeSystem, &t("group.theme"), |ui| {
        let themes = [
            ("view.theme.system", ThemeChoice::System),
            ("view.theme.light", ThemeChoice::Light),
            ("view.theme.dark", ThemeChoice::Dark),
        ];
        for (id, choice) in themes {
            let on = app.settings.theme == choice;
            command(ui, &tk, id, true, on, &mut out);
        }
    });
    language_group(ui, app, &mut out);
    group(ui, &tk, Icon::PanelLeft, &t("group.panels"), |ui| {
        let panels = [
            ("view.panel_left", app.left_panel),
            ("view.panel_right", app.right_panel),
        ];
        for (id, on) in panels {
            command(ui, &tk, id, true, on, &mut out);
        }
    });
    layout_group(ui, app, &mut out);
    overview_group(ui, app, &mut out);
    group(ui, &tk, Icon::Shading, &t("group.evaluation"), |ui| {
        evaluate::display_toggles(ui, app, &mut out);
    });
    #[cfg(debug_assertions)]
    group(ui, &tk, Icon::Open, &t("group.debug"), |ui| {
        command(ui, &tk, "view.load_sample", true, false, &mut out);
        let on = app.show_source_icons;
        command(ui, &tk, "view.source_icons", true, on, &mut out);
    });
    app.pending.extend(out);
}

/// "Language ▾": follow the OS, or one of the UI languages, each named in
/// its own language.
fn language_group(ui: &mut Ui, app: &ReasonCraftApp, out: &mut Vec<Action>) {
    let tk = app.tokens;
    let current = app.settings.ui_language_choice();
    group(ui, &tk, Icon::Language, &t("group.language"), |ui| {
        let label = t("action.ui_language");
        let resp = large_menu(ui, &tk, Icon::Language, &label, true);
        menu_popup(ui, &resp, |ui| {
            let langs = UI_LANGUAGES.map(|l| (l, Some(l)));
            for (key, lang) in std::iter::once(("system", None)).chain(langs) {
                let cmd = commands::get(&format!("view.language.{key}"));
                let on = lang == current;
                if menu_item(ui, true, Button::new(t(cmd.label)).selected(on)) && !on {
                    out.push(cmd.action.clone());
                }
            }
        });
    });
}

/// "Direction ▾": the active document's layout direction, with a menu of
/// all three (each with its glyph); "Connectors ▾": the connector style
/// (a personal setting), each previewed by its glyph.
fn layout_group(ui: &mut Ui, app: &ReasonCraftApp, out: &mut Vec<Action>) {
    let tk = app.tokens;
    let current = app.active_tab().doc.layout_direction;
    let style = app.settings.connector_style;
    group(
        ui,
        &tk,
        Icon::Direction(current),
        &t("group.layout"),
        |ui| {
            let label = t("action.layout_direction");
            let resp = large_menu(ui, &tk, Icon::Direction(current), &label, true);
            menu_popup(ui, &resp, |ui| {
                for d in LayoutDirection::ALL {
                    let cmd = commands::get(&format!("view.direction.{}", d.key()));
                    let button = Button::new((direction_atom(d, 16.0), t(cmd.label)));
                    if menu_item(ui, true, button.selected(d == current)) {
                        out.push(cmd.action.clone());
                    }
                }
            });
            let label = t("action.connector_style");
            let resp = large_menu(ui, &tk, Icon::Connector(style), &label, true);
            menu_popup(ui, &resp, |ui| {
                for s in ConnectorStyle::ALL {
                    let cmd = commands::get(&format!("view.connector.{}", s.key()));
                    let button = Button::new((connector_atom(s, 16.0), t(cmd.label)));
                    if menu_item(ui, true, button.selected(s == style)) {
                        out.push(cmd.action.clone());
                    }
                }
            });
        },
    );
}

/// "Minimap ▾" (automatic, always or never) and the "Numbers" toggle.
fn overview_group(ui: &mut Ui, app: &ReasonCraftApp, out: &mut Vec<Action>) {
    let tk = app.tokens;
    let current = app.settings.minimap;
    group(ui, &tk, Icon::Minimap, &t("group.overview"), |ui| {
        let resp = large_menu(ui, &tk, Icon::Minimap, &t("action.minimap"), true);
        menu_popup(ui, &resp, |ui| {
            for m in MinimapMode::ALL {
                let cmd = commands::get(&format!("view.minimap.{}", m.key()));
                if menu_item(ui, true, Button::new(t(cmd.label)).selected(m == current)) {
                    out.push(cmd.action.clone());
                }
            }
        });
        let on = app.settings.show_numbers;
        command(ui, &tk, "view.numbers", true, on, out);
    });
}
