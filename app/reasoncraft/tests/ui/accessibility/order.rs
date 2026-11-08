//! The keyboard walks the title bar in the order it is drawn, left to
//! right.

use super::*;

/// The name of the control with the keyboard focus, if any.
fn focused_name(h: &H) -> Option<String> {
    h.root()
        .children_recursive()
        .map(|n| n.accesskit_node())
        .find(|n| n.is_focused())
        .map(|n| name(&n))
}

/// From the quick Save button, Tab goes on to Undo, Redo, the command
/// search, then Minimize, Maximize and Close; Shift+Tab comes back.
#[test]
fn title_bar_tab_order_is_left_to_right() {
    let (d, reason) = sample();
    let mut h = harness(app_with_doc(d));
    // Something to undo and to redo, so both take the focus.
    act(&mut h, Action::Select(Some(Item::Box(reason))));
    act(&mut h, Action::Rate(reasoncraft::actions::Rate::High));
    act(&mut h, Action::Rate(reasoncraft::actions::Rate::Low));
    act(&mut h, Action::Undo);
    let mut args = FluentArgs::new();
    args.set("keys", "Alt+Q");
    let order = [
        t("action.save"),
        t("action.undo"),
        t("action.redo"),
        t_args("titlebar.search", &args),
        t("window.minimize"),
        t("window.maximize"),
        t("window.close"),
    ];
    h.query_all_by_label(&order[0])
        .min_by(|a, b| a.rect().top().total_cmp(&b.rect().top()))
        .unwrap()
        .focus();
    h.run();
    assert_eq!(focused_name(&h).as_ref(), Some(&order[0]));
    for want in &order[1..] {
        h.key_press(Key::Tab);
        h.run();
        assert_eq!(focused_name(&h).as_ref(), Some(want), "Tab");
    }
    for want in order[..order.len() - 1].iter().rev() {
        h.key_press_modifiers(Modifiers::SHIFT, Key::Tab);
        h.run();
        assert_eq!(focused_name(&h).as_ref(), Some(want), "Shift+Tab");
    }
}
