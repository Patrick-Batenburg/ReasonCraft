//! A [`Scene`] written as SVG 1.1: live `<text>` in the canvas's font, or
//! (the default) with the text turned into outlines so the file looks the
//! same without that font installed.

use std::fmt::Write as _;

use base64::Engine as _;
use eframe::egui::{Align, Color32, Pos2, Rect};
use reasoncraft_core::Assets;
use resvg::usvg;

use super::assets::export_png_bytes;
use super::svg_bold::{BOLD_STROKE, stroke_outline};
use super::{TEXT_FACE, shared_fontdb};
use crate::canvas::images::MAX_STORED_SIDE;
use crate::scene::{DASH_LEN, GAP_LEN, Scene, Shape, StrokeSpec, missing_image};
use crate::theme::Tokens;

pub(super) use super::svg_attr::num;
use super::svg_attr::{escape, paint, path_data, strip_data};

/// Default space around the map, in world px.
pub const DEFAULT_MARGIN: f32 = 24.0;

pub(super) const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

/// The width and height in a PNG's header (`IHDR`, the first chunk);
/// `None` when `bytes` do not start like a PNG.
fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    let header = bytes.strip_prefix(PNG_SIGNATURE)?.get(..16)?;
    if &header[4..8] != b"IHDR" {
        return None;
    }
    let be = |b: &[u8]| u32::from_be_bytes([b[0], b[1], b[2], b[3]]);
    Some((be(&header[8..12]), be(&header[12..16])))
}

/// Whether the canvas draws an image this size (see [`MAX_STORED_SIDE`]).
fn drawable((w, h): (u32, u32)) -> bool {
    w.max(h) <= MAX_STORED_SIDE
}

/// How a scene is written.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SvgOptions {
    /// Turn text into outlines (paths), so no font is needed to view it.
    pub text_as_paths: bool,
    /// Leave the background out.
    pub transparent: bool,
    pub background: Color32,
    /// Space around the map, in world px.
    pub margin: f32,
    /// The colour of the placeholder for a missing image.
    pub muted: Color32,
    /// Draw the stroke that makes faux-bold text bold as glyph outlines
    /// under the text, not on it. A PDF converter writes stroked text as
    /// two text runs (fill and stroke), which would copy twice.
    pub bold_stroke_as_path: bool,
}

impl SvgOptions {
    /// Outlined text on the canvas colour of `tokens`.
    pub fn new(tokens: &Tokens) -> Self {
        Self {
            text_as_paths: true,
            transparent: false,
            background: tokens.canvas,
            margin: DEFAULT_MARGIN,
            muted: tokens.text_muted,
            bold_stroke_as_path: false,
        }
    }

    /// Live text, each word written once, for [`svg_to_pdf`](super::svg_to_pdf).
    pub fn for_pdf(tokens: &Tokens) -> Self {
        Self {
            text_as_paths: false,
            bold_stroke_as_path: true,
            ..Self::new(tokens)
        }
    }
}

/// `scene` as an SVG document whose view box is the scene's bounds plus
/// the margin. Text uses `font_family` (see
/// [`bundled_font_family`](super::bundled_font_family)); images are
/// embedded from `assets` at full resolution. The scene must be drawn at
/// zoom 1 (as export scenes are), so its world units are SVG px.
pub fn scene_to_svg(
    scene: &Scene,
    assets: &Assets,
    opts: &SvgOptions,
    font_family: &str,
) -> String {
    let svg = live_svg(scene, assets, opts, font_family);
    if !opts.text_as_paths {
        return svg;
    }
    match outline_text(&svg) {
        Ok(outlined) => outlined,
        Err(e) => {
            // Our own output always parses; keep live text rather than
            // lose the export if it ever does not.
            debug_assert!(false, "generated SVG does not parse: {e}");
            svg
        }
    }
}

/// `svg` with its text turned into paths, via usvg and the bundled fonts.
fn outline_text(svg: &str) -> Result<String, usvg::Error> {
    let opt = usvg::Options {
        fontdb: shared_fontdb(),
        ..Default::default()
    };
    let tree = usvg::Tree::from_str(svg, &opt)?;
    Ok(tree.to_string(&usvg::WriteOptions::default()))
}

