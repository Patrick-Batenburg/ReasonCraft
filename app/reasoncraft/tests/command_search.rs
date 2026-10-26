//! UI tests for the command registry in the ribbon and the title bar's
//! Search commands box (Alt+Q).

use eframe::egui::{
    self, Key, Modifiers,
    accesskit::{Role, Toggled},
};
use egui_kittest::{
    Harness,
    kittest::{NodeT, Queryable},
};
use reasoncraft::actions::Action;
use reasoncraft::app::RibbonTab;
use reasoncraft::canvas::input::{shortcut_for, shortcut_text};
use reasoncraft::i18n::{FluentArgs, t, t_args};
use reasoncraft::{ReasonCraftApp, commands};

fn harness() -> Harness<'static, ReasonCraftApp> {
    Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .with_max_steps(32)
        .build_ui_state(
            |ui, app: &mut ReasonCraftApp| app.show(ui),
            ReasonCraftApp::new_for_test(),
        )
}

/// The search field's accessible name: its hint text.
fn field_label() -> String {
    let mut args = FluentArgs::new();
    let keys = shortcut_for(&Action::OpenCommandSearch).map(shortcut_text);
    args.set("keys", keys.unwrap());
    t_args("titlebar.search", &args)
}

fn field_focused(h: &Harness<'static, ReasonCraftApp>) -> bool {
    h.get_by_label(&field_label()).is_focused()
}

/// A listed result row: whether it is highlighted and whether it is greyed out.
#[derive(Debug)]
struct Row {
    highlighted: bool,
    disabled: bool,
}

/// The result row of command `id`, if listed.
fn row(h: &Harness<'static, ReasonCraftApp>, id: &str) -> Option<Row> {
    let label = t(commands::get(id).label);
    h.query_by_role_and_label(Role::ListItem, &label)
        .map(|n| Row {
            highlighted: n.accesskit_node().toggled() == Some(Toggled::True),
            disabled: n.accesskit_node().is_disabled(),
        })
}

/// Alt+Q, then type `query` into the box.
fn search_for(h: &mut Harness<'static, ReasonCraftApp>, query: &str) {
    h.key_press_modifiers(Modifiers::ALT, Key::Q);
    h.run();
    assert!(field_focused(h), "Alt+Q focuses the search box");
    h.get_by_label(&field_label()).type_text(query);
    h.run();
}

#[test]
fn ribbon_renders_all_tabs_with_registry() {
    let mut h = harness();
    for tab in [
        "ribbon.tab.home",
        "ribbon.tab.evaluate",
        "ribbon.tab.view",
        "ribbon.tab.export",
        "ribbon.tab.file",
    ] {
        h.get_by_label(&t(tab)).click();
        h.run();
    }
    assert_eq!(h.state().ribbon_tab, RibbonTab::File);
    // The backstage's file commands come from the registry too.
    h.get_by_label(&t(commands::get("file.save_as").label));
}

#[test]
fn alt_q_focuses_search_enter_runs() {
    let mut h = harness();
    h.run();
    h.state_mut().tabs[0].camera.zoom = 0.5;
    search_for(&mut h, "zoom 100");
    assert!(
        row(&h, "view.zoom_100").is_some(),
        "the list shows Zoom 100%"
    );
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state().tabs[0].camera.zoom, 1.0);
    // Running a command closes the list and leaves the box.
    assert!(row(&h, "view.zoom_100").is_none());
    assert!(!field_focused(&h));
    assert_eq!(h.get_by_label(&field_label()).value().as_deref(), Some(""));
}

#[test]
fn arrows_move_highlight_and_click_runs() {
    let mut h = harness();
    h.run();
    h.state_mut().tabs[0].camera.zoom = 0.5;
    search_for(&mut h, "zoom");
    let selected = |h: &Harness<'static, ReasonCraftApp>, id| row(h, id).unwrap().highlighted;
    // "Fit" comes first in the Zoom group; Down moves to "100%".
    assert!(selected(&h, "view.zoom_fit"));
    h.key_press(Key::ArrowDown);
    h.run();
    assert!(selected(&h, "view.zoom_100"));
    assert!(field_focused(&h), "Down keeps the focus in the box");
    h.key_press(Key::ArrowUp);
    h.run();
    assert!(selected(&h, "view.zoom_fit"));
    let label = t(commands::get("view.zoom_100").label);
    h.get_by_role_and_label(Role::ListItem, &label).click();
    h.run();
    assert_eq!(h.state().tabs[0].camera.zoom, 1.0);
}

#[test]
fn disabled_command_not_run() {
    let mut h = harness();
    h.state_mut().tabs[0].selection = None;
    h.run();
    let boxes = h.state().tabs[0].doc.boxes.len();
    search_for(&mut h, "add co-premise");
    let co_premise = row(&h, "home.add_co_premise").expect("listed");
    assert!(co_premise.disabled, "greyed out");
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state().tabs[0].doc.boxes.len(), boxes, "nothing ran");
    assert!(h.query_by_label(&t("command.disabled")).is_some());
    assert!(field_focused(&h), "the box keeps the focus");
}

#[test]
fn search_esc_restores_focus() {
    let mut h = harness();
    h.run();
    search_for(&mut h, "zoom");
    assert!(row(&h, "view.zoom_fit").is_some());
    h.key_press(Key::Escape);
    h.run();
    assert!(row(&h, "view.zoom_fit").is_none(), "the list closes");
    assert!(!field_focused(&h));
    assert_eq!(h.get_by_label(&field_label()).value().as_deref(), Some(""));
    // The map has the keys again: Ctrl+0 zooms.
    h.state_mut().tabs[0].camera.zoom = 0.5;
    h.key_press_modifiers(Modifiers::COMMAND, Key::Num0);
    h.run();
    assert_eq!(h.state().tabs[0].camera.zoom, 1.0);
}

#[test]
fn search_runs_from_backstage_and_returns_to_map() {
    let mut h = harness();
    h.get_by_label(&t("ribbon.tab.file")).click();
    h.run();
    // The backstage takes no shortcuts: click into the box instead.
    h.get_by_label(&field_label()).click();
    h.run();
    h.get_by_label(&field_label()).type_text("add reason");
    h.run();
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state().ribbon_tab, RibbonTab::Home);
}

#[test]
fn alt_q_while_editing_keeps_the_text() {
    let mut h = harness();
    h.run();
    let root = h.state().tabs[0].doc.root;
    h.event(egui::Event::Text("Robots".into()));
    h.run();
    assert_eq!(h.state().tabs[0].editing(), Some(root));
    h.key_press_modifiers(Modifiers::ALT, Key::Q);
    h.run();
    assert!(field_focused(&h));
    let tab = &h.state().tabs[0];
    assert_eq!(tab.editing(), None);
    assert_eq!(tab.doc.boxes[&root].text, "Robots");
    assert_eq!(tab.history.undo.len(), 1);
}
