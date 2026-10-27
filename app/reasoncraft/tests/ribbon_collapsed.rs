//! A collapsed ribbon group's drop-down, by mouse and keyboard: the menus
//! inside it open as submenus and run their items, Esc and Tab never
//! leave the focus nowhere, and a command done once closes it.

use eframe::egui::{self, Key, accesskit::Role};
use egui_kittest::{
    Harness,
    kittest::{NodeT, Queryable},
};
use reasoncraft::ReasonCraftApp;
use reasoncraft::app::RibbonTab;
use reasoncraft::i18n::{FluentArgs, t, t_args};
use reasoncraft::settings::MinimapMode;
use reasoncraft_core::SourceType;

type H = Harness<'static, ReasonCraftApp>;

/// The smallest window at 150 % on `tab`: Home collapses Add and
/// Editing, View collapses every group.
fn harness(tab: RibbonTab) -> H {
    let mut app = ReasonCraftApp::new_for_test();
    app.settings.ui_scale = 1.5;
    app.ribbon_tab = tab;
    let mut h = Harness::builder()
        .with_size(egui::vec2(800.0, 500.0))
        .with_max_steps(64)
        .build_ui_state(|ui, app: &mut ReasonCraftApp| app.show(ui), app);
    h.run();
    h
}

/// The name of group `key`'s button when it is collapsed.
fn more(key: &str) -> String {
    let mut args = FluentArgs::new();
    args.set("group", t(key));
    t_args("ribbon.group_more", &args)
}

/// Click `label` in group `key`'s open drop-down (its nodes are the
/// group button's children; the same name may be elsewhere too).
fn click_inside(h: &H, key: &str, label: &str) {
    let group = more(key);
    let under_group = |n: &egui_kittest::Node<'_>| {
        let mut p = n.parent();
        while let Some(q) = p {
            if q.accesskit_node().label().as_deref() == Some(group.as_str()) {
                return true;
            }
            p = q.parent();
        }
        false
    };
    h.query_all_by_role_and_label(Role::Button, label)
        .find(under_group)
        .unwrap_or_else(|| panic!("{label:?} in the {group:?} drop-down"))
        .click();
}

fn focused(h: &H) -> Option<String> {
    h.root()
        .children_recursive()
        .find(|n| n.accesskit_node().is_focused())
        .and_then(|n| n.accesskit_node().label())
}

/// Press `key` until `name` has the focus.
fn move_focus_to(h: &mut H, key: Key, name: &str) {
    for _ in 0..20 {
        if focused(h).as_deref() == Some(name) {
            return;
        }
        h.key_press(key);
        h.run();
    }
    panic!("{key:?} never reached {name:?}; at {:?}", focused(h));
}

fn a_source() -> (SourceType, String) {
    // Not the first ("Source"), named like the menu button.
    let st = SourceType::ALL[1].clone();
    let label = t(&format!("source.{}", st.key()));
    (st, label)
}

#[test]
fn a_menu_in_a_collapsed_group_runs_its_item_by_mouse() {
    let mut h = harness(RibbonTab::View);
    h.get_by_label(&more("group.overview")).click();
    h.run();
    click_inside(&h, "group.overview", &t("action.minimap"));
    h.run();
    h.get_by_label(&t("minimap.on")).click();
    h.run();
    assert_eq!(h.state().settings.minimap, MinimapMode::On);
    assert!(!egui::Popup::is_any_open(&h.ctx), "the drop-down closed");

    let mut h = harness(RibbonTab::Home);
    let boxes = h.state().tabs[0].doc.boxes.len();
    h.get_by_label(&more("group.add")).click();
    h.run();
    click_inside(&h, "group.add", &t("action.add_source"));
    h.run();
    h.get_by_label(&a_source().1).click();
    h.run();
    assert_eq!(h.state().tabs[0].doc.boxes.len(), boxes + 1);
}

