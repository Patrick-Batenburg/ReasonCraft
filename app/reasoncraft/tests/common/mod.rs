//! Helpers shared by the app's test binaries (each uses some of them).
#![allow(dead_code)]

use eframe::egui::{self, epaint::Shape};
use egui_kittest::{Harness, HarnessBuilder};
use reasoncraft::ReasonCraftApp;
use reasoncraft::doc_tab::DocTab;
use reasoncraft_core::Document;

/// The harness every UI test drives.
pub type H = Harness<'static, ReasonCraftApp>;

/// The usual test window: 1400×900, at most 32 steps per `run`. Tests
/// that need another size, scale or frame time start from this.
pub fn builder() -> HarnessBuilder<ReasonCraftApp> {
    Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .with_max_steps(32)
}

/// `app` in the usual window, before its first frame.
pub fn build(app: ReasonCraftApp) -> H {
    builder().build_ui_state(|ui, app: &mut ReasonCraftApp| app.show(ui), app)
}

/// `app` in the usual window, run until it settles.
pub fn harness(app: ReasonCraftApp) -> H {
    let mut h = build(app);
    h.run();
    h
}

/// A test app (see `ReasonCraftApp::new_for_test`) showing `doc`.
pub fn app_with_doc(doc: Document) -> ReasonCraftApp {
    let mut app = ReasonCraftApp::new_for_test();
    app.tabs[0] = DocTab::from_document(doc);
    app
}

/// [`harness`] for a test app showing `doc`.
pub fn harness_with_doc(doc: Document) -> H {
    harness(app_with_doc(doc))
}

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
