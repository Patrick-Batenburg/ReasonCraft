//! Help, keyboard and About icons.

use eframe::egui::Color32;

use super::Icon;
use super::pen::Pen;
use crate::theme::Tokens;

/// Draw a help `icon` with `pen` in colour `fg`.
pub(super) fn paint(pen: &Pen, icon: Icon, fg: Color32, t: &Tokens) {
    let muted = |c: Color32| if fg == t.text_muted { fg } else { c };
    match icon {
        // A question mark in a circle.
        Icon::Help => {
            pen.circle(0.5, 0.5, 0.4);
            pen.arc(
                0.5,
                0.4,
                0.13,
                std::f32::consts::PI,
                2.4 * std::f32::consts::PI,
            );
            pen.line(&[(0.56, 0.51), (0.5, 0.56), (0.5, 0.62)]);
            pen.disc(0.5, 0.74, 0.04, muted(t.accent));
        }
        // A keyboard: its outline, two rows of keys and the space bar.
        Icon::Keyboard => {
            pen.rect(0.06, 0.26, 0.94, 0.76, fg);
            for row in [0.4, 0.52] {
                for i in 0..5 {
                    let x = 0.2 + 0.15 * i as f32;
                    pen.disc(x, row, 0.035, fg);
                }
            }
            pen.line_c(&[(0.3, 0.64), (0.7, 0.64)], muted(t.accent));
        }
        // An "i" in a circle.
        Icon::About => {
            pen.circle(0.5, 0.5, 0.4);
            pen.disc(0.5, 0.3, 0.045, muted(t.accent));
            pen.line(&[(0.44, 0.44), (0.5, 0.44), (0.5, 0.72)]);
            pen.line(&[(0.42, 0.72), (0.58, 0.72)]);
        }
        _ => {}
    }
}