#[test]
fn a_menu_in_a_collapsed_group_runs_its_item_by_keyboard() {
    let mut h = harness(RibbonTab::View);
    h.get_by_label(&more("group.overview")).focus();
    h.run();
    h.key_press(Key::Enter);
    h.run();
    move_focus_to(&mut h, Key::Tab, &t("action.minimap"));
    h.key_press(Key::Enter);
    h.run();
    move_focus_to(&mut h, Key::ArrowDown, &t("minimap.on"));
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state().settings.minimap, MinimapMode::On);
    assert!(!egui::Popup::is_any_open(&h.ctx), "the drop-down closed");

    let mut h = harness(RibbonTab::Home);
    let boxes = h.state().tabs[0].doc.boxes.len();
    h.get_by_label(&more("group.add")).focus();
    h.run();
    h.key_press(Key::Enter);
    h.run();
    move_focus_to(&mut h, Key::Tab, &t("action.add_source"));
    h.key_press(Key::Enter);
    h.run();
    move_focus_to(&mut h, Key::ArrowDown, &a_source().1);
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state().tabs[0].doc.boxes.len(), boxes + 1);
}

#[test]
fn esc_gives_the_focus_back_to_the_group_button() {
    let mut h = harness(RibbonTab::Home);
    let button = more("group.add");
    h.get_by_label(&button).focus();
    h.run();
    h.key_press(Key::Enter);
    h.run();
    move_focus_to(&mut h, Key::Tab, &t("action.add_reason"));
    h.key_press(Key::Escape);
    h.run();
    assert!(
        h.query_by_label(&t("action.add_reason")).is_none(),
        "closed"
    );
    assert_eq!(focused(&h), Some(button));
}

#[test]
fn tabbing_out_of_the_drop_down_closes_it() {
    let mut h = harness(RibbonTab::Home);
    h.get_by_label(&more("group.add")).focus();
    h.run();
    h.key_press(Key::Enter);
    h.run();
    // The last button of the Add group.
    let last = t("action.comment");
    move_focus_to(&mut h, Key::Tab, &last);
    h.key_press(Key::Tab);
    h.run();
    assert!(h.query_by_label(&last).is_none(), "the drop-down closed");
    let now = focused(&h).expect("the focus went somewhere");
    assert_eq!(now, more("group.editing"), "the next group's button");
}

#[test]
fn a_command_done_once_closes_the_drop_down_a_toggle_does_not() {
    let mut h = harness(RibbonTab::View);
    h.get_by_label(&more("group.overview")).click();
    h.run();
    let numbers = t("action.numbers");
    click_inside(&h, "group.overview", &numbers);
    h.run();
    assert!(h.state().settings.show_numbers);
    assert!(egui::Popup::is_any_open(&h.ctx), "a toggle keeps it open");

    let mut h = harness(RibbonTab::View);
    h.get_by_label(&more("group.zoom")).click();
    h.run();
    click_inside(&h, "group.zoom", &t("action.zoom_fit_selection"));
    h.run();
    assert!(!egui::Popup::is_any_open(&h.ctx), "Fit selection closes it");
}

/// A column of three small buttons ends above the group's label.
#[test]
fn small_buttons_end_above_the_group_label() {
    let mut app = ReasonCraftApp::new_for_test();
    app.settings.ui_scale = 1.5;
    let mut h = Harness::builder()
        .with_size(egui::vec2(3000.0, 900.0))
        .build_ui_state(|ui, app: &mut ReasonCraftApp| app.show(ui), app);
    h.run();
    let paste = h
        .get_by_role_and_label(Role::Button, &t("action.paste"))
        .rect();
    for key in ["action.copy_image", "action.remove_image", "action.comment"] {
        let label = t(key);
        let ribbon = h
            .query_all_by_label(&label)
            .map(|n| n.rect())
            .filter(|r| r.top() >= paste.top() - 0.5 && r.top() < paste.bottom());
        let mut found = false;
        for r in ribbon {
            found = true;
            assert!(
                r.bottom() <= paste.bottom() + 0.5,
                "{label:?} ends at {} below the large buttons ({})",
                r.bottom(),
                paste.bottom()
            );
        }
        assert!(found, "{label:?} on the ribbon");
    }
}
