//! Scene shapes of boxes: claim, premise, category, source and note.

use eframe::egui::{Align, Color32, Rect, pos2, vec2};
use reasoncraft_core::layout::BOX_TEXT_PAD;
use reasoncraft_core::{BoxKind, BoxNode, SourceType};

use super::build::Builder;
use super::metrics::{content_rect, font_for, image_rect, screen_font};
use super::source_icons::source_icon_shapes_px;
use super::{PathEl, Shape};
use crate::i18n::{t, t_lang};

pub const BOX_RADIUS: f32 = 7.0;
const SOURCE_RADIUS: f32 = 5.0;
const NOTE_RADIUS: f32 = 2.0;
/// Size of a note's folded corner.
const NOTE_FOLD: f32 = 10.0;
const SOURCE_TYPE_SIZE: f32 = 11.5;
/// Size of the type glyph in a source's top row.
const SOURCE_ICON: f32 = 14.0;
/// Gap between a source's type glyph and its type name.
const SOURCE_ICON_GAP: f32 = 5.0;
/// Top of a source's citation text below the box top: padding plus the
/// icon and type row. The layout reserves this plus the bottom padding.
pub const SOURCE_HEADER: f32 = 24.0;

pub fn box_radius(kind: &BoxKind) -> f32 {
    match kind {
        BoxKind::Source { .. } => SOURCE_RADIUS,
        BoxKind::Note => NOTE_RADIUS,
        _ => BOX_RADIUS,
    }
}

impl Builder<'_> {
    /// A box of any kind: its indicator row (`row` high, its marks drawn
    /// elsewhere) on top, then its image, then its content. `border` is the
    /// colour of the card a premise sits in.
    pub(super) fn box_(
        &mut self,
        world: Rect,
        row: f32,
        node: &BoxNode,
        border: Option<Color32>,
        is_root: bool,
    ) {
        if !self.visible(world) {
            return;
        }
        let tk = self.opts.tokens;
        let kind = &node.kind;
        // Borders are one screen px wide.
        let px = self.px;
        let with = |c: Color32| Some((px, c));
        let (fill, stroke, text_colour) = match kind {
            BoxKind::Claim => (tk.box_fill, with(tk.claim_border), tk.box_text),
            BoxKind::Premise => {
                let c = border.unwrap_or(tk.text_muted);
                (tk.box_fill, with(c), tk.box_text)
            }
            BoxKind::Category => (tk.box_fill, with(tk.text_muted), tk.box_text),
            BoxKind::Source { .. } => (tk.box_fill, None, tk.box_text),
            BoxKind::Note => (tk.note, None, tk.note_text),
        };
        self.shapes.push(Shape::Rect {
            rect: world,
            radius: box_radius(kind),
            fill,
            stroke,
        });

        let text_width = world.width() - 2.0 * BOX_TEXT_PAD;
        let mut body = world;
        body.min.y += row;
        if let Some(img) = &node.image {
            self.shapes.push(Shape::Image {
                rect: image_rect(body, img),
                asset: img.asset.clone(),
                missing: t_lang(self.opts.lang, "image.missing"),
            });
        }
        // The inline editor shows the text of the box being edited.
        let text = (self.opts.editing != Some(node.id)).then_some(node.text.as_str());
        // Header and text go below the image, as the layout measured them.
        let content = content_rect(body, node);
        match kind {
            BoxKind::Source { source_type } => {
                self.source_header(content, source_type);
                if let Some(text) = text {
                    let rows = self.fonts.rows(text, text_width, kind);
                    let top = content.top() + SOURCE_HEADER;
                    self.text(content.center().x, top, rows, kind, text_colour);
                }
            }
            _ => {
                if kind == &BoxKind::Note {
                    self.note_fold(world);
                }
                let Some(text) = text else { return };
                let (shown, colour) = if is_root && text.is_empty() {
                    if self.opts.for_export {
                        return;
                    }
                    (t("canvas.empty_claim"), tk.text_muted)
                } else {
                    (text.to_owned(), text_colour)
                };
                let rows = self.fonts.rows(&shown, text_width, kind);
                let height = self.fonts.text_height(&shown, text_width, kind);
                let top = content.center().y - height / 2.0;
                self.text(content.center().x, top, rows, kind, colour);
            }
        }
    }

    /// Wrapped rows, each centred on `centre_x`, the first at `top`.
    fn text(&mut self, centre_x: f32, top: f32, rows: Vec<String>, kind: &BoxKind, c: Color32) {
        let size = font_for(kind).size;
        self.shapes.push(Shape::Text {
            origin: pos2(centre_x, top),
            rows,
            size,
            bold: false,
            color: c,
            align: Align::Center,
            line_height: self.fonts.line_height(size),
        });
    }

    /// Type glyph and bold type name, centred in the source's top row.
    fn source_header(&mut self, world: Rect, source_type: &SourceType) {
        if screen_font(SOURCE_TYPE_SIZE, self.opts.zoom).is_none() {
            return;
        }
        let name = t_lang(self.opts.lang, &format!("source.{}", source_type.key()));
        let width = self.text_width(&name, SOURCE_TYPE_SIZE);
        let line_height = self.fonts.line_height(SOURCE_TYPE_SIZE);
        let total = SOURCE_ICON + SOURCE_ICON_GAP + width;
        let centre = pos2(world.center().x, world.top() + 14.0);
        let left = centre.x - total / 2.0;
        let icon = Rect::from_min_size(
            pos2(left, centre.y - SOURCE_ICON / 2.0),
            vec2(SOURCE_ICON, SOURCE_ICON),
        );
        let muted = self.opts.tokens.text_muted;
        let glyph = source_icon_shapes_px(icon, source_type, muted, self.px);
        self.shapes.extend(glyph);
        self.shapes.push(Shape::Text {
            origin: pos2(
                icon.right() + SOURCE_ICON_GAP,
                centre.y - self.row_height(SOURCE_TYPE_SIZE) / 2.0,
            ),
            rows: vec![name],
            size: SOURCE_TYPE_SIZE,
            bold: true,
            color: self.opts.tokens.box_text,
            align: Align::Min,
            line_height,
        });
    }

    /// The folded top-right corner of a sticky note.
    fn note_fold(&mut self, r: Rect) {
        let f = NOTE_FOLD.min(r.height() / 3.0);
        let tk = self.opts.tokens;
        let corner = r.right_top();
        self.shapes.push(Shape::Path {
            points: vec![
                PathEl::MoveTo(corner - vec2(f, 0.0)),
                PathEl::LineTo(corner + vec2(0.0, f)),
                PathEl::LineTo(corner + vec2(-f, f)),
                PathEl::Close,
            ],
            stroke: None,
            fill: Some(tk.note.lerp_to_gamma(tk.note_text, 0.22)),
            dashed: false,
        });
    }
}
