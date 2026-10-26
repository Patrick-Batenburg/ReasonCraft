//! The link glyph of a box with a link: a small chain inside the box's
//! top-left corner, clear of the arrowheads that arrive at the box's ports
//! and of the "+" button on its bottom-right corner. A click opens the
//! link; the tooltip shows it.
//! A stored link that is not a safe web or e-mail link (a hand-edited file
//! may hold `file:` paths or UNC shares) is struck through and never opened.

use eframe::egui::{self, Rect, Role, Sense, Stroke, Ui, WidgetInfo, vec2};
use reasoncraft_core::Layout;

use super::paint::Pass;
use super::rating::{CORNER_INSET, MARKER_MIN_ZOOM};
use crate::actions::Action;
use crate::doc_tab::DocTab;
use crate::i18n::{FluentArgs, t_args};
use crate::shell::icons::{self, Icon};
use crate::shell::link_modal::{safe_link, visible_link};

/// The glyph's size, in world px.
pub const LINK_GLYPH: f32 = 12.0;
/// The glyph of the box at `world` (world units): inside its top-left
/// corner, [`CORNER_INSET`] from both edges.
pub fn link_rect(world: Rect) -> Rect {
    Rect::from_min_size(
        world.min + vec2(CORNER_INSET, CORNER_INSET),
        vec2(LINK_GLYPH, LINK_GLYPH),
    )
}

/// The glyph's accessible name for stored link `url`, e.g. "Open link:
/// https://example.org", or one saying it cannot be opened (not a safe
/// link: see [`safe_link`]).
pub fn link_label(url: &str) -> String {
    let mut args = FluentArgs::new();
    args.set("url", visible_link(url));
    let key = if safe_link(url).is_some() {
        "link.open"
    } else {
        "link.unsafe_label"
    };
    t_args(key, &args)
}

impl Pass<'_> {
    /// The chain glyph of a box at `world` that has a link. A link that
    /// will not be opened (`safe`: false) is greyed out and struck through.
    pub(super) fn link_glyph(&self, world: Rect, safe: bool) {
        if self.cam.zoom < MARKER_MIN_ZOOM {
            return;
        }
        let r = self.screen(link_rect(world));
        if !self.visible(r) {
            return;
        }
        let tk = self.t;
        self.painter
            .circle_filled(r.center(), r.width() * 0.62, tk.box_fill);
        let colour = if safe { tk.accent } else { tk.text_muted };
        icons::paint(self.painter, r, Icon::Link, colour, tk.box_fill, tk);
        if !safe {
            let stroke = Stroke::new((1.5 * self.cam.zoom).max(1.25), tk.objection);
            self.painter
                .line_segment([r.left_top(), r.right_bottom()], stroke);
        }
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
            .world_rect_to_screen(link_rect(world))
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
        // An unsafe link is never opened: the click explains why instead.
        if response.on_hover_text(visible_link(url)).clicked() {
            pending.push(Action::OpenLink(id));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::plus_menu::BUTTON_PX;
    use eframe::egui::{Vec2, pos2};

    #[test]
    fn glyph_sits_in_the_top_left_corner_clear_of_plus_button() {
        let world = Rect::from_min_size(pos2(100.0, 50.0), vec2(156.0, 40.0));
        let glyph = link_rect(world);
        assert_eq!(glyph.size(), vec2(LINK_GLYPH, LINK_GLYPH));
        assert_eq!(glyph.min, world.min + vec2(CORNER_INSET, CORNER_INSET));
        for zoom in [MARKER_MIN_ZOOM, 0.5, 1.0, 2.0, 4.0] {
            // The "+" button is BUTTON_PX screen px, centred on the corner.
            let plus = Rect::from_center_size(world.right_bottom(), Vec2::splat(BUTTON_PX / zoom));
            assert!(!glyph.intersects(plus), "{zoom}: under the + button");
        }
    }
}
