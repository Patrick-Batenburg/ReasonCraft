//! Find and Replace icons.

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
        _ => {}
    }
}
