use eframe::egui::{self, Label, Ui, vec2};
use egui_kittest::{Harness, kittest::Queryable};

use super::*;

/// A list of rows 20 tall, or 60 where `tall`, estimated at 20.
struct List {
    heights: RowHeights<usize>,
    keys: Vec<usize>,
    tall: Vec<bool>,
}

fn harness(n: usize) -> Harness<'static, List> {
    let list = List {
        heights: RowHeights::default(),
        keys: (0..n).collect(),
        tall: vec![false; n],
    };
    Harness::builder()
        .with_size(vec2(300.0, 400.0))
        .with_max_steps(32)
        .build_ui_state(
            |ui, l: &mut List| {
                let tall = &l.tall;
                let draw = |ui: &mut Ui, i: usize| {
                    let h = if tall[i] { 60.0 } else { 20.0 };
                    ui.add_sized([ui.available_width(), h], Label::new(format!("row {i}")));
                };
                let rows = Rows {
                    keys: &l.keys,
                    estimate: 20.0,
                    revision: 1,
                    draw,
                };
                show(ui, &mut l.heights, rows);
            },
            list,
        )
}

/// The top of row `i` on screen, if it is laid out.
fn top_of(h: &Harness<'static, List>, i: usize) -> Option<f32> {
    h.query_by_label(&format!("row {i}"))
        .map(|n| n.rect().top())
}

#[test]
fn virtual_rows_do_not_jump_when_measured_above() {
    let mut h = harness(400);
    h.run();
    h.event(egui::Event::PointerMoved(egui::pos2(150.0, 200.0)));
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: vec2(0.0, -3000.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
        modifiers: egui::Modifiers::default(),
    });
    h.run();
    // The first row wholly in view; the one before it may be cut off.
    let k = (0..400)
        .find(|&i| top_of(&h, i).is_some_and(|y| y >= 0.0))
        .expect("scrolled to rows in view");
    assert!(k > 100, "scrolled well down: row {k}");
    let before = top_of(&h, k).unwrap();
    // The rows above the view turn out taller than measured: the ones
    // near the view are measured again, the others keep their heights.
    h.state_mut().tall[..k - 1].fill(true);
    // Not even for one frame.
    for _ in 0..4 {
        h.step();
        let after = top_of(&h, k).expect("still in view");
        assert!((after - before).abs() < 0.5, "row {k}: {before} -> {after}");
    }
    // Scrolling up past newly measured rows does not move it either,
    // beyond the scroll itself.
    let before = top_of(&h, k).unwrap();
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: vec2(0.0, 100.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
        modifiers: egui::Modifiers::default(),
    });
    h.run();
    let after = top_of(&h, k).expect("still in view");
    assert!(
        (after - before - 100.0).abs() < 0.5,
        "row {k}: {before} -> {after}"
    );
}
