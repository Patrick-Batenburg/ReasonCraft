//! Painting of a laid-out map (spec §5): the dot grid, the document's
//! [`crate::scene`] drawn through the [`Camera`], and the screen-only
//! overlays on top: find-hit outlines, selection ring, comment badges and
//! link glyphs.

use std::collections::{HashMap, HashSet};

use eframe::egui::{Color32, FontId, Painter, Rect, Shape, Stroke, StrokeKind, pos2, vec2};
use reasoncraft_core::{ConnectorStyle, Document, Item, Layout, NodeId, Role};

use super::Camera;
use super::images::ImageCache;
use crate::scene::{BOX_RADIUS, SceneOptions, TextRows, box_radius, build_scene, paint_scene};
use crate::shell::link_modal::safe_link;
use crate::theme::Tokens;

pub const FRAME_RADIUS: f32 = 10.0;
const SELECT_OFFSET: f32 = 2.0;
const SELECT_WIDTH: f32 = 2.0;
/// The find-hit outline sits right on the box edge, inside the selection ring.
const FIND_HIT_WIDTH: f32 = 2.0;
const DOT_SPACING: f32 = 24.0;
/// Dots closer than this many screen px are thinned out.
const MIN_DOT_SPACING: f32 = 14.0;
const PARKING_PAD: f32 = 24.0;
const PARKING_HEADING: f32 = 26.0;
/// Text smaller than this many screen px is not drawn.
const MIN_TEXT_PX: f32 = 3.0;

pub fn role_colour(t: &Tokens, role: Role) -> Color32 {
    match role {
        Role::Supports => t.reason,
        Role::Opposes => t.objection,
        Role::Rebuts => t.rebuttal,
    }
}

/// The dashed frame around the parking area, heading included (world units).
pub fn parking_frame(layout: &Layout) -> Option<Rect> {
    layout.parking_bounds.map(|b| {
        Rect::from_min_max(
            b.min - vec2(PARKING_PAD, PARKING_PAD + PARKING_HEADING),
            b.max + vec2(PARKING_PAD, PARKING_PAD),
        )
    })
}

/// Everything that is drawn, in world units: the layout plus the parking frame.
pub fn content_bounds(layout: &Layout) -> Rect {
    parking_frame(layout).map_or(layout.bounds, |p| p.union(layout.bounds))
}

/// Screen font size for a world size, rounded down to half pixels so zooming
/// does not rasterise glyphs at every possible size. Rounding down keeps
/// rows inside their box.
pub fn screen_font(size: f32, zoom: f32) -> Option<FontId> {
    let px = (size * zoom * 2.0).floor() / 2.0;
    (px >= MIN_TEXT_PX).then(|| FontId::proportional(px))
}

/// Display settings that affect painting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaintOptions {
    /// Draw ✓ / ? / × markers on rated boxes.
    pub show_markers: bool,
    /// Tint and weight cards and connectors by their rating.
    pub eval_shading: bool,
    /// How connectors are drawn.
    pub connector_style: ConnectorStyle,
    /// Show each numbered box's outline number.
    pub numbers: bool,
}

