//! Attribute values and text of the SVG [`super::svg`] writes: colours,
//! numbers, path data and escaped text.

use std::fmt::Write as _;

use eframe::egui::{Color32, Pos2};

use crate::scene::PathEl;

/// ` fill="#rrggbb"` (or `stroke`), plus its opacity when not opaque.
pub(super) fn paint(out: &mut String, what: &str, c: Color32) {
    let [r, g, b, a] = c.to_srgba_unmultiplied();
    let _ = write!(out, " {what}=\"#{r:02x}{g:02x}{b:02x}\"");
    if a < 255 {
        let _ = write!(out, " {what}-opacity=\"{}\"", num(f32::from(a) / 255.0));
    }
}

/// The `d` attribute of a path.
pub(super) fn path_data(els: &[PathEl]) -> String {
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

/// The `d` attribute of a band: along its left edge, back along its right
/// edge, closed.
pub(super) fn strip_data(left: &[Pos2], right: &[Pos2]) -> String {
    let els: Vec<PathEl> = left
        .iter()
        .chain(right.iter().rev())
        .enumerate()
        .map(|(i, &p)| {
            if i == 0 {
                PathEl::MoveTo(p)
            } else {
                PathEl::LineTo(p)
            }
        })
        .chain([PathEl::Close])
        .collect();
    path_data(&els)
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
pub(super) fn escape(s: &str) -> String {
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
