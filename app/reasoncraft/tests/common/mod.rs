//! Helpers shared by the UI test files (each uses some of them).
#![allow(dead_code)]

use eframe::egui::{self, epaint::Shape};
use egui_kittest::Harness;
use reasoncraft::ReasonCraftApp;

/// Each text painted in the last frame, with where its first line starts.
pub fn painted_texts(h: &Harness<'_, ReasonCraftApp>) -> Vec<(String, egui::Pos2)> {
    fn flatten(s: &Shape, out: &mut Vec<(String, egui::Pos2)>) {
        match s {
            Shape::Vec(v) => v.iter().for_each(|s| flatten(s, out)),
            Shape::Text(t) => out.push((t.galley.text().to_owned(), t.pos)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    for clipped in &h.output().shapes {
        flatten(&clipped.shape, &mut out);
    }
    out
}

/// The pointer's way from `from` to `to` over up to eight frames, never
/// stopping near the map's edge, where a drag scrolls the map.
pub fn glide(h: &Harness<'_, ReasonCraftApp>, from: egui::Pos2, to: egui::Pos2) -> Vec<egui::Pos2> {
    let view = h.state().tabs[0].viewport.expect("the map shows");
    let inner = view.shrink(reasoncraft::canvas::drag::EDGE_PX);
    (1..=8)
        .map(|i| from.lerp(to, i as f32 / 8.0))
        .filter(|p| !view.contains(*p) || inner.contains(*p))
        .collect()
}
