//! A ribbon row wider than the window first shrinks its groups from the
//! right (compact, then collapsed into a drop-down). Only when even that
//! does not fit does it scroll, and then it keeps its buttons where they
//! would be if it fitted: no room is kept for a scroll chevron, a chevron
//! shows only at an end with more beyond it, and the buttons do not move
//! when one appears.

use eframe::egui::{self, Rect, accesskit::Role};
use egui_kittest::{
    Harness,
    kittest::{NodeT, Queryable},
};
use reasoncraft::ReasonCraftApp;
use reasoncraft::app::RibbonTab;
use reasoncraft::i18n::t;

type H = Harness<'static, ReasonCraftApp>;

/// A window of `size` at 150 %, on `tab`.
fn harness_on(size: egui::Vec2, tab: RibbonTab) -> H {
    let mut app = ReasonCraftApp::new_for_test();
    app.settings.ui_scale = 1.5;
    app.ribbon_tab = tab;
    let mut h = Harness::builder()
        .with_size(size)
        .with_max_steps(32)
        .build_ui_state(|ui, app: &mut ReasonCraftApp| app.show(ui), app);
    h.run();
    h
}

/// A window of `size` at 150 %, on the Home tab.
fn harness(size: egui::Vec2) -> H {
    harness_on(size, RibbonTab::Home)
}

/// The smallest window on the View tab, whose groups overflow even all
/// collapsed: the row scrolls.
fn small() -> H {
    harness_on(egui::vec2(800.0, 500.0), RibbonTab::View)
}

fn left_arrow(h: &H) -> bool {
    h.query_by_label(&t("ribbon.scroll_left")).is_some()
}

fn right_arrow(h: &H) -> bool {
    h.query_by_label(&t("ribbon.scroll_right")).is_some()
}

/// The ribbon's command buttons (not the chevrons), left to right.
fn buttons(h: &H) -> Vec<(String, Rect)> {
    let strip = h
        .get_by_role_and_label(Role::Tab, &t("ribbon.tab.home"))
        .rect();
    let canvas = h.get_by_label(&t("canvas.label")).rect();
    let arrows = ["ribbon.scroll_left", "ribbon.scroll_right"].map(t);
    let mut out: Vec<(String, Rect)> = h
        .query_all_by_role(Role::Button)
        .map(|n| (n.accesskit_node().label().unwrap_or_default(), n.rect()))
        .filter(|(name, r)| {
            r.center().y > strip.bottom() && r.center().y < canvas.top() && !arrows.contains(name)
        })
        .collect();
    out.sort_by(|a, b| a.1.left().total_cmp(&b.1.left()));
    out
}

#[test]
fn an_overflowing_row_starts_where_a_fitting_row_does() {
    let wide = harness_on(egui::vec2(4000.0, 900.0), RibbonTab::View);
    assert!(!left_arrow(&wide) && !right_arrow(&wide), "it fits");
    let h = small();
    assert!(right_arrow(&h), "it overflows");
    let (first, at) = buttons(&h)[0].clone();
    let (wide_first, wide_at) = buttons(&wide)[0].clone();
    assert!(
        (at.left() - wide_at.left()).abs() < 0.5,
        "{first:?} starts at {} when overflowing, {wide_first:?} at {} when it fits",
        at.left(),
        wide_at.left()
    );
}

#[test]
fn chevrons_show_only_where_there_is_more() {
    let mut h = small();
    assert!(!left_arrow(&h), "no way back at the start");
    assert!(right_arrow(&h), "more to the right");
    let mut clicks = 0;
    while let Some(arrow) = h.query_by_label(&t("ribbon.scroll_right")) {
        arrow.click();
        h.run();
        clicks += 1;
        assert!(clicks < 30, "never reached the end");
    }
    assert!(left_arrow(&h), "the way back at the end");
    assert!(!right_arrow(&h));
}