/// Paint the map: the dot grid, the scene of the document, then the
/// screen-only overlays (outlines of the boxes in `find_hits`, selection
/// ring, comment badges, link glyphs) on top, so the selection ring never
/// hides them. `fonts` is the measure the layout used; `leads` moves
/// glyphs and badges past box numbers.
#[allow(clippy::too_many_arguments)]
pub fn paint(
    painter: &Painter,
    layout: &Layout,
    doc: &Document,
    cam: &Camera,
    tokens: &Tokens,
    selection: Option<Item>,
    editing: Option<NodeId>,
    opts: PaintOptions,
    images: &ImageCache,
    fonts: &dyn TextRows,
    leads: &HashMap<NodeId, f32>,
    find_hits: &HashSet<NodeId>,
) {
    let p = Pass {
        painter,
        cam,
        t: tokens,
        clip: painter.clip_rect(),
    };
    p.dots();
    let view = Rect::from_min_max(
        cam.screen_to_world(p.clip.min),
        cam.screen_to_world(p.clip.max),
    );
    let scene_opts = SceneOptions {
        tokens: *tokens,
        show_markers: opts.show_markers,
        numbers: opts.numbers,
        eval_shading: opts.eval_shading,
        for_export: false,
        include_parking: true,
        only: None,
        view: Some(view),
        zoom: cam.zoom,
        editing,
        connector_style: opts.connector_style,
        lang: crate::i18n::language_of(&doc.meta.language),
    };
    let scene = build_scene(doc, layout, &scene_opts, fonts);
    paint_scene(painter, &scene, cam, images, tokens);

    for id in find_hits {
        if let (Some(&rect), Some(node)) = (layout.boxes.get(id), doc.boxes.get(id)) {
            p.find_hit(rect, box_radius(&node.kind));
        }
    }

    let selected = match selection {
        Some(Item::Box(b)) => layout.boxes.get(&b).map(|&r| {
            let radius = doc
                .boxes
                .get(&b)
                .map_or(BOX_RADIUS, |n| box_radius(&n.kind));
            (r, radius)
        }),
        Some(Item::Card(c)) => layout.cards.get(&c).map(|&r| (r, FRAME_RADIUS)),
        None => None,
    };
    if let Some((rect, radius)) = selected {
        p.selection(rect, radius);
    }
    for (id, &rect) in &layout.boxes {
        if let Some(node) = doc.boxes.get(id) {
            let row = layout.indicator_row(*id);
            let lead = leads.get(id).copied().unwrap_or(0.0);
            p.comment_badge(rect, row, lead, &node.comments);
            if let Some(link) = &node.link {
                p.link_glyph(rect, row, lead, safe_link(link).is_some());
            }
        }
    }
}

/// One paint pass of the screen-only parts: shared state for the drawing
/// helpers.
pub(super) struct Pass<'a> {
    pub painter: &'a Painter,
    pub cam: &'a Camera,
    pub t: &'a Tokens,
    pub clip: Rect,
}

impl Pass<'_> {
    pub(super) fn screen(&self, r: Rect) -> Rect {
        self.cam.world_rect_to_screen(r)
    }

    pub(super) fn visible(&self, r: Rect) -> bool {
        self.clip.intersects(r.expand(4.0))
    }

    fn dots(&self) {
        let mut step = DOT_SPACING * self.cam.zoom;
        while step < MIN_DOT_SPACING {
            step *= 2.0;
        }
        let radius = (1.1 * self.cam.zoom).clamp(0.8, 1.5);
        let origin = self.cam.offset;
        let first = |min: f32, o: f32| min - (min - o).rem_euclid(step);
        let mut y = first(self.clip.top(), origin.y);
        let mut shapes = Vec::new();
        while y <= self.clip.bottom() {
            let mut x = first(self.clip.left(), origin.x);
            while x <= self.clip.right() {
                shapes.push(Shape::circle_filled(pos2(x, y), radius, self.t.canvas_dot));
                x += step;
            }
            y += step;
        }
        self.painter.extend(shapes);
    }

    fn find_hit(&self, world: Rect, radius: f32) {
        let r = self.screen(world);
        if self.visible(r) {
            self.painter.rect_stroke(
                r,
                radius * self.cam.zoom,
                Stroke::new(FIND_HIT_WIDTH, self.t.find_hit),
                StrokeKind::Outside,
            );
        }
    }

    fn selection(&self, world: Rect, radius: f32) {
        let r = self
            .screen(world)
            .expand(SELECT_OFFSET + SELECT_WIDTH / 2.0);
        self.painter.rect_stroke(
            r,
            radius * self.cam.zoom + SELECT_OFFSET,
            Stroke::new(SELECT_WIDTH, self.t.accent),
            StrokeKind::Middle,
        );
    }
}
