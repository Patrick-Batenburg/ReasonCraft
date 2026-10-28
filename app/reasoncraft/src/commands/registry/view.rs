//! The View tab's commands: zoom, theme, language, panels, layout and
//! overview.

use reasoncraft_core::{ConnectorStyle, LayoutDirection};

use super::cmd;
use crate::actions::{Action, Panel};
use crate::commands::Command;
use crate::settings::MinimapMode;
use crate::shell::icons::Icon;
use crate::theme::ThemeChoice;

pub(super) fn commands() -> Vec<Command> {
    use Action as A;
    let (zoom, theme, panels) = ("group.zoom", "group.theme", "group.panels");
    let language = |id, label, lang: Option<&str>| {
        let action = A::SetUiLanguage(lang.map(str::to_owned));
        cmd(id, label, "group.language", action)
    };
    let direction = |id, d: LayoutDirection| {
        let label = match d {
            LayoutDirection::TopDown => "layout_direction.top_down",
            LayoutDirection::BottomUp => "layout_direction.bottom_up",
            LayoutDirection::LeftRight => "layout_direction.left_right",
        };
        cmd(id, label, "group.layout", A::SetLayoutDirection(d))
    };
    let connector = |id, s: ConnectorStyle| {
        let label = match s {
            ConnectorStyle::Curved => "connector_style.curved",
            ConnectorStyle::Elbow => "connector_style.elbow",
            ConnectorStyle::Tapered => "connector_style.tapered",
        };
        cmd(id, label, "group.layout", A::SetConnectorStyle(s))
    };
    let overview = "group.overview";
    let minimap = |id, m: MinimapMode| {
        let label = match m {
            MinimapMode::Auto => "minimap.auto",
            MinimapMode::On => "minimap.on",
            MinimapMode::Off => "minimap.off",
        };
        cmd(id, label, overview, A::SetMinimap(m))
    };
    #[allow(unused_mut)] // Debug builds add their commands.
    let mut out = vec![
        cmd("view.zoom_fit", "action.zoom_fit", zoom, A::ZoomFit).with_icon(Icon::ZoomFit),
        cmd("view.zoom_100", "action.zoom_100", zoom, A::Zoom100).with_icon(Icon::Zoom100),
        cmd(
            "view.zoom_fit_selection",
            "action.zoom_fit_selection",
            zoom,
            A::ZoomFitSelection,
        )
        .with_icon(Icon::ZoomFitSelection),
        cmd("view.zoom_in", "action.zoom_in", zoom, A::ZoomIn).with_icon(Icon::ZoomIn),
        cmd("view.zoom_out", "action.zoom_out", zoom, A::ZoomOut).with_icon(Icon::ZoomOut),
        cmd(
            "view.theme.system",
            "theme.system",
            theme,
            A::SetTheme(ThemeChoice::System),
        )
        .with_icon(Icon::ThemeSystem),
        cmd(
            "view.theme.light",
            "theme.light",
            theme,
            A::SetTheme(ThemeChoice::Light),
        )
        .with_icon(Icon::ThemeLight),
        cmd(
            "view.theme.dark",
            "theme.dark",
            theme,
            A::SetTheme(ThemeChoice::Dark),
        )
        .with_icon(Icon::ThemeDark),
        language("view.language.system", "ui_language.system", None),
        language("view.language.en", "ui_language.en", Some("en")),
        language("view.language.nl", "ui_language.nl", Some("nl")),
        cmd(
            "view.panel_left",
            "action.panel_left",
            panels,
            A::TogglePanel(Panel::Left),
        )
        .with_icon(Icon::PanelLeft),
        cmd(
            "view.panel_right",
            "action.panel_right",
            panels,
            A::TogglePanel(Panel::Right),
        )
        .with_icon(Icon::PanelRight),
        cmd("view.pane.next", "action.next_pane", panels, A::NextPane),
        cmd("view.pane.prev", "action.prev_pane", panels, A::PrevPane),
        direction("view.direction.top_down", LayoutDirection::TopDown),
        direction("view.direction.bottom_up", LayoutDirection::BottomUp),
        direction("view.direction.left_right", LayoutDirection::LeftRight),
        connector("view.connector.curved", ConnectorStyle::Curved),
        connector("view.connector.elbow", ConnectorStyle::Elbow),
        connector("view.connector.tapered", ConnectorStyle::Tapered),
        minimap("view.minimap.auto", MinimapMode::Auto),
        minimap("view.minimap.on", MinimapMode::On),
        minimap("view.minimap.off", MinimapMode::Off),
        cmd("view.numbers", "action.numbers", overview, A::ToggleNumbers).with_icon(Icon::Numbers),
    ];
    #[cfg(debug_assertions)]
    out.extend([
        cmd(
            "view.load_sample",
            "action.load_sample",
            "group.debug",
            A::LoadSampleMap,
        )
        .with_icon(Icon::Open),
        cmd(
            "view.source_icons",
            "action.source_icons",
            "group.debug",
            A::ToggleSourceIcons,
        )
        .with_icon(Icon::Source),
    ]);
    out
}