/// A 2 point scroll (the left chevron shows past 1 point) moves the
/// buttons by 2 points, no more: nothing is made room for.
#[test]
fn buttons_do_not_jump_when_the_left_chevron_appears() {
    let mut h = small();
    let (name, before) = buttons(&h)[1].clone();
    h.hover_at(before.center());
    h.run();
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(-2.0, 0.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Unknown,
        modifiers: egui::Modifiers::NONE,
    });
    h.run();
    assert!(left_arrow(&h), "scrolled off the start");
    let after = h.get_by_label(&name).rect();
    let moved = before.left() - after.left();
    assert!(
        (moved - 2.0).abs() < 0.5,
        "{name:?} moved {moved} for a 2 point scroll"
    );
}

/// At the smallest window size, Home's groups shrink instead of scrolling:
/// a group collapses into one drop-down that opens the whole group, and
/// its commands run from there.
#[test]
fn a_collapsed_group_opens_and_runs_its_commands() {
    let mut h = harness(egui::vec2(800.0, 500.0));
    assert!(!left_arrow(&h) && !right_arrow(&h), "it fits, shrunk");
    let more = reasoncraft::i18n::t_args("ribbon.group_more", &{
        let mut args = reasoncraft::i18n::FluentArgs::new();
        args.set("group", t("group.add"));
        args
    });
    let reason = t("action.add_reason");
    assert!(h.query_by_label(&reason).is_none(), "folded away");
    h.get_by_label(&more).click();
    h.run();
    let boxes = h.state().tabs[0].doc.boxes.len();
    h.get_by_label(&reason).click();
    h.run();
    assert_eq!(
        h.state().tabs[0].doc.boxes.len(),
        boxes + 1,
        "a reason was added"
    );
}

/// Shrunk groups give their room back when the window grows again.
#[test]
fn groups_come_back_when_there_is_room() {
    let mut h = harness(egui::vec2(800.0, 500.0));
    let reason = t("action.add_reason");
    assert!(h.query_by_label(&reason).is_none());
    h.set_size(egui::vec2(4000.0, 900.0));
    h.run();
    assert!(h.query_by_label(&reason).is_some());
}

/// The keyboard opens a collapsed group too: Enter on its button, then Tab
/// to a command inside and Enter runs it.
#[test]
fn the_keyboard_opens_a_collapsed_group() {
    let mut h = harness(egui::vec2(800.0, 500.0));
    let mut args = reasoncraft::i18n::FluentArgs::new();
    args.set("group", t("group.add"));
    let more = reasoncraft::i18n::t_args("ribbon.group_more", &args);
    h.get_by_label(&more).focus();
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    let reason = t("action.add_reason");
    let focused = |h: &H| {
        h.root()
            .children_recursive()
            .find(|n| n.accesskit_node().is_focused())
            .and_then(|n| n.accesskit_node().label())
    };
    for _ in 0..10 {
        if focused(&h).as_deref() == Some(reason.as_str()) {
            break;
        }
        h.key_press(egui::Key::Tab);
        h.run();
    }
    assert_eq!(focused(&h).as_deref(), Some(reason.as_str()));
    let boxes = h.state().tabs[0].doc.boxes.len();
    h.key_press(egui::Key::Enter);
    h.run();
    assert_eq!(h.state().tabs[0].doc.boxes.len(), boxes + 1);
}

/// Clicking the right chevron by keyboard until it goes leaves the focus
/// on the row's last button, not nowhere.
#[test]
fn the_focus_moves_on_when_its_chevron_goes() {
    let mut h = small();
    let right = t("ribbon.scroll_right");
    h.get_by_label(&right).focus();
    h.run();
    let mut presses = 0;
    while right_arrow(&h) {
        h.key_press(egui::Key::Enter);
        h.run();
        presses += 1;
        assert!(presses < 30, "never reached the end");
    }
    let focused = h
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().is_focused())
        .and_then(|n| n.accesskit_node().label());
    let last = buttons(&h).last().map(|(name, _)| name.clone());
    assert!(focused.is_some(), "the focus went somewhere");
    assert_eq!(focused, last);
}
