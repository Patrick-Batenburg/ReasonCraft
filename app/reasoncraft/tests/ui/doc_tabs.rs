//! The row of open documents in a narrow window: chips shrink, then the
//! row shows the tabs around the active one with a menu of all of them;
//! the active tab and "+" always show. Ctrl+Tab, Ctrl+Shift+Tab,
//! Ctrl+PgDn, Ctrl+PgUp, Ctrl+W and Ctrl+F4 work the tabs, and Search
//! commands finds them.

use crate::common::H;
use eframe::egui::{self, Key, Modifiers, accesskit::Role};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use reasoncraft::ReasonCraftApp;
use reasoncraft::actions::Action;
use reasoncraft::i18n::t;

/// `n` untitled tabs in a window `width` wide, the last one active.
fn tabs(n: usize, width: f32) -> H {
    let mut app = ReasonCraftApp::new_for_test();
    for _ in 1..n {
        reasoncraft::actions::apply(&mut app, Action::New);
    }
    for (i, tab) in app.tabs.iter_mut().enumerate() {
        tab.doc.meta.title = format!("Map {i}");
    }
    let mut h = Harness::builder()
        .with_size(egui::vec2(width, 800.0))
        .with_max_steps(64)
        .build_ui_state(|ui, app: &mut ReasonCraftApp| app.show(ui), app);
    h.run();
    assert_eq!(h.state().tabs.len(), n);
    h
}

/// The document tab chips shown, left to right: (selected, rect).
fn chips(h: &H) -> Vec<(bool, egui::Rect)> {
    let mut out: Vec<(bool, egui::Rect)> = h
        .query_all_by_role(Role::Tab)
        .filter(|n| (36.0..66.0).contains(&n.rect().center().y))
        .map(|n| {
            let on = n.accesskit_node().toggled() == Some(egui::accesskit::Toggled::True);
            (on, n.rect())
        })
        .collect();
    out.sort_by(|a, b| a.1.left().total_cmp(&b.1.left()));
    out
}

/// The "+" button's rect.
fn plus(h: &H) -> egui::Rect {
    h.get_by_label(&t("tabs.new")).rect()
}

#[test]
fn few_tabs_show_in_full() {
    let h = tabs(3, 1280.0);
    let shown = chips(&h);
    assert_eq!(shown.len(), 3);
    assert!(
        h.query_by_label(&t("tabs.more")).is_none(),
        "no menu needed"
    );
}

#[test]
fn chips_shrink_before_any_hide() {
    // Seven chips of at least 120 px need 840 px: at 800 they shrink.
    let h = tabs(7, 800.0);
    let shown = chips(&h);
    assert_eq!(shown.len(), 7, "all still show");
    assert!(shown.iter().all(|(_, r)| r.width() < 120.0));
    assert!(plus(&h).right() <= 800.0);
    assert!(h.query_by_label(&t("tabs.more")).is_none());
}

#[test]
fn many_tabs_keep_the_active_one_and_plus_in_view() {
    let mut h = tabs(12, 800.0);
    let shown = chips(&h);
    assert!(shown.len() < 12, "some are left out");
    assert!(shown.iter().any(|(on, _)| *on), "the active tab shows");
    assert!(shown.iter().all(|(_, r)| r.right() <= 800.0));
    assert!(plus(&h).right() <= 800.0, "+ shows");
    // The menu lists every tab and switches to the one picked.
    h.get_by_label(&t("tabs.more")).click();
    h.run();
    let items: Vec<_> = h.query_all_by_label("Map 0").collect();
    assert_eq!(items.len(), 1, "only in the menu");
    items[0].click();
    h.run();
    assert_eq!(h.state().active, 0);
    assert!(chips(&h).iter().any(|(on, r)| *on && r.right() <= 800.0));
}

#[test]
fn keys_switch_and_close_tabs() {
    let mut h = tabs(3, 1280.0);
    assert_eq!(h.state().active, 2);
    h.key_press_modifiers(Modifiers::COMMAND, Key::Tab);
    h.run();
    assert_eq!(h.state().active, 0, "Ctrl+Tab wraps round");
    h.key_press_modifiers(Modifiers::COMMAND | Modifiers::SHIFT, Key::Tab);
    h.run();
    assert_eq!(h.state().active, 2, "Ctrl+Shift+Tab");
    h.key_press_modifiers(Modifiers::COMMAND, Key::PageUp);
    h.run();
    assert_eq!(h.state().active, 1, "Ctrl+PgUp");
    h.key_press_modifiers(Modifiers::COMMAND, Key::PageDown);
    h.run();
    assert_eq!(h.state().active, 2, "Ctrl+PgDn");
    let boxes = h.state().tabs[2].doc.boxes.len();
    assert_eq!(boxes, 1, "Ctrl+Tab added nothing");
    h.key_press_modifiers(Modifiers::COMMAND, Key::W);
    h.run();
    assert_eq!(h.state().tabs.len(), 2, "Ctrl+W closes the clean tab");
    h.key_press_modifiers(Modifiers::COMMAND, Key::F4);
    h.run();
    assert_eq!(h.state().tabs.len(), 1, "Ctrl+F4 too");
}

#[test]
fn search_commands_finds_the_tab_commands() {
    for id in ["file.next_tab", "file.prev_tab", "file.close_tab"] {
        let cmd = reasoncraft::commands::get(id);
        let found = reasoncraft::commands::search(&t(cmd.label), 12);
        assert!(found.iter().any(|c| c.id == id), "{id}");
    }
}

/// A chip too narrow for its title shows it whole in a tooltip.
#[test]
fn a_narrowed_chip_shows_its_title_on_hover() {
    let mut h = tabs(7, 800.0);
    let title = "Wide map title";
    h.state_mut().tabs[0].doc.meta.title = title.to_owned();
    h.run();
    let chip = h.get_by_role_and_label(Role::Tab, title);
    chip.hover();
    h.run_steps(30);
    let shown = h.query_all_by_label(title).count();
    assert!(shown >= 2, "the chip and its tooltip: {shown}");
}
