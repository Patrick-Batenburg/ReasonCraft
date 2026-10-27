//! In the smallest window at 150 % in Dutch (the longest words), the
//! keyboard reaches every ribbon button, each one scrolled into view
//! clear of the scroll arrows.

use eframe::egui::accesskit::Role;
use eframe::egui::{self, Key, Rect};
use egui_kittest::{
    Harness,
    kittest::{NodeT, Queryable},
};
use reasoncraft::ReasonCraftApp;
use reasoncraft::actions::Action;
use reasoncraft::i18n::t;

use super::lock;

const TABS: [&str; 6] = [
    "ribbon.tab.home",
    "ribbon.tab.insert",
    "ribbon.tab.evaluate",
    "ribbon.tab.view",
    "ribbon.tab.export",
    "ribbon.tab.help",
];

type H = Harness<'static, ReasonCraftApp>;

/// The band of the window holding the ribbon's command row.
fn band(h: &H) -> (f32, f32) {
    let strip = h
        .get_by_role_and_label(Role::Tab, &t("ribbon.tab.home"))
        .rect();
    let canvas = h.get_by_label(&t("canvas.label")).rect();
    (strip.bottom(), canvas.top())
}

/// The focused control's name and rect, if any.
fn focused(h: &H) -> Option<(String, Rect)> {
    h.root()
        .children_recursive()
        .find(|n| n.accesskit_node().is_focused())
        .map(|n| (n.accesskit_node().label().unwrap_or_default(), n.rect()))
}

#[test]
fn keyboard_reaches_every_ribbon_button_in_view() {
    let _lang = lock();
    let mut app = ReasonCraftApp::new_for_test();
    app.settings.ui_scale = 1.5;
    let mut h = Harness::builder()
        .with_size(egui::vec2(800.0, 500.0))
        .with_max_steps(32)
        .build_ui_state(|ui, app: &mut ReasonCraftApp| app.show(ui), app);
    h.state_mut()
        .push(Action::SetUiLanguage(Some("nl".to_owned())));
    h.run();
    let right = h.ctx.content_rect().right();
    let arrows = ["ribbon.scroll_left", "ribbon.scroll_right"].map(t);
    for key in TABS {
        h.get_by_role_and_label(Role::Tab, &t(key)).click();
        h.run();
        let (top, bottom) = band(&h);
        let in_band = |r: Rect| r.center().y > top && r.center().y < bottom;
        // The ribbon's enabled buttons, left to right.
        let mut buttons: Vec<(String, Rect)> = h
            .query_all_by_role(Role::Button)
            .filter(|n| !n.accesskit_node().is_disabled())
            .map(|n| (n.accesskit_node().label().unwrap_or_default(), n.rect()))
            .filter(|(name, r)| in_band(*r) && !arrows.contains(name))
            .collect();
        buttons.sort_by(|a, b| a.1.left().total_cmp(&b.1.left()));
        let (first, last) = (buttons[0].0.clone(), buttons.last().unwrap().0.clone());
        h.get_by_label(&first).focus();
        h.run();
        let mut seen = Vec::new();
        for _ in 0..60 {
            let Some((name, rect)) = focused(&h) else {
                break;
            };
            if !in_band(rect) {
                break;
            }
            if arrows.contains(&name) {
                // The scroll arrows take the focus too.
                h.key_press(Key::Tab);
                h.run();
                continue;
            }
            assert!(
                rect.left() >= 0.0 && rect.right() <= right,
                "{key}: {name:?} at {rect:?} is outside the window"
            );
            for arrow in &arrows {
                if let Some(a) = h.query_by_label(arrow) {
                    let a = a.rect();
                    let overlap = a.intersect(rect);
                    assert!(
                        overlap.width() <= 0.5,
                        "{key}: {name:?} {rect:?} under {arrow} {a:?}"
                    );
                }
            }
            seen.push(name);
            h.key_press(Key::Tab);
            h.run();
        }
        assert!(
            seen.contains(&last),
            "{key}: Tab never reached {last:?}: {seen:?}"
        );
    }
}