/// The world rect an SVG of `scene` shows: its bounds plus the margin.
fn view_rect(scene: &Scene, opts: &SvgOptions) -> Rect {
    scene.bounds.expand(opts.margin.max(0.0))
}

/// The width and height in px of the SVG [`scene_to_svg`] writes.
pub fn svg_size(scene: &Scene, opts: &SvgOptions) -> (f32, f32) {
    let view = view_rect(scene, opts);
    (view.width().max(1.0), view.height().max(1.0))
}

fn live_svg(scene: &Scene, assets: &Assets, opts: &SvgOptions, font_family: &str) -> String {
    let view = view_rect(scene, opts);
    let (w, h) = svg_size(scene, opts);
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    let _ = writeln!(
        out,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" \
         xmlns:xlink=\"http://www.w3.org/1999/xlink\" version=\"1.1\" \
         width=\"{}\" height=\"{}\" viewBox=\"{} {} {} {}\">",
        num(w),
        num(h),
        num(view.min.x),
        num(view.min.y),
        num(w),
        num(h),
    );
    if !opts.transparent {
        let _ = write!(
            out,
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"",
            num(view.min.x),
            num(view.min.y),
            num(w),
            num(h)
        );
        paint(&mut out, "fill", opts.background);
        out.push_str("/>\n");
    }
    let _ = writeln!(out, "<g font-family=\"{}\">", escape(font_family));
    let mut w = Writer {
        out,
        opts,
        assets,
        font_family,
    };
    for shape in &scene.shapes {
        w.shape(shape);
    }
    let mut out = w.out;
    out.push_str("</g>\n</svg>\n");
    out
}

struct Writer<'a> {
    out: String,
    opts: &'a SvgOptions,
    assets: &'a Assets,
    font_family: &'a str,
}

