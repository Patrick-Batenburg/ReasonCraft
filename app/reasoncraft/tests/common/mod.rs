//! Helpers shared by the app's test binaries (each uses some of them).
#![allow(dead_code)]

use std::time::{Duration, Instant};

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

/// The longest a test waits for background work: generous, as several
/// test binaries may run side by side on a loaded machine.
pub const WAIT: Duration = Duration::from_secs(120);

/// The longest one frame may take while a test steps through background
/// work: nothing may wait on the UI thread (a held job waits two minutes).
pub const FRAME_LIMIT: Duration = Duration::from_secs(20);

/// Calls `done` until it returns true (at most [`WAIT`]); `what` says what
/// a test that gives up was waiting for.
pub fn poll_until(what: &str, mut done: impl FnMut() -> bool) {
    let start = Instant::now();
    while !done() {
        assert!(start.elapsed() < WAIT, "still waiting for {what}");
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// Steps `h` until `done` holds (at most [`WAIT`]), checking that no
/// frame blocks.
pub fn step_until(h: &mut H, done: impl Fn(&ReasonCraftApp) -> bool, what: &str) {
    poll_until(what, || {
        if done(h.state()) {
            return true;
        }
        let frame = Instant::now();
        h.step();
        assert!(frame.elapsed() < FRAME_LIMIT, "a frame blocked ({what})");
        false
    });
}

/// Steps `h` until no background work runs (see
/// `ReasonCraftApp::background_busy`), then runs it until the frames
/// settle, and again while settling starts new work (an image that comes
/// into view is decoded). Use it instead of `run` after anything that
/// starts background work: how long the work takes never matters.
pub fn settle(h: &mut H) {
    poll_until("the background work to end", || {
        if h.state().background_busy() {
            h.step();
            return false;
        }
        h.run_ok().is_some() && !h.state().background_busy()
    });
}
