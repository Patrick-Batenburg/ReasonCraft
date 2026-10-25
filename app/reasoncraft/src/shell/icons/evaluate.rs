//! Evaluation and comment icons.

use std::f32::consts::PI;

use eframe::egui::{Color32, Stroke};

use super::Icon;
use super::pen::Pen;
use crate::theme::Tokens;

/// Draw an evaluation or comment `icon` with `pen` in colour `fg`.
pub(super) fn paint(pen: &Pen, icon: Icon, fg: Color32, t: &Tokens) {
    let painter = pen.painter;
    let rect = pen.rect;
    let muted = |c: Color32| if fg == t.text_muted { fg } else { c };
    match icon {
        Icon::Accept => {
            pen.circle(0.5, 0.5, 0.4);
            pen.line_c(&[(0.3, 0.52), (0.44, 0.66), (0.7, 0.36)], muted(t.reason));
        }
        Icon::Unsure => {
            pen.circle(0.5, 0.5, 0.4);
            pen.arc(0.5, 0.4, 0.12, PI, 2.4 * PI);
            pen.line(&[(0.5, 0.52), (0.5, 0.58)]);
            pen.disc(0.5, 0.7, 0.035, fg);
        }
        Icon::Reject => {
            pen.circle(0.5, 0.5, 0.4);
            let c = muted(t.objection);
            pen.line_c(&[(0.36, 0.36), (0.64, 0.64)], c);
            pen.line_c(&[(0.64, 0.36), (0.36, 0.64)], c);
        }
        // A link that carries its conclusion firmly, weakly or not at all.
        Icon::Strong => {
            let thick = Stroke::new(pen.stroke.width * 2.2, fg);
            painter.line_segment([pen.p(0.5, 0.88), pen.p(0.5, 0.2)], thick);
            painter.line(
                vec![pen.p(0.3, 0.38), pen.p(0.5, 0.16), pen.p(0.7, 0.38)],
                thick,
            );
        }
        Icon::Weak => {
            for i in 0..3 {
                let y = 0.88 - 0.2 * i as f32;
                pen.line(&[(0.5, y), (0.5, y - 0.1)]);
            }
            pen.line(&[(0.34, 0.34), (0.5, 0.18), (0.66, 0.34)]);
        }
        Icon::Nil => {
            pen.circle(0.5, 0.5, 0.32);
            pen.line(&[(0.22, 0.86), (0.78, 0.14)]);
        }
        Icon::Clear => {
            // An eraser on a line.
            pen.line(&[
                (0.18, 0.56),
                (0.5, 0.24),
                (0.8, 0.54),
                (0.48, 0.86),
                (0.18, 0.56),
            ]);
            pen.line(&[(0.34, 0.4), (0.64, 0.7)]);
            pen.line_c(&[(0.48, 0.9), (0.92, 0.9)], muted(t.accent));
        }
        Icon::Markers => {
            pen.rect(0.1, 0.3, 0.78, 0.82, fg);
            pen.disc(0.78, 0.3, 0.14, muted(t.reason));
        }
        Icon::Source => crate::shell::source_icons::paint_source_icon(
            painter,
            rect,
            &reasoncraft_core::SourceType::Source,
            fg,
        ),
        Icon::Shading => {
            pen.rect(0.12, 0.2, 0.88, 0.8, fg);
            pen.fill(0.12, 0.2, 0.5, 0.8, muted(t.accent).gamma_multiply(0.6));
        }
        Icon::Comment => {
            pen.closed(&[
                (0.1, 0.18),
                (0.9, 0.18),
                (0.9, 0.68),
                (0.46, 0.68),
                (0.26, 0.88),
                (0.28, 0.68),
                (0.1, 0.68),
            ]);
            pen.line_c(&[(0.26, 0.36), (0.74, 0.36)], muted(t.accent));
            pen.line_c(&[(0.26, 0.5), (0.6, 0.5)], muted(t.accent));
        }
        // Command icons are drawn by the caller.
        _ => {}
    }
}