impl Writer<'_> {
    fn shape(&mut self, shape: &Shape) {
        match shape {
            Shape::Rect {
                rect,
                radius,
                fill,
                stroke,
            } => {
                if *fill != Color32::TRANSPARENT {
                    self.rect(*rect, *radius);
                    paint(&mut self.out, "fill", *fill);
                    self.out.push_str("/>\n");
                }
                // The scene's rect strokes lie inside the rect; SVG centres
                // them on the edge, so the edge moves in by half a stroke.
                if let Some((width, colour)) = *stroke {
                    let half = width / 2.0;
                    self.rect(rect.shrink(half), (radius - half).max(0.0));
                    self.out.push_str(" fill=\"none\"");
                    self.stroke(Some((width, colour)));
                    self.out.push_str("/>\n");
                }
            }
            Shape::Circle {
                center,
                radius,
                fill,
                stroke,
            } => {
                let _ = write!(
                    self.out,
                    "<circle cx=\"{}\" cy=\"{}\" r=\"{}\"",
                    num(center.x),
                    num(center.y),
                    num(*radius)
                );
                self.fill(*fill);
                self.stroke(*stroke);
                self.out.push_str("/>\n");
            }
            Shape::Path {
                points,
                stroke,
                fill,
                dashed,
            } => {
                let _ = write!(self.out, "<path d=\"{}\"", path_data(points));
                self.fill(*fill);
                self.stroke(*stroke);
                if *dashed {
                    self.dashes(DASH_LEN, GAP_LEN);
                }
                self.out.push_str("/>\n");
            }
            Shape::Text {
                origin,
                rows,
                size,
                bold,
                color,
                align,
                line_height,
            } => {
                for (i, row) in rows.iter().enumerate() {
                    let top = origin.y + i as f32 * line_height;
                    self.text_row(Pos2::new(origin.x, top), row, *size, *bold, *color, *align);
                }
            }
            Shape::Image { rect, asset } => self.image(*rect, asset),
            Shape::Strip { left, right, fill } => {
                let _ = write!(self.out, "<path d=\"{}\"", strip_data(left, right));
                self.fill(Some(*fill));
                self.out.push_str(
                    "/>
",
                );
            }
        }
    }

    /// One row of text whose top is at `at.y`, drawn once. Bold uses the
    /// font's bold face, or (the bundled fonts have none) a thin stroke
    /// painted under the fill.
    fn text_row(&mut self, at: Pos2, row: &str, size: f32, bold: bool, c: Color32, align: Align) {
        let anchor = match align {
            Align::Min => "start",
            Align::Center => "middle",
            Align::Max => "end",
        };
        let mut open = format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"{}\" text-anchor=\"{anchor}\"",
            num(at.x),
            num(at.y + TEXT_FACE.baseline(size)),
            num(size),
        );
        let body = format!(" xml:space=\"preserve\">{}</text>", escape(row));
        if bold && TEXT_FACE.has_bold {
            open.push_str(" font-weight=\"bold\"");
        }
        let mut stroke = bold && !TEXT_FACE.has_bold;
        if stroke
            && self.opts.bold_stroke_as_path
            && let Some(d) = stroke_outline(&format!("{open}{body}"), self.font_family)
        {
            let _ = write!(self.out, "<path d=\"{d}\" fill=\"none\"");
            self.bold_stroke(c);
            self.out.push_str("/>\n");
            stroke = false;
        }
        self.out.push_str(&open);
        paint(&mut self.out, "fill", c);
        if stroke {
            self.bold_stroke(c);
            self.out.push_str(" paint-order=\"stroke\"");
        }
        self.out.push_str(&body);
        self.out.push('\n');
    }

    /// The stroke that makes text bold, in its colour `c`.
    fn bold_stroke(&mut self, c: Color32) {
        self.stroke(Some((BOLD_STROKE, c)));
        self.out.push_str(" stroke-linejoin=\"round\"");
    }

    /// An embedded PNG (JPEG and WebP assets converted), or the
    /// missing-image placeholder (also for one the canvas refuses to draw
    /// as too large).
    fn image(&mut self, r: Rect, asset: &str) {
        let asset = self.assets.get(asset);
        let png = asset.as_deref().and_then(export_png_bytes);
        match png {
            Some(bytes) if png_size(&bytes).is_some_and(drawable) => {
                let data = base64::engine::general_purpose::STANDARD.encode(&bytes);
                let _ = writeln!(
                    self.out,
                    "<image x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" \
                     preserveAspectRatio=\"none\" xlink:href=\"data:image/png;base64,{data}\"/>",
                    num(r.min.x),
                    num(r.min.y),
                    num(r.width()),
                    num(r.height()),
                );
            }
            _ => self.missing_image(r),
        }
    }

    fn missing_image(&mut self, r: Rect) {
        let muted = self.opts.muted;
        self.rect(r, 0.0);
        self.out.push_str(" fill=\"none\"");
        self.stroke(Some((missing_image::STROKE, muted)));
        self.dashes(missing_image::DASH, missing_image::GAP);
        self.out.push_str("/>\n");
        let size = missing_image::TEXT_SIZE;
        let top = r.center().y - TEXT_FACE.row_height(size) / 2.0;
        let label = missing_image::label();
        let at = Pos2::new(r.center().x, top);
        self.text_row(at, &label, size, false, muted, Align::Center);
    }

    /// The start of a `<rect>` element.
    fn rect(&mut self, r: Rect, radius: f32) {
        let _ = write!(
            self.out,
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"",
            num(r.min.x),
            num(r.min.y),
            num(r.width().max(0.0)),
            num(r.height().max(0.0)),
        );
        if radius > 0.0 {
            let _ = write!(self.out, " rx=\"{}\"", num(radius));
        }
    }

    fn fill(&mut self, fill: Option<Color32>) {
        match fill {
            Some(c) if c != Color32::TRANSPARENT => paint(&mut self.out, "fill", c),
            _ => self.out.push_str(" fill=\"none\""),
        }
    }

    fn stroke(&mut self, stroke: Option<StrokeSpec>) {
        if let Some((width, c)) = stroke {
            paint(&mut self.out, "stroke", c);
            let _ = write!(self.out, " stroke-width=\"{}\"", num(width));
        }
    }

    fn dashes(&mut self, dash: f32, gap: f32) {
        let _ = write!(self.out, " stroke-dasharray=\"{} {}\"", num(dash), num(gap));
    }
}
