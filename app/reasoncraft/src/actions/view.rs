//! View preferences: theme, interface language and scale, connector style, rating
//! display, minimap, numbering, panels, the Text panel tab, the outline's
//! switches and which checks run; moving around the map: zooming, keyboard
//! navigation, revealing the selection; and the document's layout
//! direction, the only one that changes the document.

use reasoncraft_core::meta::LANGUAGES;
use reasoncraft_core::{Item, LayoutDirection};

use super::{Dir, Panel};
use crate::app::ReasonCraftApp;
use crate::canvas::input::{nav_dir, navigate, parking_jump};
use crate::doc_tab::DocTab;
use crate::i18n::UI_LANGUAGES;

const ZOOM_STEP: f32 = 1.25;

/// Show the interface in `lang` (`None`: the OS language); a language
/// the interface does not have is refused.
pub(super) fn set_ui_language(app: &mut ReasonCraftApp, lang: Option<String>) {
    if lang.as_deref().is_none_or(|l| UI_LANGUAGES.contains(&l)) {
        app.settings.ui_language = lang;
        app.apply_ui_language();
    }
}

/// Start new documents in `lang` (`None`: the interface language); a
/// language documents cannot have is refused.
pub(super) fn set_default_doc_language(app: &mut ReasonCraftApp, lang: Option<String>) {
    if lang.as_deref().is_none_or(|l| LANGUAGES.contains(&l)) {
        app.settings.default_doc_language = lang;
    }
}

/// Turn spellcheck on or off; off ends every tab's spelling pass.
pub(super) fn set_spellcheck(app: &mut ReasonCraftApp, on: bool) {
    app.settings.spellcheck = on;
    if !on {
        for tab in &mut app.tabs {
            tab.spelling = None;
        }
    }
}

/// The flag that shows `panel`.
pub(super) fn panel(app: &mut ReasonCraftApp, panel: Panel) -> &mut bool {
    match panel {
        Panel::Left => &mut app.left_panel,
        Panel::Right => &mut app.right_panel,
    }
}

/// Move the selection with an arrow key (`screen`: its on-screen
/// direction); with nothing selected, select the root.
pub(super) fn navigate_tab(tab: &mut DocTab, screen: Dir) {
    let dir = nav_dir(screen, tab.doc.layout_direction);
    tab.selection = match tab.selection {
        Some(from) => navigate(&tab.doc, from, dir).or(Some(from)),
        None => Some(Item::Box(tab.doc.root)),
    };
    tab.reveal_pending = true;
}

/// Jump between the main tree's root and the first parked item.
pub(super) fn jump_parking(tab: &mut DocTab) {
    if let Some(to) = parking_jump(&tab.doc, tab.selection) {
        tab.selection = Some(to);
        tab.reveal_pending = true;
    }
}

/// Zoom in (`true`) or out one step.
pub(super) fn zoom_step(tab: &mut DocTab, zoom_in: bool) {
    let zoom = tab.camera.zoom;
    set_zoom(
        tab,
        if zoom_in {
            zoom * ZOOM_STEP
        } else {
            zoom / ZOOM_STEP
        },
    );
}

/// Lay the document out in direction `d` (one history step; none when it
/// already is), then show the selection, or all of the map.
pub(super) fn set_layout_direction(tab: &mut DocTab, d: LayoutDirection) {
    if tab.edit(|doc| doc.set_layout_direction(d)).is_some() {
        // The map moves under the view.
        if tab.selection.is_some() {
            tab.reveal_pending = true;
        } else {
            tab.fit_pending = true;
        }
    }
}

/// Zoom around the centre of the view (or the world origin before the
/// canvas has been shown), clamped to the zoom range.
pub(super) fn set_zoom(tab: &mut DocTab, zoom: f32) {
    let centre = tab
        .viewport
        .map_or(eframe::egui::Pos2::ZERO, |v| v.center());
    tab.camera.zoom_to(centre, zoom);
}
