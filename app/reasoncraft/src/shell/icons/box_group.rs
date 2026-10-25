//! Icons of the Home ribbon's Box group; the link icon is also the link
//! glyph on the canvas.

use std::f32::consts::PI;

use eframe::egui::Color32;

use super::Icon;
use super::pen::Pen;
use crate::theme::Tokens;

/// Draw a Box-group `icon` with `pen` in colour `fg`.
pub(super) fn paint(pen: &Pen, icon: Icon, fg: Color32, t: &Tokens) {
    let muted = |c: Color32| if fg == t.text_muted { fg } else { c };
    match icon {
        // A box between two arrows that turn it into another kind.
        Icon::BoxType => {
            pen.rect(0.22, 0.3, 0.78, 0.7, fg);
            let accent = muted(t.accent);
            pen.arc(0.5, 0.5, 0.42, 1.15 * PI, 1.75 * PI);
            pen.line_c(&[(0.64, 0.07), (0.78, 0.17), (0.66, 0.29)], accent);
            pen.arc(0.5, 0.5, 0.42, 0.15 * PI, 0.75 * PI);
            pen.line_c(&[(0.36, 0.93), (0.22, 0.83), (0.34, 0.71)], accent);
        }
        Icon::Link => {
            // Two chain links, slanted, hooked into each other.
            link(pen, (0.36, 0.64));
            link(pen, (0.64, 0.36));
        }
        _ => {}
    }
}

/// One chain link: a stadium centred at `c`, slanted at 45°.
fn link(pen: &Pen, c: (f32, f32)) {
    let (half_len, r) = (0.17, 0.13);
    let (dx, dy) = (PI / 4.0).sin_cos();
    // Along the link (up-right) and across it.
    let (ax, ay) = (dx, -dy);
    let (nx, ny) = (dy, dx);
    let mut pts = Vec::with_capacity(34);
    for (end, start) in [(1.0, -PI / 2.0), (-1.0, PI / 2.0)] {
        let (ex, ey) = (c.0 + ax * half_len * end, c.1 + ay * half_len * end);
        for i in 0..=16 {
            let a = start + PI * i as f32 / 16.0;
            pts.push((
                ex + r * (a.cos() * ax + a.sin() * nx),
                ey + r * (a.cos() * ay + a.sin() * ny),
            ));
        }
    }
    pen.closed(&pts);
}
