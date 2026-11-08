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

/// A squeezed panel gets its width back when the window has room again.
#[test]
fn squeezed_panels_get_their_width_back() {
    let mut h = harness(1.5);
    let text_left =
        |h: &Harness<'static, ReasonCraftApp>| h.get_by_label(&t("text.tab.outline")).rect().left();
    let right = |h: &Harness<'static, ReasonCraftApp>| h.ctx.content_rect().right();
    let squeezed = right(&h) - text_left(&h);
    h.state_mut().settings.ui_scale = 1.0;
    for _ in 0..3 {
        h.run();
    }
    let back = right(&h) - text_left(&h);
    assert!(
        back > squeezed + 100.0,
        "the Text panel is {back} wide again (was {squeezed})"
    );
}

/// Shrinking the window while a box is being edited keeps that box in
/// view (and in edit mode).
#[test]
fn edited_box_stays_in_view_when_the_window_shrinks() {
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_ui_state(
            |ui, app: &mut ReasonCraftApp| app.show(ui),
            ReasonCraftApp::new_for_test(),
        );
    h.run();
    let root = h.state().tabs[0].doc.root;
    h.state_mut().tabs[0].begin_edit(root, false);
    h.run();
    // Put the box near the right edge of the large view.
    let view = h.state().tabs[0].viewport.expect("map shown");
    // The box's text field, where the box is drawn.
    let shown = |h: &Harness<'static, ReasonCraftApp>| {
        h.get_by_role(egui::accesskit::Role::MultilineTextInput)
            .rect()
    };
    let now = shown(&h);
    h.state_mut().tabs[0].camera.offset.x += view.right() - 30.0 - now.right();
    h.run();
    h.set_size(egui::vec2(800.0, 500.0));
    for _ in 0..3 {
        h.run();
    }
    let view = h.state().tabs[0].viewport.expect("map shown");
    let rect = shown(&h);
    assert!(view.contains_rect(rect), "{rect:?} outside {view:?}");
    assert_eq!(h.state().tabs[0].editing(), Some(root));
}

/// eframe storage held in memory.
#[derive(Default)]
struct MemStorage(std::collections::HashMap<String, String>);

impl eframe::Storage for MemStorage {
    fn get_string(&self, key: &str) -> Option<String> {
        self.0.get(key).cloned()
    }
    fn set_string(&mut self, key: &str, value: String) {
        self.0.insert(key.to_owned(), value);
    }
    fn remove_string(&mut self, key: &str) {
        self.0.remove(key);
    }
    fn flush(&mut self) {}
}

/// Drag the Text panel's edge from its left side to `x`.
fn drag_text_panel_to(h: &mut Harness<'static, ReasonCraftApp>, x: f32) {
    use reasoncraft::shell::panes::Pane;
    let rect = h.state().pane_rect(Pane::Text).expect("Text panel shown");
    let from = egui::pos2(rect.left() + 2.0, rect.center().y);
    h.event(egui::Event::PointerMoved(from));
    h.run();
    h.drag_at(from);
    h.run();
    for at in [egui::pos2(from.x - 20.0, from.y), egui::pos2(x, from.y)] {
        h.event(egui::Event::PointerMoved(at));
        h.run();
    }
    h.event(egui::Event::PointerButton {
        pos: egui::pos2(x, from.y),
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    h.run();
}

/// A width the user dragged survives a restart in a small window: egui's
/// memory (as eframe saves it) brings it back once there is room.
#[test]
fn a_dragged_panel_width_survives_a_restart_in_a_small_window() {
    use reasoncraft::shell::panes::Pane;
    let build = |size: egui::Vec2, scale: f32| {
        let mut app = ReasonCraftApp::new_for_test();
        app.settings.ui_scale = scale;
        Harness::builder()
            .with_size(size)
            .build_ui_state(|ui, app: &mut ReasonCraftApp| app.show(ui), app)
    };
    let big = egui::vec2(1280.0, 800.0);
    let mut h = build(big, 1.0);
    h.run();
    drag_text_panel_to(&mut h, 1280.0 - 420.0);
    let width =
        |h: &Harness<'static, ReasonCraftApp>| h.state().pane_rect(Pane::Text).unwrap().width();
    let dragged = width(&h);
    assert!(dragged > 380.0, "dragged to {dragged}");
    // Closed and started again in a small window, egui's memory saved
    // and loaded the way eframe does it (temporary data is dropped).
    let mut storage = MemStorage::default();
    eframe::set_value(&mut storage, "egui", &h.ctx.memory(|m| m.clone()));
    let mut h = build(egui::vec2(800.0, 500.0), 1.5);
    let memory: egui::Memory = eframe::get_value(&storage, "egui").unwrap();
    h.ctx.memory_mut(|m| *m = memory);
    h.run();
    h.run();
    eframe::set_value(&mut storage, "egui", &h.ctx.memory(|m| m.clone()));
    // And again in the large window.
    let mut h = build(big, 1.0);
    let memory: egui::Memory = eframe::get_value(&storage, "egui").unwrap();
    h.ctx.memory_mut(|m| *m = memory);
    h.run();
    h.run();
    assert!(
        (width(&h) - dragged).abs() < 1.0,
        "back to {}, not {dragged}",
        width(&h)
    );
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
