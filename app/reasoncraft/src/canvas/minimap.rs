//! The overview minimap: a small picture of the whole map in the canvas's
//! bottom-right corner, with the part in view outlined. A press or drag in
//! it centres the view on that point. It is a screen overlay, never part
//! of a scene, so exports never show it.

use eframe::egui::{
    self, Color32, Pos2, Rect, Role, Sense, Stroke, StrokeKind, Ui, Vec2, WidgetInfo, vec2,
};
use reasoncraft_core::layout::IndicatorOptions;
use reasoncraft_core::{BoxKind, Document, Layout, LayoutDirection, Role as MapRole};

use super::paint::content_bounds;
use super::rating::card_colour;
use crate::doc_tab::DocTab;
use crate::i18n::t;
use crate::settings::MinimapMode;
use crate::theme::Tokens;

/// The minimap's size in screen px.
pub const SIZE: Vec2 = vec2(200.0, 140.0);
/// Its distance from the canvas's right and bottom edges.
pub const INSET: f32 = 12.0;
/// Its background: the panel colour at this opacity.
const BG_ALPHA: f32 = 0.92;
/// Room between its border and the map picture.
const PAD: f32 = 6.0;
const CORNER: f32 = 4.0;

/// One rect of the picture, in world units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MiniRect {
    pub rect: Rect,
    pub colour: Color32,
    /// Boxes are filled; card frames are outlines.
    pub filled: bool,
}

/// What the cached picture was made for: the document revision, the
/// theme, the layout direction and the marks that size the boxes.
type CacheKey = (u64, bool, LayoutDirection, IndicatorOptions);

/// The minimap's picture, kept until its [`CacheKey`] changes.
pub struct Cache {
    key: Option<CacheKey>,
    rects: Vec<MiniRect>,
    bounds: Rect,
}

impl Default for Cache {
    fn default() -> Self {
        Self {
            key: None,
            rects: Vec::new(),
            bounds: Rect::NOTHING,
        }
    }
}

impl Cache {
    /// The picture of `doc` laid out as `layout`, rebuilt when `key` differs
    /// from the one it was made for: card frames first, then boxes.
    fn get(&mut self, key: CacheKey, doc: &Document, layout: &Layout, tk: &Tokens) -> &Self {
        if self.key != Some(key) {
            self.key = Some(key);
            self.bounds = content_bounds(layout);
            self.rects = picture(doc, layout, tk);
        }
        self
    }
}

/// Card frames as outlines in their colour, then boxes filled in theirs:
/// premises their card's, others by kind.
pub fn picture(doc: &Document, layout: &Layout, tk: &Tokens) -> Vec<MiniRect> {
    let card = |id| {
        let role = layout
            .card_roles
            .get(&id)
            .copied()
            .unwrap_or(MapRole::Supports);
        card_colour(tk, role, layout.card_strength.get(&id).copied().flatten())
    };
    let mut out: Vec<MiniRect> = layout
        .cards
        .iter()
        .map(|(&id, &rect)| MiniRect {
            rect,
            colour: card(id),
            filled: false,
        })
        .collect();
    let premise_of: std::collections::HashMap<_, _> = doc
        .cards
        .values()
        .flat_map(|c| c.premises.iter().map(move |&p| (p, c.id)))
        .collect();
    out.extend(layout.boxes.iter().map(|(id, &rect)| {
        let colour = match (premise_of.get(id), doc.boxes.get(id).map(|n| &n.kind)) {
            (Some(&c), _) => card(c),
            (None, Some(BoxKind::Claim)) => tk.claim_border,
            (None, Some(BoxKind::Note)) => tk.note,
            (None, Some(BoxKind::Source { .. })) => tk.attachment_line,
            _ => tk.text_muted,
        };
        MiniRect {
            rect,
            colour,
            filled: true,
        }
    }));
    out
}

/// The minimap's screen rect in a canvas showing `viewport`.
pub fn panel_rect(viewport: Rect) -> Rect {
    Rect::from_min_size(viewport.max - SIZE - Vec2::splat(INSET), SIZE)
}

/// Whether the minimap shows in `mode`: Auto shows it only while the
/// content (`content`, world units) at `zoom` is wider or taller than
/// `viewport`.
pub fn shown(mode: MinimapMode, content: Rect, zoom: f32, viewport: Rect) -> bool {
    match mode {
        MinimapMode::On => true,
        MinimapMode::Off => false,
        MinimapMode::Auto => {
            let size = content.size() * zoom;
            size.x > viewport.width() || size.y > viewport.height()
        }
    }
}

/// World `bounds` scaled to fit, centred, inside the minimap `panel`.
#[derive(Clone, Copy, Debug)]
pub struct Mapping {
    bounds: Rect,
    centre: Pos2,
    scale: f32,
}

