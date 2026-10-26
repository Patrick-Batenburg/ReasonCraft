//! Find, Replace and Spelling icons.

use eframe::egui::Color32;

use super::Icon;
use super::pen::Pen;
use crate::theme::Tokens;

/// Draw a find `icon` with `pen` in colour `fg`.
pub(super) fn paint(pen: &Pen, icon: Icon, fg: Color32, t: &Tokens) {
    let muted = |c: Color32| if fg == t.text_muted { fg } else { c };
    match icon {
        // A magnifying glass.
        Icon::Find => {
            pen.circle(0.42, 0.42, 0.28);
            pen.line(&[(0.62, 0.62), (0.9, 0.9)]);
        }
        // A line of text, an arrow down and the line that replaces it.
        Icon::Replace => {
            pen.line(&[(0.12, 0.2), (0.6, 0.2)]);
            pen.line_c(&[(0.36, 0.32), (0.36, 0.68)], muted(t.accent));
            pen.line_c(&[(0.24, 0.56), (0.36, 0.68), (0.48, 0.56)], muted(t.accent));
            pen.line(&[(0.12, 0.82), (0.88, 0.82)]);
        }
        // "abc" as three letter strokes over a wavy underline.
        Icon::Spelling => {
            pen.line(&[(0.14, 0.62), (0.26, 0.22), (0.38, 0.62)]);
            pen.line(&[(0.19, 0.46), (0.33, 0.46)]);
            pen.line(&[(0.48, 0.2), (0.48, 0.62)]);
            pen.circle(0.58, 0.5, 0.1);
            pen.line(&[
                (0.88, 0.42),
                (0.8, 0.4),
                (0.74, 0.5),
                (0.8, 0.6),
                (0.88, 0.58),
            ]);
            let wave: Vec<(f32, f32)> = (0..=8)
                .map(|i| (0.12 + i as f32 * 0.095, if i % 2 == 0 { 0.8 } else { 0.88 }))
                .collect();
            pen.line_c(&wave, muted(t.spell_error));
        }
        _ => {}
    }
}
