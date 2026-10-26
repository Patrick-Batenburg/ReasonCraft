//! View preferences: theme, connector style, rating display, minimap,
//! numbering, panels, the Text panel tab,
//! the outline's switches and which checks run; moving around the map:
//! zooming, keyboard navigation, revealing the selection; and the
//! document's layout direction, the only one that changes the document.

use reasoncraft_core::Item;
use reasoncraft_core::meta::LANGUAGES;

use super::{Action, Panel};
use crate::app::ReasonCraftApp;
use crate::canvas::input::{nav_dir, navigate, parking_jump};
use crate::doc_tab::DocTab;

const ZOOM_STEP: f32 = 1.25;

pub(super) fn apply(app: &mut ReasonCraftApp, action: Action) {
    let s = &mut app.settings;
    match action {
        Action::SetTheme(choice) => s.theme = choice,
        Action::SetConnectorStyle(style) => s.connector_style = style,
        Action::ToggleMarkers => s.show_markers = !s.show_markers,
        Action::SetMinimap(mode) => s.minimap = mode,
        Action::ToggleNumbers => s.show_numbers = !s.show_numbers,
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

/// Zoom, navigate, reveal and set the layout direction in `tab`.
pub(super) fn apply_to_tab(tab: &mut DocTab, action: Action) {
    match action {
        Action::RevealSelection => tab.reveal_pending = true,
        Action::Navigate(screen) => {
            let dir = nav_dir(screen, tab.doc.layout_direction);
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
        Action::ZoomFitSelection => tab.fit_selection_pending = tab.selection.is_some(),
        Action::Zoom100 => set_zoom(tab, 1.0),
        Action::SetZoom(z) => set_zoom(tab, z),
        Action::SetLayoutDirection(d) => {
            if tab.edit(|doc| doc.set_layout_direction(d)).is_some() {
                // The map moves under the view: show the selection, or all of it.
                if tab.selection.is_some() {
                    tab.reveal_pending = true;
                } else {
                    tab.fit_pending = true;
                }
            }
        }
        _ => unreachable!("not a zoom, navigation or direction action"),
    }
}

/// Zoom around the centre of the view (or the world origin before the
/// canvas has been shown), clamped to the zoom range.
fn set_zoom(tab: &mut DocTab, zoom: f32) {
    let centre = tab
        .viewport
        .map_or(eframe::egui::Pos2::ZERO, |v| v.center());
    tab.camera.zoom_to(centre, zoom);
}
