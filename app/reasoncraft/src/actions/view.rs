//! View preferences: theme, rating display, panels, the Text panel tab,
//! the outline's switches and which checks run; and moving around the
//! map: zooming, keyboard navigation, revealing the selection. None of
//! them change the document.

use reasoncraft_core::Item;
use reasoncraft_core::meta::LANGUAGES;

use super::{Action, Panel, ZOOM_MAX, ZOOM_MIN};
use crate::app::ReasonCraftApp;
use crate::canvas::input::{navigate, parking_jump};
use crate::doc_tab::DocTab;

const ZOOM_STEP: f32 = 1.25;

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
        Action::SetDefaultDocLanguage(lang) => {
            if LANGUAGES.contains(&lang.as_str()) {
                s.default_doc_language = lang;
            }
        }
        _ => unreachable!("not a view action"),
    }
}

/// Zoom, navigate and reveal in `tab`.
pub(super) fn apply_to_tab(tab: &mut DocTab, action: Action) {
    match action {
        Action::RevealSelection => tab.reveal_pending = true,
        Action::Navigate(dir) => {
            tab.selection = match tab.selection {
                Some(from) => navigate(&tab.doc, from, dir).or(Some(from)),
                None => Some(Item::Box(tab.doc.root)),
            };
            tab.reveal_pending = true;
        }
        Action::JumpParking => {
            if let Some(to) = parking_jump(&tab.doc, tab.selection) {
                tab.selection = Some(to);
                tab.reveal_pending = true;
            }
        }
        Action::ZoomIn => set_zoom(tab, tab.camera.zoom * ZOOM_STEP),
        Action::ZoomOut => set_zoom(tab, tab.camera.zoom / ZOOM_STEP),
        // The canvas fits on its next frame, with the layout of that frame.
        Action::ZoomFit => tab.fit_pending = true,
        Action::Zoom100 => set_zoom(tab, 1.0),
        Action::SetZoom(z) => set_zoom(tab, z),
        _ => unreachable!("not a zoom or navigation action"),
    }
}

/// Zoom around the centre of the view (or the world origin before the
/// canvas has been shown), clamped to the zoom range.
fn set_zoom(tab: &mut DocTab, zoom: f32) {
    let centre = tab
        .viewport
        .map_or(eframe::egui::Pos2::ZERO, |v| v.center());
    let factor = zoom.clamp(ZOOM_MIN, ZOOM_MAX) / tab.camera.zoom;
    tab.camera.zoom_at(centre, factor);
}
