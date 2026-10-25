//! A [`Scene`] written as SVG 1.1: live `<text>` in the canvas's font, or
//! (the default) with the text turned into outlines so the file looks the
//! same without that font installed.

use std::fmt::Write as _;

use base64::Engine as _;
use eframe::egui::{Align, Color32, Pos2, Rect};
use reasoncraft_core::Assets;
use resvg::usvg;

use super::{TEXT_FACE, shared_fontdb};
use crate::scene::{
    DASH_LEN, GAP_LEN, PathEl, Scene, Shape, StrokeSpec, bold_offset, missing_image,
};
use crate::theme::Tokens;

/// Default space around the map, in world px.
pub const DEFAULT_MARGIN: f32 = 24.0;

const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

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
    let mut w = Writer { out, opts, assets };
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
        }
    }

    /// One row of text whose top is at `at.y`.
    fn text_row(&mut self, at: Pos2, row: &str, size: f32, bold: bool, c: Color32, align: Align) {
        let anchor = match align {
            Align::Min => "start",
            Align::Center => "middle",
            Align::Max => "end",
        };
        let _ = write!(
            self.out,
            "<text x=\"{}\" y=\"{}\" font-size=\"{}\" text-anchor=\"{anchor}\"",
            num(at.x),
            num(at.y + TEXT_FACE.baseline(size)),
            num(size),
        );
        paint(&mut self.out, "fill", c);
        if bold {
            // The screen draws bold text twice, `bold_offset` apart; a
            // stroke that wide thickens the glyphs the same amount.
            self.stroke(Some((bold_offset(1.0), c)));
            self.out.push_str(" stroke-linejoin=\"round\"");
        }
        let _ = writeln!(self.out, " xml:space=\"preserve\">{}</text>", escape(row));
    }

    /// An embedded PNG, or the missing-image placeholder.
    fn image(&mut self, r: Rect, asset: &str) {
        match self.assets.get(asset) {
            Some(bytes) if bytes.starts_with(PNG_SIGNATURE) => {
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

/// ` fill="#rrggbb"` (or `stroke`), plus its opacity when not opaque.
fn paint(out: &mut String, what: &str, c: Color32) {
    let [r, g, b, a] = c.to_srgba_unmultiplied();
    let _ = write!(out, " {what}=\"#{r:02x}{g:02x}{b:02x}\"");
    if a < 255 {
        let _ = write!(out, " {what}-opacity=\"{}\"", num(f32::from(a) / 255.0));
    }
}

/// The `d` attribute of a path.
fn path_data(els: &[PathEl]) -> String {
    let mut d = String::new();
    let p = |p: Pos2| format!("{} {}", num(p.x), num(p.y));
    for el in els {
        let step = match *el {
            PathEl::MoveTo(a) => format!("M{}", p(a)),
            PathEl::LineTo(a) => format!("L{}", p(a)),
            PathEl::CubicTo(a, b, c) => format!("C{} {} {}", p(a), p(b), p(c)),
            PathEl::Close => "Z".to_owned(),
        };
        if !d.is_empty() {
            d.push(' ');
        }
        d.push_str(&step);
    }
    d
}

/// `v` with at most three decimals and no trailing zeros.
pub(super) fn num(v: f32) -> String {
    // NaN and infinity are not SVG numbers.
    let v = if v.is_finite() { v } else { 0.0 };
    let s = format!("{v:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" {
        "0".to_owned()
    } else {
        s.to_owned()
    }
}

/// `s` safe in XML text and attribute values.
fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            // Not allowed in XML 1.0 at all.
            c if (c as u32) < 0x20 && !matches!(c, '\t' | '\n' | '\r') => {}
            // Noncharacters XML 1.0 forbids.
            '\u{FFFE}' | '\u{FFFF}' => {}
            c => out.push(c),
        }
    }
    out
}
