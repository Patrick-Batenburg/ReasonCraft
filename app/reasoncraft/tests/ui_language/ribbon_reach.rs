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

/// The labels of the ribbon's menu buttons, whose items open in a submenu
/// inside a collapsed group's drop-down.
const MENUS: [&str; 8] = [
    "action.add_source",
    "action.box_type",
    "action.ui_language",
    "action.layout_direction",
    "action.connector_style",
    "action.minimap",
    "export.outline",
    "export.essay",
];

/// Whether `n` lies inside the control called `name` (a drop-down's nodes
/// are its button's children).
fn under(n: &egui_kittest::Node<'_>, name: &str) -> bool {
    let mut p = n.parent();
    while let Some(q) = p {
        if q.accesskit_node().label().as_deref() == Some(name) {
            return true;
        }
        p = q.parent();
    }
    false
}

/// The focused control's name, if it lies inside `name`.
fn focused_under(h: &H, name: &str) -> Option<String> {
    h.root()
        .children_recursive()
        .find(|n| n.accesskit_node().is_focused())
        .filter(|n| under(n, name))
        .map(|n| n.accesskit_node().label().unwrap_or_default())
}

/// Open the collapsed group `group` from the keyboard.
fn open(h: &mut H, group: &str) {
    h.get_by_label(group).focus();
    h.run();
    h.key_press(Key::Enter);
    h.run();
}

/// In the same window, the keyboard opens every collapsed group, reaches
/// each enabled control in its drop-down, and opens each menu there (a
/// submenu) and walks its items; Esc gives the focus back to the group.
#[test]
fn keyboard_reaches_every_command_in_collapsed_groups() {
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
    let mut args = reasoncraft::i18n::FluentArgs::new();
    args.set("group", "X");
    let suffix = reasoncraft::i18n::t_args("ribbon.group_more", &args)[1..].to_owned();
    let menus = MENUS.map(t);
    let (mut collapsed_seen, mut menus_seen) = (0, 0);
    for key in TABS {
        h.get_by_role_and_label(Role::Tab, &t(key)).click();
        h.run();
        let (top, bottom) = band(&h);
        let groups: Vec<String> = h
            .query_all_by_role(Role::Button)
            .filter(|n| {
                let r = n.rect();
                r.center().y > top && r.center().y < bottom
            })
            .filter_map(|n| n.accesskit_node().label())
            .filter(|name| name.ends_with(&suffix))
            .collect();
        for group in groups {
            collapsed_seen += 1;
            open(&mut h, &group);
            let inside: Vec<String> = h
                .query_all_by_role(Role::Button)
                .filter(|n| under(n, &group) && !n.accesskit_node().is_disabled())
                .filter_map(|n| n.accesskit_node().label())
                .collect();
            let shown = h
                .query_all_by_role(Role::Button)
                .filter(|n| under(n, &group))
                .count();
            // A group may be all disabled (no card selected to rate).
            assert!(shown > 0, "{key}: {group:?} opens");
            let mut seen = Vec::new();
            for _ in 0..40 {
                h.key_press(Key::Tab);
                h.run();
                match focused_under(&h, &group) {
                    Some(name) if !seen.contains(&name) => seen.push(name),
                    _ => break,
                }
            }
            for name in &inside {
                assert!(
                    seen.contains(name),
                    "{key}: Tab never reached {name:?} in {group:?}: {seen:?}"
                );
            }
            // Tabbing out closed it.
            assert!(
                !egui::Popup::is_any_open(&h.ctx),
                "{key}: {group:?} stays open"
            );
            for menu in inside.iter().filter(|n| menus.contains(n)) {
                menus_seen += 1;
                open(&mut h, &group);
                let button = h
                    .query_all_by_role_and_label(Role::Button, menu)
                    .find(|n| under(n, &group))
                    .expect("the menu button");
                button.focus();
                h.run();
                h.key_press(Key::Enter);
                h.run();
                let items: Vec<String> = h
                    .query_all_by_role(Role::Button)
                    .filter(|n| under(n, menu) && !n.accesskit_node().is_disabled())
                    .filter_map(|n| n.accesskit_node().label())
                    .collect();
                assert!(!items.is_empty(), "{key}: {menu:?} in {group:?} opens");
                let mut walked = Vec::new();
                for _ in 0..40 {
                    match focused_under(&h, menu) {
                        Some(name) if !walked.contains(&name) => walked.push(name),
                        _ => break,
                    }
                    h.key_press(Key::ArrowDown);
                    h.run();
                }
                for item in &items {
                    assert!(
                        walked.contains(item),
                        "{key}: {item:?} of {menu:?} not reached: {walked:?}"
                    );
                }
                h.key_press(Key::Escape);
                h.run();
                assert!(
                    !egui::Popup::is_any_open(&h.ctx),
                    "{key}: Esc closes {menu:?}"
                );
                let back = h
                    .root()
                    .children_recursive()
                    .find(|n| n.accesskit_node().is_focused())
                    .and_then(|n| n.accesskit_node().label());
                assert_eq!(
                    back.as_deref(),
                    Some(group.as_str()),
                    "{key}: Esc gives the focus back"
                );
            }
        }
    }
    assert!(collapsed_seen > 0, "some group collapses at this size");
    assert!(menus_seen >= 4, "menus walked: {menus_seen}");
}
