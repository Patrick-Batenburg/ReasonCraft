//! The smallest supported window (800×500) at a large interface size:
//! every ribbon command stays reachable by mouse and the map keeps room.

use eframe::egui;
use egui_kittest::{Harness, kittest::Queryable};
use reasoncraft::ReasonCraftApp;
use reasoncraft::app::RibbonTab;
use reasoncraft::i18n::t;

/// The minimum window at interface size `scale`.
pub(crate) fn harness(scale: f32) -> Harness<'static, ReasonCraftApp> {
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

/// The View tab overflows even with its groups collapsed: the right
/// chevron brings the last group into view, and its drop-down runs the
/// command.
#[test]
fn far_ribbon_commands_are_reachable_by_scrolling() {
    let mut h = harness(1.5);
    h.state_mut().ribbon_tab = RibbonTab::View;
    h.run();
    let mut args = reasoncraft::i18n::FluentArgs::new();
    args.set("group", t("group.evaluation"));
    let group = reasoncraft::i18n::t_args("ribbon.group_more", &args);
    let right = t("ribbon.scroll_right");
    assert!(h.query_by_label(&t("ribbon.scroll_left")).is_none());
    let mut clicks = 0;
    while h.get_by_label(&group).rect().right() > right_edge(&h) {
        h.get_by_label(&right).click();
        h.run();
        clicks += 1;
        assert!(clicks < 20, "the scroll button never reached it");
    }
    assert!(clicks > 0, "it started out of view");
    // Scrolled: the way back shows up.
    assert!(h.query_by_label(&t("ribbon.scroll_left")).is_some());
    h.get_by_label(&group).click();
    h.run();
    let before = h.state().settings.eval_shading;
    h.get_by_label(&t("eval.shading")).click();
    h.run();
    assert_ne!(h.state().settings.eval_shading, before);
}

/// On every tab, clicking the right arrow until it goes brings the last
/// button fully into the window.
#[test]
fn every_tab_scrolls_to_its_last_button() {
    use egui_kittest::kittest::NodeT;
    let tabs = [
        RibbonTab::Home,
        RibbonTab::Insert,
        RibbonTab::Evaluate,
        RibbonTab::View,
        RibbonTab::Export,
        RibbonTab::Help,
    ];
    let arrows = [t("ribbon.scroll_left"), t("ribbon.scroll_right")];
    for tab in tabs {
        let mut h = harness(1.5);
        h.state_mut().ribbon_tab = tab;
        h.run();
        let (top, bottom) = (110.0, 200.0);
        let last = |h: &Harness<'static, ReasonCraftApp>| {
            h.query_all_by_role(egui::accesskit::Role::Button)
                .map(|n| (n.accesskit_node().label().unwrap_or_default(), n.rect()))
                .filter(|(name, r)| {
                    r.center().y > top && r.center().y < bottom && !arrows.contains(name)
                })
                .max_by(|a, b| a.1.right().total_cmp(&b.1.right()))
                .expect("ribbon buttons")
        };
        let mut clicks = 0;
        while let Some(arrow) = h.query_by_label(&arrows[1]) {
            arrow.click();
            h.run();
            clicks += 1;
            assert!(clicks < 30, "{tab:?}: never reached the end");
        }
        let (name, rect) = last(&h);
        assert!(
            rect.right() <= right_edge(&h),
            "{tab:?}: {name:?} at {rect:?} still past the window"
        );
    }
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

#[test]
fn settings_page_fits_beside_the_backstage_entries() {
    let mut h = harness(1.5);
    h.state_mut().ribbon_tab = RibbonTab::File;
    h.run();
    // Below the window's bottom edge at this size: reached by keyboard.
    h.get_by_label(&t("backstage.settings")).focus();
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    let right = right_edge(&h);
    let dutch = reasoncraft::a11y::in_group(&t("settings.ui_language"), "Nederlands");
    for label in [dutch, t("settings.author")] {
        let node = h.query_by_label(&label);
        let rect = node
            .unwrap_or_else(|| panic!("{label}: not on screen"))
            .rect();
        assert!(rect.right() <= right, "{label}: {rect:?} past {right}");
    }
    // A hint wraps inside the window.
    let hint = h.get_by_label(&t("settings.ui_language_hint")).rect();
    assert!(hint.right() <= right, "hint {hint:?} past {right}");
    // The icon-only entries keep their names.
    assert!(h.query_by_label(&t("backstage.about")).is_some());
}

