//! Turning a laid-out document into a [`Scene`] (spec §5, style D).
//!
//! Draw order: parking area, connectors, card frames, boxes (each with its
//! rating marker and number), card labels. Sizes the screen keeps constant in px
//! (strokes, arrowheads, bars) are multiplied by `px`, the world size of a
//! screen px at the zoom the scene is built for.

use std::collections::{HashMap, HashSet};

use eframe::egui::{Align, Color32, Pos2, Rect, Vec2, pos2, vec2};
use reasoncraft_core::{CardStrength, Document, Item, Layout, NodeId, Role};

use super::{PathEl, Scene, SceneOptions, Shape, StrokeSpec, TextRows};
use crate::canvas::paint::{FRAME_RADIUS, parking_frame, screen_font};
use crate::canvas::rating::{
    MARKER_MIN_ZOOM, Mark, card_colour, frame_alpha, frame_border, mark_for, mark_shapes,
    marker_circle, rating_dots, rating_label,
};
use crate::i18n::t_lang;

const FRAME_STROKE_ALPHA: f32 = 0.55;
const LABEL_SIZE: f32 = 10.5;
const PARKING_TITLE_SIZE: f32 = 12.0;
/// Screen px kept around the view when culling, for strokes and markers.
const CULL_MARGIN: f32 = 4.0;
/// Card labels are kept this many screen px beyond the view.
const LABEL_MARGIN: f32 = 100.0;

/// The scene of `doc` laid out as `layout`, with text rows from `fonts`
/// (the measure the layout used). Its bounds cover every shape.
pub fn build_scene(
    doc: &Document,
    layout: &Layout,
    opts: &SceneOptions,
    fonts: &dyn TextRows,
) -> Scene {
    let mut b = Builder {
        shapes: Vec::new(),
        opts,
        fonts,
        px: 1.0 / opts.zoom,
    };
    let keep = Filter::new(doc, opts);
    if opts.include_parking
        && opts.only.is_none()
        && let Some(frame) = parking_frame(layout)
    {
        b.parking(frame);
    }
    let turns = b.elbow_turns(&layout.connectors);
    for (i, c) in layout.connectors.iter().enumerate() {
        if keep.item(c.child) && Some(c.child) != opts.only {
            b.connector(c, turns.get(i).copied());
        }
    }
    let rating = |id| {
        let role = layout
            .card_roles
            .get(&id)
            .copied()
            .unwrap_or(Role::Supports);
        (role, layout.card_strength.get(&id).copied().flatten())
    };
    for (&id, &rect) in &layout.cards {
        if keep.item(Item::Card(id)) {
            let (role, strength) = rating(id);
            b.card_frame(rect, role, strength);
        }
    }
    // Premise borders take their card's colour (grey when it is rated nil).
    let premise_colour: HashMap<NodeId, Color32> = doc
        .cards
        .values()
        .flat_map(|c| {
            let (role, strength) = rating(c.id);
            let colour = card_colour(&opts.tokens, role, strength);
            c.premises.iter().map(move |&b| (b, colour))
        })
        .collect();
    let markers = opts.show_markers && opts.zoom >= MARKER_MIN_ZOOM;
    for (id, &rect) in &layout.boxes {
        let Some(node) = doc.boxes.get(id) else {
            continue;
        };
        if !keep.item(Item::Box(*id)) {
            continue;
        }
        let row = layout.indicator_row(*id);
        let border = premise_colour.get(id).copied();
        b.box_(rect, row, node, border, *id == doc.root);
        if markers && let Some(mark) = mark_for(node) {
            b.marker(rect, row, mark);
        }
        if opts.numbers
            && row > 0.0
            && let Some(number) = layout.numbers.get(id)
        {
            b.number(rect, row, number);
        }
    }
    for (&id, &anchor) in &layout.card_labels {
        if keep.item(Item::Card(id)) {
            let (role, strength) = rating(id);
            b.card_label(anchor, role, strength);
        }
    }
    let shapes = b.shapes;
    let bounds = if shapes.is_empty() {
        Rect::ZERO
    } else {
        shapes
            .iter()
            .fold(Rect::NOTHING, |r, s| r.union(s.bounds()))
    };
    Scene { shapes, bounds }
}

/// The items a scene shows: a selection's subtree, or everything but the
/// parking area when it is left out.
struct Filter {
    only: Option<HashSet<Item>>,
    parked: HashSet<Item>,
}

impl Filter {
    fn new(doc: &Document, opts: &SceneOptions) -> Self {
        let tree = |root: Item| {
            let mut set: HashSet<Item> = doc.descendants(root).into_iter().collect();
            set.insert(root);
            set
        };
        let only = opts.only.map(tree);
        let parked = if opts.include_parking || only.is_some() {
            HashSet::new()
        } else {
            doc.parking.iter().flat_map(|&p| tree(p)).collect()
        };
        Self { only, parked }
    }

    fn item(&self, item: Item) -> bool {
        self.only.as_ref().is_none_or(|s| s.contains(&item)) && !self.parked.contains(&item)
    }
}

