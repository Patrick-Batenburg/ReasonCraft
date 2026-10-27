//! The ribbon fits a 1280 × 800 window in Dutch, the language with the
//! longer words.

use eframe::egui::accesskit::Role;
use eframe::egui::{self, Rect, epaint};
use egui_kittest::{
    Harness,
    kittest::{NodeT, Queryable},
};
use reasoncraft::ReasonCraftApp;
use reasoncraft::actions::Action;
use reasoncraft::i18n::{self, t};

use super::lock;

const WINDOW: egui::Vec2 = egui::vec2(1280.0, 800.0);

/// The ribbon tabs with groups of commands (File opens the backstage).
const TABS: [&str; 6] = [
    "ribbon.tab.home",
    "ribbon.tab.insert",
    "ribbon.tab.evaluate",
    "ribbon.tab.view",
    "ribbon.tab.export",
    "ribbon.tab.help",
];

/// Every text painted in the last frame, with its rect.
fn painted_texts(h: &Harness<'static, ReasonCraftApp>) -> Vec<(String, Rect)> {
    fn walk(s: &epaint::Shape, out: &mut Vec<(String, Rect)>) {
        match s {
            epaint::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
            epaint::Shape::Text(t) => {
                let rect = t.galley.rect.translate(t.pos.to_vec2());
                out.push((t.galley.job.text.clone(), rect));
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    for clipped in &h.output().shapes {
        walk(&clipped.shape, &mut out);
    }
    out
}

#[test]
fn ribbon_fits_in_dutch_at_1280() {
    let _lang = lock();
    let mut h = Harness::builder()
        .with_size(WINDOW)
        .with_max_steps(32)
        .build_ui_state(
            |ui, app: &mut ReasonCraftApp| app.show(ui),
            ReasonCraftApp::new_for_test(),
        );
    h.state_mut()
        .push(Action::SetUiLanguage(Some("nl".to_owned())));
    h.run();
    assert_eq!(i18n::ui_language(), "nl");
    for key in TABS {
        let label = t(key);
        h.get_by_role_and_label(Role::Tab, &label).click();
        h.run();
        let strip = h.get_by_role_and_label(Role::Tab, &label).rect();
        let canvas = h.get_by_label(&t("canvas.label")).rect();
        // The band of the window between the tab strip and the map.
        let in_ribbon = |r: Rect| r.center().y > strip.bottom() && r.center().y < canvas.top();
        // The Debug group (View tab) exists in debug builds only.
        let debug = ["group.debug", "action.load_sample", "action.source_icons"].map(t);
        let shown = |name: &String, r: Rect| in_ribbon(r) && !debug.contains(name);
        let buttons: Vec<(String, Rect)> = h
            .query_all_by_role(Role::Button)
            .map(|n| (n.accesskit_node().label().unwrap_or_default(), n.rect()))
            .filter(|(name, r)| shown(name, *r))
            .collect();
        assert!(!buttons.is_empty() || key.ends_with("insert") || key.ends_with("help"));
        let texts: Vec<_> = painted_texts(&h)
            .into_iter()
            .filter(|(name, r)| shown(name, *r))
            .collect();
        assert!(!texts.is_empty(), "{label} shows something");
        for (name, rect) in buttons.iter().chain(&texts) {
            assert!(
                rect.left() >= 0.0 && rect.right() <= WINDOW.x,
                "{label}: {name:?} at {rect:?} is outside the {} px window",
                WINDOW.x
            );
        }
        // Each button's label lies within the button. A name may put the
        // group before the label ("Card: Nil"). The scroll arrows of an
        // overflowing row are drawn arrows, not text.
        let arrows = ["ribbon.scroll_left", "ribbon.scroll_right"].map(t);
        for (name, rect) in buttons.iter().filter(|(n, _)| !arrows.contains(n)) {
            let text = texts
                .iter()
                .find(|(text, r)| name.ends_with(text.as_str()) && r.intersects(*rect))
                .unwrap_or_else(|| panic!("{label}: {name:?} paints its label"));
            assert!(
                rect.expand(0.5).contains_rect(text.1),
                "{label}: {name:?} text {:?} is clipped by its button {rect:?}",
                text.1
            );
        }
    }
}
