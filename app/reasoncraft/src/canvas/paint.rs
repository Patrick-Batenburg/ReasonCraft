//! Style-D painting of a laid-out map (spec §5).
//!
//! Everything is laid out in world units and drawn through the [`Camera`].
//! Stroke widths, arrowheads and bars are in screen px so they stay visible
//! at every zoom. Draw order: background, parking area, connectors, card
//! frames, boxes, labels, selection.

use std::collections::HashMap;

use eframe::egui::{
    Align2, Color32, FontId, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, Vec2, epaint, pos2,
    vec2,
};
use reasoncraft_core::connector::{
    ARROW_PX, EndMarker, STUB_PX, Segment, Strength, curved_path, end_marker, stroke_px,
};
use reasoncraft_core::{CardId, Connector, ConnectorKind, Document, Item, Layout, NodeId, Role};

use super::Camera;
use super::boxes::{BOX_RADIUS, box_radius};
use crate::i18n::t;
use crate::theme::Tokens;

pub(super) const FRAME_RADIUS: f32 = 10.0;
const FRAME_FILL_ALPHA: f32 = 0.12;
const FRAME_STROKE_ALPHA: f32 = 0.55;
const LABEL_SIZE: f32 = 10.5;
/// Opposes/rebuts end bar: length and thickness in screen px.
const BAR_PX: f32 = 11.0;
const BAR_THICKNESS: f32 = 2.4;
const SELECT_OFFSET: f32 = 2.0;
const SELECT_WIDTH: f32 = 2.0;
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

fn role_label(role: Role) -> String {
    t(match role {
        Role::Supports => "role.supports",
        Role::Opposes => "role.opposes",
        Role::Rebuts => "role.rebuts",
    })
}

