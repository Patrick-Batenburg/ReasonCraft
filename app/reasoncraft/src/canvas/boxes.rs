//! Painting of boxes: claim, premise, category, source and note.

use eframe::egui::{Align2, Color32, Rect, Shape, Stroke, StrokeKind, pos2, vec2};
use reasoncraft_core::BoxKind;
use reasoncraft_core::layout::BOX_TEXT_PAD;

use super::measure;
use super::paint::{Pass, screen_font};
use crate::i18n::t;

pub(super) const BOX_RADIUS: f32 = 7.0;
const SOURCE_RADIUS: f32 = 5.0;
const NOTE_RADIUS: f32 = 2.0;
/// Size of a note's folded corner.
const NOTE_FOLD: f32 = 10.0;
const SOURCE_TYPE_SIZE: f32 = 11.5;
/// Top of a source's citation text below the box top: padding plus the
/// icon and type row. The layout reserves this plus the bottom padding.
pub(super) const SOURCE_HEADER: f32 = 24.0;

pub(super) fn box_radius(kind: &BoxKind) -> f32 {
    match kind {
        BoxKind::Source { .. } => SOURCE_RADIUS,
        BoxKind::Note => NOTE_RADIUS,
        _ => BOX_RADIUS,
    }
}

impl Pass<'_> {
    /// A box of any kind. `border` is the colour of the card a premise sits in.
    /// `text` is `None` while the inline editor shows the text instead.
    pub(super) fn box_(
        &self,
        world: Rect,
        kind: &BoxKind,
        text: Option<&str>,
        border_colour: Option<Color32>,
        is_root: bool,
    ) {
        let r = self.screen(world);
        if !self.visible(r) {
            return;
        }
        let tk = self.t;
        let radius = box_radius(kind) * self.cam.zoom;
        let border = |c: Color32| Stroke::new(1.0, c);
        let (fill, stroke, text_colour) = match kind {
            BoxKind::Claim => (tk.box_fill, border(tk.claim_border), tk.box_text),
            BoxKind::Premise => {
                let c = border_colour.unwrap_or(tk.text_muted);
                (tk.box_fill, border(c), tk.box_text)
            }
            BoxKind::Category => (tk.box_fill, border(tk.text_muted), tk.box_text),
            BoxKind::Source { .. } => (tk.box_fill, Stroke::NONE, tk.box_text),
            BoxKind::Note => (tk.note, Stroke::NONE, tk.note_text),
        };
        self.painter.rect_filled(r, radius, fill);
        if stroke != Stroke::NONE {
            self.painter
                .rect_stroke(r, radius, stroke, StrokeKind::Inside);
        }

        let text_width = world.width() - 2.0 * BOX_TEXT_PAD;
        match kind {
            BoxKind::Source { source_type } => {
                self.source_header(world, source_type.key());
                if let Some(text) = text {
                    let top = world.top() + SOURCE_HEADER;
                    self.text(world.center().x, top, text, text_width, kind, text_colour);
                }
            }
            _ => {
                if kind == &BoxKind::Note {
                    self.note_fold(r);
                }
                let Some(text) = text else { return };
                let (shown, colour) = if is_root && text.is_empty() {
                    (t("canvas.empty_claim"), tk.text_muted)
                } else {
                    (text.to_owned(), text_colour)
                };
                let height = measure::galley(self.painter.ctx(), &shown, text_width, kind)
                    .size()
                    .y;
                let top = world.center().y - height / 2.0;
                self.text(world.center().x, top, &shown, text_width, kind, colour);
            }
        }
    }

    /// Wrapped text, each row centred on `centre_x`, the first row at `top`
    /// (world units). Rows come from the same wrapping the layout measured.
    fn text(&self, centre_x: f32, top: f32, text: &str, width: f32, kind: &BoxKind, c: Color32) {
        let base = measure::font_for(kind);
        let Some(font) = screen_font(base.size, self.cam.zoom) else {
            return;
        };
        let galley = measure::galley(self.painter.ctx(), text, width, kind);
        for row in &galley.rows {
            let line = row.text();
            let line = line.trim_end();
            if line.is_empty() {
                continue;
            }
            let at = self.cam.world_to_screen(pos2(centre_x, top + row.pos.y));
            self.painter
                .text(at, Align2::CENTER_TOP, line, font.clone(), c);
        }
    }

    /// Document glyph and bold source type, centred in the source's top row.
    fn source_header(&self, world: Rect, source_type: &str) {
        let Some(font) = screen_font(SOURCE_TYPE_SIZE, self.cam.zoom) else {
            return;
        };
        let z = self.cam.zoom;
        let key = format!("source.{source_type}");
        let mut name = t(&key);
        if name == key {
            name = source_type.to_owned();
        }
        let colour = self.t.box_text;
        let galley = self.painter.layout_no_wrap(name, font, colour);
        let icon = vec2(10.0, 13.0) * z;
        let gap = 5.0 * z;
        let total = icon.x + gap + galley.size().x;
        let centre = self
            .cam
            .world_to_screen(pos2(world.center().x, world.top() + 14.0));
        let left = centre.x - total / 2.0;
        let icon_rect = Rect::from_min_size(pos2(left, centre.y - icon.y / 2.0), icon);
        self.doc_glyph(icon_rect);
        let text_pos = pos2(left + icon.x + gap, centre.y - galley.size().y / 2.0);
        // Faux bold: the default fonts have no bold face.
        self.painter.galley(
            text_pos + vec2(0.45 * z.min(1.5), 0.0),
            galley.clone(),
            colour,
        );
        self.painter.galley(text_pos, galley, colour);
    }

    /// A generic document: a page with a folded corner and three text lines.
    fn doc_glyph(&self, r: Rect) {
        let stroke = Stroke::new(1.0, self.t.text_muted);
        let fold = r.width() * 0.35;
        let outline = vec![
            r.left_top(),
            r.right_top() - vec2(fold, 0.0),
            r.right_top() + vec2(0.0, fold),
            r.right_bottom(),
            r.left_bottom(),
        ];
        self.painter.add(Shape::closed_line(outline, stroke));
        self.painter.add(Shape::line(
            vec![
                r.right_top() - vec2(fold, 0.0),
                r.right_top() + vec2(-fold, fold),
                r.right_top() + vec2(0.0, fold),
            ],
            stroke,
        ));
        for i in 1..=3 {
            let y = r.top() + fold + i as f32 * (r.height() - fold) / 4.0;
            self.painter.line_segment(
                [
                    pos2(r.left() + r.width() * 0.2, y),
                    pos2(r.right() - r.width() * 0.2, y),
                ],
                stroke,
            );
        }
    }

    /// The folded top-right corner of a sticky note.
    fn note_fold(&self, r: Rect) {
        let f = (NOTE_FOLD * self.cam.zoom).min(r.height() / 3.0);
        let shade = self.t.note.lerp_to_gamma(self.t.note_text, 0.22);
        self.painter.add(Shape::convex_polygon(
            vec![
                r.right_top() - vec2(f, 0.0),
                r.right_top() + vec2(0.0, f),
                r.right_top() + vec2(-f, f),
            ],
            shade,
            Stroke::NONE,
        ));
    }
}
