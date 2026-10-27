//! Faux bold in exports: the bundled fonts have no bold face, so bold text
//! gets a thin stroke in its own colour, painted under the fill.

use std::fmt::Write as _;

use resvg::usvg::{self, Node, tiny_skia_path::PathSegment};

use super::shared_fontdb;
use super::svg_attr::{escape, num};

/// The width of the stroke that makes text bold, in px.
pub(super) const BOLD_STROKE: f32 = 0.35;

/// The glyph outlines of `text`, one `<text>` element, as the `d` of a
/// path in the same user space; `None` when it has no visible glyphs.
pub(super) fn stroke_outline(text: &str, font_family: &str) -> Option<String> {
    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\"><g font-family=\"{}\">{text}</g></svg>",
        escape(font_family)
    );
    let opt = usvg::Options {
        fontdb: shared_fontdb(),
        ..Default::default()
    };
    let tree = usvg::Tree::from_str(&svg, &opt).ok()?;
    let mut d = String::new();
    outlines(tree.root(), false, &mut d);
    (!d.is_empty()).then_some(d)
}

/// Appends the paths under `group` to `d`; only those of text (`in_text`)
/// when not inside a text node yet.
fn outlines(group: &usvg::Group, in_text: bool, d: &mut String) {
    for node in group.children() {
        match node {
            Node::Group(g) => outlines(g, in_text, d),
            Node::Text(t) => outlines(t.flattened(), true, d),
            Node::Path(p) if in_text => {
                let Some(path) = p.data().clone().transform(p.abs_transform()) else {
                    continue;
                };
                for seg in path.segments() {
                    if !d.is_empty() {
                        d.push(' ');
                    }
                    let pt = |p: usvg::tiny_skia_path::Point| format!("{} {}", num(p.x), num(p.y));
                    let _ = match seg {
                        PathSegment::MoveTo(a) => write!(d, "M{}", pt(a)),
                        PathSegment::LineTo(a) => write!(d, "L{}", pt(a)),
                        PathSegment::QuadTo(a, b) => write!(d, "Q{} {}", pt(a), pt(b)),
                        PathSegment::CubicTo(a, b, c) => {
                            write!(d, "C{} {} {}", pt(a), pt(b), pt(c))
                        }
                        PathSegment::Close => write!(d, "Z"),
                    };
                }
            }
            _ => {}
        }
    }
}