fn connector_colour(t: &Tokens, kind: ConnectorKind) -> Color32 {
    match kind {
        ConnectorKind::Supports => t.reason,
        ConnectorKind::Opposes => t.objection,
        ConnectorKind::Rebuts => t.rebuttal,
        ConnectorKind::Neutral => t.text_muted,
        ConnectorKind::Attachment => t.attachment_line,
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
pub(super) fn screen_font(size: f32, zoom: f32) -> Option<FontId> {
    let px = (size * zoom * 2.0).floor() / 2.0;
    (px >= MIN_TEXT_PX).then(|| FontId::proportional(px))
}

pub fn paint(
    painter: &Painter,
    layout: &Layout,
    doc: &Document,
    cam: &Camera,
    tokens: &Tokens,
    selection: Option<Item>,
    editing: Option<NodeId>,
) {
    let p = Pass {
        painter,
        cam,
        t: tokens,
        clip: painter.clip_rect(),
    };
    p.dots();
    if let Some(frame) = parking_frame(layout) {
        p.parking(frame);
    }
    for c in &layout.connectors {
        p.connector(c);
    }
    for (&id, &rect) in &layout.cards {
        p.card_frame(rect, doc.card_role(id));
    }
    let owner: HashMap<NodeId, CardId> = doc
        .cards
        .values()
        .flat_map(|c| c.premises.iter().map(|&b| (b, c.id)))
        .collect();
    for (id, &rect) in &layout.boxes {
        if let Some(node) = doc.boxes.get(id) {
            let role = owner.get(id).map(|&c| doc.card_role(c));
            let text = (editing != Some(*id)).then_some(node.text.as_str());
            p.box_(rect, &node.kind, text, role, *id == doc.root);
        }
    }
    for (&id, &anchor) in &layout.card_labels {
        p.card_label(anchor, doc.card_role(id));
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
}

/// One paint pass: shared state for the drawing helpers. Box painting
/// lives in [`super::boxes`].
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

    fn parking(&self, frame: Rect) {
        let r = self.screen(frame);
        if !self.visible(r) {
            return;
        }
        let stroke = Stroke::new(1.0, self.t.text_muted);
        let path = rounded_rect_path(r, FRAME_RADIUS * self.cam.zoom);
        self.painter
            .extend(Shape::dashed_line(&path, stroke, 6.0, 4.0));
        if let Some(font) = screen_font(12.0, self.cam.zoom) {
            let at = r.min + vec2(12.0, 8.0) * self.cam.zoom;
            self.painter.text(
                at,
                Align2::LEFT_TOP,
                t("parking.title"),
                font,
                self.t.text_muted,
            );
        }
    }

    fn connector(&self, c: &Connector) {
        let from = self.cam.world_to_screen(c.from);
        let to = self.cam.world_to_screen(c.to);
        if !self.visible(Rect::from_two_pos(from, to)) {
            return;
        }
        let colour = connector_colour(self.t, c.kind);
        let stroke = Stroke::new(stroke_px(c.kind, Strength::Normal, self.cam.zoom), colour);
        // A full stub when there is room, otherwise half the drop.
        let drop = from.y - to.y;
        let stub = if drop >= 2.0 * STUB_PX {
            STUB_PX
        } else {
            (drop / 2.0).max(0.0)
        };
        for seg in curved_path(from, to, stub) {
            match seg {
                Segment::Cubic(points) => {
                    self.painter
                        .add(epaint::CubicBezierShape::from_points_stroke(
                            points,
                            false,
                            Color32::TRANSPARENT,
                            stroke,
                        ));
                }
                Segment::Line([a, b]) => self.stub(a, b, stroke, end_marker(c.kind)),
            }
        }
    }

    /// The straight end of a connector from `a` up to the port `b`, with its
    /// marker oriented along it.
    fn stub(&self, a: Pos2, b: Pos2, stroke: Stroke, marker: EndMarker) {
        let d = b - a;
        let dir = if d.length() > 1e-3 {
            d.normalized()
        } else {
            vec2(0.0, -1.0)
        };
        let side = dir.rot90();
        match marker {
            EndMarker::Arrow => {
                // Stop the line just inside the arrowhead so it cannot poke out of the tip.
                let len = (b - a).length() - (ARROW_PX - 1.0);
                if len > 0.0 {
                    self.painter.line_segment([a, a + dir * len], stroke);
                }
                let base = b - dir * ARROW_PX;
                let half = ARROW_PX * 0.5;
                self.painter.add(Shape::convex_polygon(
                    vec![b, base + side * half, base - side * half],
                    stroke.color,
                    Stroke::NONE,
                ));
            }
            EndMarker::Bar => {
                // The bar's far edge touches the port.
                let centre = b - dir * (BAR_THICKNESS / 2.0);
                self.painter.line_segment([a, centre], stroke);
                let half = side * (BAR_PX / 2.0);
                self.painter.line_segment(
                    [centre - half, centre + half],
                    Stroke::new(BAR_THICKNESS, stroke.color),
                );
            }
            EndMarker::None => {
                self.painter.line_segment([a, b], stroke);
            }
        }
    }

    fn card_frame(&self, world: Rect, role: Role) {
        let r = self.screen(world);
        if !self.visible(r) {
            return;
        }
        let colour = role_colour(self.t, role);
        let radius = FRAME_RADIUS * self.cam.zoom;
        self.painter
            .rect_filled(r, radius, colour.gamma_multiply(FRAME_FILL_ALPHA));
        self.painter.rect_stroke(
            r,
            radius,
            Stroke::new(1.0, colour.gamma_multiply(FRAME_STROKE_ALPHA)),
            StrokeKind::Inside,
        );
    }

    fn card_label(&self, anchor: Pos2, role: Role) {
        let at = self.cam.world_to_screen(anchor);
        if !self.clip.expand(100.0).contains(at) {
            return;
        }
        if let Some(font) = screen_font(LABEL_SIZE, self.cam.zoom) {
            self.painter.text(
                at,
                Align2::RIGHT_CENTER,
                role_label(role),
                font,
                role_colour(self.t, role),
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

/// Points along the outline of `r` with rounded corners, closed.
fn rounded_rect_path(r: Rect, radius: f32) -> Vec<Pos2> {
    let radius = radius.min(r.width() / 2.0).min(r.height() / 2.0);
    let corners = [
        (r.right_top() + vec2(-radius, radius), -90.0_f32),
        (r.right_bottom() + vec2(-radius, -radius), 0.0),
        (r.left_bottom() + vec2(radius, -radius), 90.0),
        (r.left_top() + vec2(radius, radius), 180.0),
    ];
    let mut points = Vec::with_capacity(4 * 7 + 1);
    for (centre, start) in corners {
        for i in 0..=6 {
            let a = (start + i as f32 * 15.0).to_radians();
            points.push(centre + Vec2::angled(a) * radius);
        }
    }
    points.push(points[0]);
    points
}