#[test]
fn text_panel_tabs_keep_to_their_panel() {
    use reasoncraft::settings::TextTab;
    for tab in [
        TextTab::Outline,
        TextTab::Essay,
        TextTab::Check,
        TextTab::Snippets,
        TextTab::Help,
    ] {
        let mut app = ReasonCraftApp::new_for_test();
        app.settings.ui_scale = 1.5;
        app.settings.text_tab = tab;
        let mut h = Harness::builder()
            .with_size(egui::vec2(800.0, 500.0))
            .build_ui_state(|ui, app: &mut ReasonCraftApp| app.show(ui), app);
        h.run();
        let map = h.state().tabs[0].viewport.expect("map shown");
        let panel_tab = h.get_by_label(&t("text.tab.outline")).rect();
        assert!(
            map.right() <= panel_tab.left(),
            "{tab:?}: the map ({map:?}) runs under the Text panel ({panel_tab:?})"
        );
    }
}

/// An icon-only backstage entry shows its label when the keyboard
/// reaches it, not only on hover.
#[test]
fn compact_backstage_entry_shows_its_label_on_focus() {
    let mut h = harness(1.5);
    h.state_mut().ribbon_tab = RibbonTab::File;
    h.run();
    let label = t("action.open");
    assert_eq!(h.query_all_by_label(&label).count(), 1, "the entry only");
    h.get_by_label(&label).focus();
    h.run();
    assert!(
        h.query_all_by_label(&label).count() >= 2,
        "the entry and its tooltip"
    );
}

/// At 200 % the ribbon's tabs do not fit 800 px: the tab row scrolls,
/// so the last tab (Help) is reached with the keyboard and the right
/// chevron, and comes into view when chosen.
#[test]
fn ribbon_tabs_scroll_when_they_do_not_fit() {
    let mut h = harness(2.0);
    let help = t("ribbon.tab.help");
    let right = right_edge(&h);
    let tab = |h: &Harness<'static, ReasonCraftApp>| {
        h.get_by_role_and_label(egui::accesskit::Role::Tab, &help)
            .rect()
    };
    assert!(tab(&h).right() > right, "Help starts out of view");
    // The tab row has a chevron of its own.
    h.get_by_label(&t("ribbon.tabs_right"));
    h.get_by_role_and_label(egui::accesskit::Role::Tab, &help)
        .focus();
    h.run();
    assert!(tab(&h).right() <= right + 0.5, "{:?}", tab(&h));
    h.get_by_role_and_label(egui::accesskit::Role::Tab, &help)
        .click();
    h.run();
    assert_eq!(h.state().ribbon_tab, RibbonTab::Help);
}

/// The window's minimum size is 800 × 500 interface points, so the
/// smallest window still holds the room the layout is made for at every
/// interface size. egui-winit turns points into pixels with the
/// interface size (egui's zoom factor) and gives the screen's size in
/// those points, so the size is sent as it is, and sent again when the
/// interface size changes (the same points are then more pixels). It is
/// never more than 90 % of the screen.
#[test]
fn minimum_window_size_follows_the_interface_size() {
    let app = ReasonCraftApp::new_for_test();
    let mut h = Harness::builder()
        .with_size(egui::vec2(1600.0, 1000.0))
        .build_ui_state(|ui, app: &mut ReasonCraftApp| app.show(ui), app);
    h.run();
    // Once the screen's size is known.
    let screen = egui::vec2(2560.0, 1440.0);
    for v in h.input_mut().viewports.values_mut() {
        v.monitor_size = Some(screen);
    }
    // What each of `n` frames sent, with the zoom factor in force.
    let sent = |h: &mut Harness<'_, ReasonCraftApp>, n: usize| {
        let mut out = Vec::new();
        for _ in 0..n {
            h.step();
            let zoom = h.ctx.zoom_factor();
            let min = (h.output().viewport_output.values())
                .flat_map(|v| v.commands.iter())
                .find_map(|c| match c {
                    egui::ViewportCommand::MinInnerSize(size) => Some(*size),
                    _ => None,
                });
            out.extend(min.map(|m| (m, zoom)));
        }
        out
    };
    let points = egui::vec2(800.0, 500.0);
    assert_eq!(sent(&mut h, 3), [(points, 1.0)], "sent once");
    h.state_mut().settings.ui_scale = 2.0;
    // The screen in the larger interface's points.
    for v in h.input_mut().viewports.values_mut() {
        v.monitor_size = Some(screen / 2.0);
    }
    // Sent again once the new zoom factor is in force (egui applies it
    // from the next pass on), as egui-winit then makes it 1600 × 1000
    // pixels at a display scale of 1.
    assert_eq!(sent(&mut h, 3), [(points, 2.0)], "again at 200 %");
    assert_eq!(
        reasoncraft::app::min_window_size(Some(egui::vec2(683.0, 384.0))),
        egui::vec2(683.0 * 0.9, 384.0 * 0.9),
        "never more than the screen holds"
    );
    assert_eq!(
        reasoncraft::app::min_window_size(None),
        egui::vec2(800.0, 500.0)
    );
}
