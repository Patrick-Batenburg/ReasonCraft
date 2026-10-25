//! View preferences: theme, rating display, panels, the Text panel tab,
//! the outline's switches and which checks run; and zooming.
//! None of them change the document or end "Connect to…".

use super::{Action, Panel, ZOOM_MAX, ZOOM_MIN};
use crate::app::ReasonCraftApp;
use crate::doc_tab::DocTab;

pub(super) fn apply(app: &mut ReasonCraftApp, action: Action) {
    let s = &mut app.settings;
    match action {
        Action::SetTheme(choice) => s.theme = choice,
        Action::ToggleMarkers => s.show_markers = !s.show_markers,
        Action::ToggleShading => s.eval_shading = !s.eval_shading,
        Action::TogglePanel(Panel::Left) => app.left_panel = !app.left_panel,
        Action::TogglePanel(Panel::Right) => app.right_panel = !app.right_panel,
        Action::SetTextTab(tab) => s.text_tab = tab,
        Action::ShowPanel(Panel::Left) => app.left_panel = true,
        Action::ShowPanel(Panel::Right) => app.right_panel = true,
        Action::SetOutlineOption(toggle, on) => s.set_outline_toggle(toggle, on),
        Action::SetCheckEnabled(kind, on) => s.set_check_enabled(kind, on),
        _ => unreachable!("not a view action"),
    }
}

/// Zoom around the centre of the view (or the world origin before the
/// canvas has been shown), clamped to the zoom range.
pub(super) fn set_zoom(tab: &mut DocTab, zoom: f32) {
    let centre = tab
        .viewport
        .map_or(eframe::egui::Pos2::ZERO, |v| v.center());
    let factor = zoom.clamp(ZOOM_MIN, ZOOM_MAX) / tab.camera.zoom;
    tab.camera.zoom_at(centre, factor);
}
