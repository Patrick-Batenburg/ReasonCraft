//! The View tab's commands: zoom, theme, panels, layout and overview.

use reasoncraft_core::{ConnectorStyle, LayoutDirection};

use super::cmd;
use crate::actions::{Action, Panel};
use crate::commands::Command;
use crate::settings::MinimapMode;
use crate::theme::ThemeChoice;

pub(super) fn commands() -> Vec<Command> {
    use Action as A;
    let (zoom, theme, panels) = ("group.zoom", "group.theme", "group.panels");
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
        cmd("view.zoom_fit", "action.zoom_fit", zoom, A::ZoomFit),
        cmd("view.zoom_100", "action.zoom_100", zoom, A::Zoom100),
        cmd(
            "view.zoom_fit_selection",
            "action.zoom_fit_selection",
            zoom,
            A::ZoomFitSelection,
        ),
        cmd("view.zoom_in", "action.zoom_in", zoom, A::ZoomIn),
        cmd("view.zoom_out", "action.zoom_out", zoom, A::ZoomOut),
        cmd(
            "view.theme.system",
            "theme.system",
            theme,
            A::SetTheme(ThemeChoice::System),
        ),
        cmd(
            "view.theme.light",
            "theme.light",
            theme,
            A::SetTheme(ThemeChoice::Light),
        ),
        cmd(
            "view.theme.dark",
            "theme.dark",
            theme,
            A::SetTheme(ThemeChoice::Dark),
        ),
        cmd(
            "view.panel_left",
            "action.panel_left",
            panels,
            A::TogglePanel(Panel::Left),
        ),
        cmd(
            "view.panel_right",
            "action.panel_right",
            panels,
            A::TogglePanel(Panel::Right),
        ),
        direction("view.direction.top_down", LayoutDirection::TopDown),
        direction("view.direction.bottom_up", LayoutDirection::BottomUp),
        direction("view.direction.left_right", LayoutDirection::LeftRight),
        connector("view.connector.curved", ConnectorStyle::Curved),
        connector("view.connector.elbow", ConnectorStyle::Elbow),
        connector("view.connector.tapered", ConnectorStyle::Tapered),
        minimap("view.minimap.auto", MinimapMode::Auto),
        minimap("view.minimap.on", MinimapMode::On),
        minimap("view.minimap.off", MinimapMode::Off),
        cmd("view.numbers", "action.numbers", overview, A::ToggleNumbers),
    ];
    #[cfg(debug_assertions)]
    out.extend([
        cmd(
            "view.load_sample",
            "action.load_sample",
            "group.debug",
            A::LoadSampleMap,
        ),
        cmd(
            "view.source_icons",
            "action.source_icons",
            "group.debug",
            A::ToggleSourceIcons,
        ),
    ]);
    out
}