/// A scene being built.
pub(super) struct Builder<'a> {
    pub shapes: Vec<Shape>,
    pub opts: &'a SceneOptions,
    pub fonts: &'a dyn TextRows,
    /// World units per screen px.
    pub px: f32,
}

/// A path through `pts`.
pub(super) fn polyline(pts: &[Pos2]) -> Vec<PathEl> {
    pts.iter()
        .enumerate()
        .map(|(i, &p)| {
            if i == 0 {
                PathEl::MoveTo(p)
            } else {
                PathEl::LineTo(p)
            }
        })
        .collect()
}

impl Builder<'_> {
    /// Whether world rect `r` is in the view (always, without one).
    pub(super) fn visible(&self, r: Rect) -> bool {
        self.opts
            .view
            .is_none_or(|v| v.intersects(r.expand(CULL_MARGIN * self.px)))
    }

    /// The world width of one row of `text` at `size` as drawn at the
    /// scene's zoom: screen fonts come in half-px steps and small sizes
    /// are hinted, so this is measured at the screen size.
    pub(super) fn text_width(&self, text: &str, size: f32) -> f32 {
        self.fonts.text_width(text, self.screen_size(size)) / self.opts.zoom
    }

    /// The row height of text at `size` as drawn at the scene's zoom (see
    /// [`Self::text_width`]), for centring one row on a point.
    pub(super) fn row_height(&self, size: f32) -> f32 {
        self.fonts.line_height(self.screen_size(size)) / self.opts.zoom
    }

    fn screen_size(&self, size: f32) -> f32 {
        let zoom = self.opts.zoom;
        screen_font(size, zoom).map_or(size * zoom, |f| f.size)
    }

    pub(super) fn line(&mut self, pts: &[Pos2], stroke: StrokeSpec) {
        self.shapes.push(Shape::Path {
            points: polyline(pts),
            stroke: Some(stroke),
            fill: None,
            dashed: false,
        });
    }

    fn parking(&mut self, frame: Rect) {
        if !self.visible(frame) {
            return;
        }
        let tk = self.opts.tokens;
        self.shapes.push(Shape::Path {
            points: polyline(&rounded_rect_path(frame, FRAME_RADIUS)),
            stroke: Some((self.px, tk.text_muted)),
            fill: None,
            dashed: true,
        });
        self.shapes.push(Shape::Text {
            origin: frame.min + vec2(12.0, 8.0),
            rows: vec![t_lang(self.opts.lang, "parking.title")],
            size: PARKING_TITLE_SIZE,
            bold: false,
            color: tk.text_muted,
            align: Align::Min,
            line_height: self.fonts.line_height(PARKING_TITLE_SIZE),
        });
    }

    fn card_frame(&mut self, world: Rect, role: Role, strength: Option<CardStrength>) {
        if !self.visible(world) {
            return;
        }
        let shading = self.opts.eval_shading;
        let colour = card_colour(&self.opts.tokens, role, strength);
        self.shapes.push(Shape::Rect {
            rect: world,
            radius: FRAME_RADIUS,
            fill: colour.gamma_multiply(frame_alpha(strength, shading)),
            stroke: Some((
                frame_border(strength, shading) * self.px,
                colour.gamma_multiply(FRAME_STROKE_ALPHA),
            )),
        });
    }

    /// The rating label, right-aligned and vertically centred on `anchor`,
    /// led by its strength dots drawn as circles.
    fn card_label(&mut self, anchor: Pos2, role: Role, strength: Option<CardStrength>) {
        if let Some(view) = self.opts.view
            && !view.expand(LABEL_MARGIN * self.px).contains(anchor)
        {
            return;
        }
        if screen_font(LABEL_SIZE, self.opts.zoom).is_none() {
            return;
        }
        let colour = card_colour(&self.opts.tokens, role, strength);
        let text = rating_label(self.opts.lang, role, strength);
        let dots = rating_dots(strength);
        let line_height = self.fonts.line_height(LABEL_SIZE);
        let width = self.text_width(&text, LABEL_SIZE);
        self.shapes.push(Shape::Text {
            origin: anchor - vec2(0.0, self.row_height(LABEL_SIZE) / 2.0),
            rows: vec![text],
            size: LABEL_SIZE,
            bold: false,
            color: colour,
            align: Align::Max,
            line_height,
        });
        // The dots scale with the font size the text is drawn at.
        let size = self.screen_size(LABEL_SIZE) / self.opts.zoom;
        let r = size * 0.24;
        let mut x = anchor.x - width - size * 0.4 - r;
        for _ in 0..dots {
            self.shapes.push(Shape::Circle {
                center: pos2(x, anchor.y),
                radius: r,
                fill: Some(colour),
                stroke: None,
            });
            x -= size * 0.62;
        }
    }

    /// A ✓ / ? / × marker in a small circle in the indicator row (`row`
    /// high) of the box at `world`.
    fn marker(&mut self, world: Rect, row: f32, mark: Mark) {
        let (centre, radius) = marker_circle(world, row);
        if !self.visible(Rect::from_center_size(centre, Vec2::splat(2.0 * radius))) {
            return;
        }
        let shapes = mark_shapes(centre, radius, mark, &self.opts.tokens, self.px);
        self.shapes.extend(shapes);
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
