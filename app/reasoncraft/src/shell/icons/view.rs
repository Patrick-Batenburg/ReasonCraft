//! View tab icons: fit selection, the minimap and map numbering.

use eframe::egui::Color32;

use super::Icon;
use super::pen::Pen;
use crate::theme::Tokens;

/// Draw a View `icon` with `pen` in colour `fg`.
pub(super) fn paint(pen: &Pen, icon: Icon, fg: Color32, t: &Tokens) {
    let muted = |c: Color32| if fg == t.text_muted { fg } else { c };
    match icon {
        // Fit's corner marks around one selected box.
        Icon::ZoomFitSelection => {
            for (x, y, dx, dy) in [
                (0.1, 0.1, 1.0, 1.0),
                (0.9, 0.1, -1.0, 1.0),
                (0.1, 0.9, 1.0, -1.0),
                (0.9, 0.9, -1.0, -1.0),
            ] {
                pen.line(&[(x, y + 0.25 * dy), (x, y), (x + 0.25 * dx, y)]);
            }
            pen.fill(0.3, 0.38, 0.7, 0.62, muted(t.accent));
        }
        // A small map in a frame, with the part in view outlined.
        Icon::Minimap => {
            pen.rect(0.08, 0.16, 0.92, 0.84, fg);
            pen.fill(0.42, 0.26, 0.58, 0.36, fg);
            pen.fill(0.2, 0.5, 0.36, 0.6, fg);
            pen.fill(0.64, 0.5, 0.8, 0.6, fg);
            pen.line(&[(0.5, 0.36), (0.5, 0.43), (0.28, 0.43), (0.28, 0.5)]);
            pen.line(&[(0.5, 0.43), (0.72, 0.43), (0.72, 0.5)]);
            pen.rect(0.14, 0.42, 0.52, 0.76, muted(t.accent));
        }
        // Numbered rows: "1" marks before lines of text.
        Icon::Numbers => {
            for y in [0.24, 0.5, 0.76] {
                pen.line_c(
                    &[(0.12, y - 0.04), (0.2, y - 0.1), (0.2, y + 0.1)],
                    muted(t.accent),
                );
                pen.line(&[(0.36, y), (0.9, y)]);
            }
        }
        _ => {}
    }
}
