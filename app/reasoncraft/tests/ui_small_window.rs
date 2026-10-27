//! The smallest supported window (800×500) at a large interface size:
//! every ribbon command stays reachable by mouse and the map keeps room.

use eframe::egui;
use egui_kittest::{Harness, kittest::Queryable};
use reasoncraft::ReasonCraftApp;
use reasoncraft::app::RibbonTab;
use reasoncraft::i18n::t;

/// The minimum window at interface size `scale`.
fn harness(scale: f32) -> Harness<'static, ReasonCraftApp> {
    let mut app = ReasonCraftApp::new_for_test();
    app.settings.ui_scale = scale;
    let mut h = Harness::builder()
        .with_size(egui::vec2(800.0, 500.0))
        .with_max_steps(32)
        .build_ui_state(|ui, app: &mut ReasonCraftApp| app.show(ui), app);
    h.run();
    h
}

/// The window's right edge, in the units the nodes report.
fn right_edge(h: &Harness<'static, ReasonCraftApp>) -> f32 {
    h.ctx.content_rect().right()
}

#[test]
fn far_ribbon_commands_are_reachable_by_scrolling() {
    let mut h = harness(1.5);
    h.state_mut().ribbon_tab = RibbonTab::View;
    h.run();
    let shading = t("eval.shading");
    let right = t("ribbon.scroll_right");
    assert!(h.query_by_label(&t("ribbon.scroll_left")).is_none());
    let mut clicks = 0;
    while h.get_by_label(&shading).rect().right() > right_edge(&h) {
        h.get_by_label(&right).click();
        h.run();
        clicks += 1;
        assert!(clicks < 20, "the scroll button never reached it");
    }
    assert!(clicks > 0, "it started out of view");
    // Scrolled: the way back shows up.
    assert!(h.query_by_label(&t("ribbon.scroll_left")).is_some());
    let before = h.state().settings.eval_shading;
    h.get_by_label(&shading).click();
    h.run();
    assert_ne!(h.state().settings.eval_shading, before);
}

#[test]
fn a_ribbon_that_fits_has_no_scroll_buttons() {
    let mut app = ReasonCraftApp::new_for_test();
    app.ribbon_tab = RibbonTab::Export;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .build_ui_state(|ui, app: &mut ReasonCraftApp| app.show(ui), app);
    h.run();
    assert!(h.query_by_label(&t("ribbon.scroll_right")).is_none());
    assert!(h.query_by_label(&t("ribbon.scroll_left")).is_none());
}

#[test]
fn map_keeps_room_beside_both_panels() {
    use reasoncraft::shell::MAP_MIN_WIDTH;
    for scale in [1.0, 1.5] {
        let h = harness(scale);
        assert!(h.state().left_panel && h.state().right_panel);
        let map = h.state().tabs[0].viewport.expect("map shown");
        assert!(
            map.width() >= MAP_MIN_WIDTH - 1.0,
            "at {scale}: map {} wide",
            map.width()
        );
    }
}