impl Mapping {
    pub fn new(bounds: Rect, panel: Rect) -> Self {
        let room = panel.shrink(PAD).size();
        let size = bounds.size().max(Vec2::splat(1.0));
        Self {
            bounds,
            centre: panel.center(),
            scale: (room.x / size.x).min(room.y / size.y),
        }
    }

    pub fn to_panel(&self, world: Pos2) -> Pos2 {
        self.centre + (world - self.bounds.center()) * self.scale
    }

    pub fn to_world(&self, at: Pos2) -> Pos2 {
        self.bounds.center() + (at - self.centre) / self.scale
    }

    fn rect(&self, world: Rect) -> Rect {
        Rect::from_min_max(self.to_panel(world.min), self.to_panel(world.max))
    }
}

/// Draw the minimap over the canvas `viewport` when `mode` asks for it,
/// and centre the view on the point pressed or dragged over in it.
pub(super) fn show(
    ui: &Ui,
    tab: &mut DocTab,
    layout: &Layout,
    tk: &Tokens,
    mode: MinimapMode,
    marks: IndicatorOptions,
) {
    let viewport = ui.max_rect();
    if !shown(mode, content_bounds(layout), tab.camera.zoom, viewport) {
        return;
    }
    let key = (tab.revision(), tk.dark, tab.doc.layout_direction, marks);
    let cache = tab.minimap.get(key, &tab.doc, layout, tk);
    let panel = panel_rect(viewport);
    let map = Mapping::new(cache.bounds, panel);

    let response = ui.interact(
        panel,
        ui.scope_id().with("minimap"),
        Sense::click_and_drag(),
    );
    let label = t("minimap.label");
    response.widget_info(|| WidgetInfo::labeled(Role::Image, true, &label));
    let painter = ui.painter_at(panel);
    painter.rect(
        panel,
        CORNER,
        tk.panel.gamma_multiply(BG_ALPHA),
        Stroke::new(1.0, tk.attachment_line),
        StrokeKind::Inside,
    );
    for r in &cache.rects {
        let rect = map.rect(r.rect);
        if r.filled {
            painter.rect_filled(rect, 1.0, r.colour);
        } else {
            painter.rect_stroke(rect, 1.0, Stroke::new(1.0, r.colour), StrokeKind::Middle);
        }
    }
    let cam = tab.camera;
    let view = Rect::from_min_max(
        cam.screen_to_world(viewport.min),
        cam.screen_to_world(viewport.max),
    );
    painter.rect_stroke(
        map.rect(view),
        1.0,
        Stroke::new(1.5, tk.accent),
        StrokeKind::Middle,
    );

    if response.is_pointer_button_down_on()
        && let Some(at) = response.interact_pointer_pos()
    {
        let before = tab.camera.offset;
        centre_on(tab, map.to_world(at), viewport);
        // The map was painted with the old view this frame.
        if tab.camera.offset != before {
            ui.ctx().request_repaint();
        }
    }
    response.on_hover_cursor(egui::CursorIcon::Crosshair);
}

/// Pan so world point `world` sits at the centre of `viewport`.
fn centre_on(tab: &mut DocTab, world: Pos2, viewport: Rect) {
    tab.camera.offset = viewport.center().to_vec2() - world.to_vec2() * tab.camera.zoom;
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::pos2;

    fn viewport() -> Rect {
        Rect::from_min_size(pos2(240.0, 120.0), vec2(900.0, 600.0))
    }

    #[test]
    fn minimap_auto_hidden_when_map_fits() {
        let content = Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 500.0));
        assert!(!shown(MinimapMode::Auto, content, 1.0, viewport()));
        assert!(shown(MinimapMode::On, content, 1.0, viewport()));
        // Zoomed in far enough, the same map no longer fits.
        assert!(shown(MinimapMode::Auto, content, 1.5, viewport()));
        assert!(!shown(MinimapMode::Off, content, 4.0, viewport()));
    }

    #[test]
    fn panel_sits_in_the_bottom_right_corner() {
        let p = panel_rect(viewport());
        assert_eq!(p.size(), SIZE);
        assert_eq!(p.max, viewport().max - Vec2::splat(INSET));
    }

    #[test]
    fn mapping_round_trips_and_fits_the_panel() {
        let bounds = Rect::from_min_size(pos2(-300.0, 50.0), vec2(4000.0, 900.0));
        let panel = panel_rect(viewport());
        let m = Mapping::new(bounds, panel);
        assert!(panel.shrink(PAD - 0.01).contains_rect(m.rect(bounds)));
        assert!((m.to_panel(bounds.center()) - panel.center()).length() < 1e-3);
        let p = pos2(1234.0, 345.0);
        assert!((m.to_world(m.to_panel(p)) - p).length() < 1e-2);
    }
}
