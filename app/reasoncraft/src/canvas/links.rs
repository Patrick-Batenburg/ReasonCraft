//! The link glyph of a box with a link: a small chain on the box's bottom
//! edge, near its right end. A click opens the link; the tooltip shows it.

use eframe::egui::{self, Rect, Role, Sense, Ui, WidgetInfo, pos2};
use reasoncraft_core::Layout;

use super::paint::Pass;
use super::plus_menu::BUTTON_PX;
use super::rating::MARKER_MIN_ZOOM;
use crate::actions::Action;
use crate::doc_tab::DocTab;
use crate::i18n::{FluentArgs, t_args};
use crate::shell::icons::{self, Icon};

/// The glyph's size, in world px.
pub const LINK_GLYPH: f32 = 12.0;
/// Gap between the glyph and the box's right edge, in world px.
const RIGHT_INSET: f32 = 20.0;
/// Room kept free, in screen px, for the "+" button centred on the
/// selected box's bottom-right corner.
const PLUS_CLEARANCE: f32 = BUTTON_PX / 2.0 + 6.0;

/// The glyph of the box at `world` (world units). It sits on the bottom
/// edge, so its upper half lies in the box's bottom padding and never
/// covers text, and left of the "+" button at every zoom.
pub fn link_rect(world: Rect, zoom: f32) -> Rect {
    let right = world.right() - RIGHT_INSET.max(PLUS_CLEARANCE / zoom);
    Rect::from_min_max(
        pos2(right - LINK_GLYPH, world.bottom() - LINK_GLYPH / 2.0),
        pos2(right, world.bottom() + LINK_GLYPH / 2.0),
    )
}

/// The glyph's accessible name, e.g. "Open link: https://example.org".
pub fn link_label(url: &str) -> String {
    let mut args = FluentArgs::new();
    args.set("url", url.to_owned());
    t_args("link.open", &args)
}

impl Pass<'_> {
    /// The chain glyph of a box at `world` that has a link.
    pub(super) fn link_glyph(&self, world: Rect) {
        if self.cam.zoom < MARKER_MIN_ZOOM {
            return;
        }
        let r = self.screen(link_rect(world, self.cam.zoom));
        if !self.visible(r) {
            return;
        }
        let tk = self.t;
        self.painter
            .circle_filled(r.center(), r.width() * 0.62, tk.box_fill);
        icons::paint(self.painter, r, Icon::Link, tk.accent, tk.box_fill, tk);
    }
}

/// Make each visible glyph a button that opens its box's link. They sit
/// above the canvas and take drags too, so a press on a glyph never
/// selects, drags the box or pans.
pub(super) fn glyphs(ui: &Ui, tab: &DocTab, layout: &Layout, pending: &mut Vec<Action>) {
    let zoom = tab.camera.zoom;
    if zoom < MARKER_MIN_ZOOM {
        return;
    }
    let clip = ui.clip_rect();
    for (&id, &world) in &layout.boxes {
        let Some(url) = tab.doc.boxes.get(&id).and_then(|n| n.link.as_deref()) else {
            continue;
        };
        let rect = tab
            .camera
            .world_rect_to_screen(link_rect(world, zoom))
            .expand(2.0);
        if !clip.intersects(rect) {
            continue;
        }
        let label = link_label(url);
        let response = ui.interact(
            rect,
            ui.scope_id().with(("link_glyph", id)),
            Sense::click_and_drag(),
        );
        response.widget_info(|| WidgetInfo::labeled(Role::Link, true, &label));
        let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
        if response.on_hover_text(url).clicked() {
            pending.push(Action::OpenLink(id));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::vec2;
    use reasoncraft_core::layout::BOX_TEXT_PAD;

    #[test]
    fn glyph_stays_clear_of_text_and_plus_button() {
        let world = Rect::from_min_size(pos2(100.0, 50.0), vec2(156.0, 40.0));
        let text = world.shrink(BOX_TEXT_PAD);
        for zoom in [MARKER_MIN_ZOOM, 0.5, 1.0, 2.0, 4.0] {
            let glyph = link_rect(world, zoom);
            assert_eq!(glyph.size(), vec2(LINK_GLYPH, LINK_GLYPH));
            assert_eq!(glyph.center().y, world.bottom(), "on the bottom edge");
            assert!(glyph.top() >= text.bottom(), "{zoom}: covers text");
            assert!(glyph.left() > world.left(), "{zoom}: inside the box");
            // The "+" button is BUTTON_PX screen px, centred on the corner.
            let plus_left = world.right() - BUTTON_PX / 2.0 / zoom;
            assert!(glyph.right() < plus_left, "{zoom}: under the + button");
        }
    }
}
