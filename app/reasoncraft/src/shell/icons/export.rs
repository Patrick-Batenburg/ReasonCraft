//! Icons of the Export ribbon tab (and Copy as image on the Home tab).

use eframe::egui::Color32;

use super::Icon;
use super::pen::Pen;
use crate::theme::Tokens;

/// Draw an Export-tab `icon` with `pen` in colour `fg`.
pub(super) fn paint(pen: &Pen, icon: Icon, fg: Color32, t: &Tokens) {
    let accent = if fg == t.text_muted { fg } else { t.accent };
    match icon {
        // A picture with an arrow out of it.
        Icon::ExportPng => {
            super::box_group::picture(pen, accent);
            pen.line_c(&[(0.82, 0.62), (0.82, 0.96)], accent);
            pen.line_c(&[(0.7, 0.84), (0.82, 0.96), (0.94, 0.84)], accent);
        }
        // A curve between two anchor handles.
        Icon::ExportSvg => {
            pen.line(&[
                (0.18, 0.78),
                (0.26, 0.5),
                (0.42, 0.3),
                (0.62, 0.26),
                (0.82, 0.22),
            ]);
            pen.rect(0.1, 0.7, 0.26, 0.86, accent);
            pen.rect(0.74, 0.14, 0.9, 0.3, accent);
            pen.line_c(&[(0.26, 0.5), (0.5, 0.62)], accent);
        }
        // A page with a folded corner and lines of text.
        Icon::ExportPdf | Icon::Essay => {
            pen.line(&[
                (0.62, 0.08),
                (0.18, 0.08),
                (0.18, 0.92),
                (0.82, 0.92),
                (0.82, 0.28),
                (0.62, 0.08),
                (0.62, 0.28),
                (0.82, 0.28),
            ]);
            let lines: &[(f32, f32)] = if icon == Icon::Essay {
                &[(0.42, 0.68), (0.54, 0.68), (0.66, 0.68), (0.78, 0.5)]
            } else {
                &[(0.5, 0.6), (0.64, 0.68), (0.78, 0.5)]
            };
            for &(y, x1) in lines {
                pen.line_c(&[(0.3, y), (x1, y)], accent);
            }
        }
        // A picture in front of a copy.
        Icon::CopyImage => {
            pen.rect(0.08, 0.08, 0.6, 0.56, fg);
            pen.rect(0.34, 0.36, 0.92, 0.9, fg);
            pen.line_c(
                &[(0.4, 0.84), (0.56, 0.62), (0.68, 0.74), (0.86, 0.56)],
                accent,
            );
            pen.disc(0.76, 0.48, 0.05, accent);
        }
        // Rows of a nested list.
        Icon::Outline => {
            for (x, y) in [(0.14, 0.22), (0.32, 0.5), (0.32, 0.78)] {
                pen.disc(x, y, 0.06, accent);
                pen.line(&[(x + 0.14, y), (0.9, y)]);
            }
        }
        // A page and its margins.
        Icon::PageSetup => {
            pen.rect(0.16, 0.08, 0.84, 0.92, fg);
            for (x, y, dx, dy) in [
                (0.3, 0.22, 1.0, 1.0),
                (0.7, 0.22, -1.0, 1.0),
                (0.3, 0.78, 1.0, -1.0),
                (0.7, 0.78, -1.0, -1.0),
            ] {
                let pts = [(x, y + 0.12 * dy), (x, y), (x + 0.12 * dx, y)];
                pen.line_c(&pts, accent);
            }
        }
        // A printer with a sheet coming out.
        Icon::Print => {
            pen.rect(0.28, 0.08, 0.72, 0.34, fg);
            pen.rect(0.08, 0.34, 0.92, 0.7, fg);
            pen.line(&[(0.28, 0.7), (0.28, 0.92), (0.72, 0.92), (0.72, 0.7)]);
            pen.line_c(&[(0.38, 0.81), (0.62, 0.81)], accent);
            pen.disc(0.8, 0.46, 0.04, accent);
        }
        _ => {}
    }
}
