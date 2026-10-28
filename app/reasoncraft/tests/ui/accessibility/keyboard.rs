//! The keyboard reaches every control: through the find bar, and to rows
//! of the virtual lists that are not laid out yet.

use super::*;
use crate::common::app_with_doc;

/// The name of the control with the keyboard focus, if any.
fn focused_name(h: &H) -> Option<String> {
    h.root()
        .children_recursive()
        .map(|n| n.accesskit_node())
        .find(|n| n.is_focused())
        .map(|n| name(&n))
}

/// Tab and Shift+Tab walk through the find bar's fields and buttons; the
/// map does not take them (Tab would add a reason).
#[test]
fn tab_cycles_through_find_bar() {
    let (d, _) = sample();
    let app = app_with_doc(d);
    let mut h = harness(app);
    h.key_press_modifiers(Modifiers::COMMAND, Key::H);
    h.run();
    h.get_by_label(&t("find.query")).type_text("They");
    h.run();
    let boxes = h.state().tabs[0].doc.boxes.len();
    let order = [
        "find.query",
        "find.match_case.tip",
        "find.whole_word.tip",
        "find.prev",
        "find.next",
        "find.close",
        "find.replacement",
        "find.replace",
        "find.replace_all",
    ]
    .map(t);
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
    assert_eq!(h.state().tabs[0].doc.boxes.len(), boxes, "nothing added");
    assert!(h.state().tabs[0].find.open);
}

/// The top and bottom of the Text panel's list: below its tabs, above
/// the status bar.
fn panel_view(h: &H) -> (f32, f32) {
    let top = h
        .get_by_role_and_label(accesskit::Role::Tab, &t("text.tab.essay"))
        .rect()
        .bottom();
    // The status bar's zoom slider.
    let zoom = t("status.zoom");
    let bottom = h
        .get_by_role_and_label(accesskit::Role::Slider, &zoom)
        .rect()
        .top();
    (top, bottom)
}

/// Press `key` (with `mods`) until `want` has the focus, at most 4 times.
fn tab_to(h: &mut H, mods: Modifiers, want: &str) {
    for _ in 0..4 {
        h.key_press_modifiers(mods, Key::Tab);
        h.run();
        if focused_name(h).as_deref() == Some(want) {
            return;
        }
    }
    panic!("{want:?} never got the focus: {:?}", focused_name(h));
}

/// With 400 statements in the Essay tab only the rows near the view are
/// laid out; Tab from the last field in view still reaches the next one,
/// and the list scrolls to show it.
#[test]
fn virtual_rows_reachable_by_keyboard() {
    let mut d = Document::new("Claim");
    let root = Parent::Box(d.root);
    for i in 0..400 {
        d.add_card(root, CardKind::Reason, &format!("Reason {i}"))
            .unwrap();
    }
    let mut app = app_with_doc(d);
    app.settings.text_tab = TextTab::Essay;
    let mut h = harness(app);
    let (top, bottom) = panel_view(&h);
    let shown: Vec<(String, egui::Rect)> = h
        .query_all_by_role(accesskit::Role::MultilineTextInput)
        .map(|n| (name(&n.accesskit_node()), n.rect()))
        .collect();
    let in_view = shown.iter().filter(|(_, r)| r.bottom() <= bottom);
    let (last, _) = in_view
        .max_by(|a, b| a.1.top().total_cmp(&b.1.top()))
        .cloned()
        .unwrap();
    let at = shown.iter().position(|(n, _)| *n == last).unwrap();
    let next = shown[at + 1].0.clone();
    h.get_by_label(&last).focus();
    h.run();
    assert_eq!(focused_name(&h).as_ref(), Some(&last));
    tab_to(&mut h, Modifiers::NONE, &next);
    let rect = h.get_by_label(&next).rect();
    assert!(
        rect.top() >= top && rect.bottom() <= bottom,
        "{rect:?} not in {top}..{bottom}"
    );
    // Shift+Tab goes back the same way.
    tab_to(&mut h, Modifiers::SHIFT, &last);
    assert_eq!(h.state().tabs[0].doc.boxes.len(), 401, "nothing added");
}

/// The role and name of the control with the keyboard focus.
fn focused_control(h: &H) -> Option<(accesskit::Role, String)> {
    h.root()
        .children_recursive()
        .map(|n| n.accesskit_node())
        .find(|n| n.is_focused())
        .map(|n| (n.role(), name(&n)))
}

/// F6 and Shift+F6 go round the panes from any control, landing on each
/// pane's first control; a hidden panel is skipped.
#[test]
fn f6_cycles_through_the_panes() {
    use accesskit::Role;
    let (d, _) = sample();
    let app = app_with_doc(d);
    let mut h = harness(app);
    let issues = h
        .query_all_by_role(Role::Button)
        .map(|n| name(&n.accesskit_node()))
        .find(|n| n.ends_with("issues"))
        .unwrap();
    let panes = [
        (Role::Tab, t("text.tab.outline")),
        (Role::Button, issues),
        (Role::Tab, t("ribbon.tab.file")),
        (Role::DisclosureTriangle, t("building.boxes")),
        (Role::Canvas, t("canvas.label")),
    ];
    let boxes = h.state().tabs[0].doc.boxes.len();
    // Nothing has the focus: the map has the keys, so F6 goes on from it.
    for want in panes.iter().chain(&panes) {
        h.key_press(Key::F6);
        h.run();
        assert_eq!(focused_control(&h).as_ref(), Some(want), "F6");
    }
    for want in panes.iter().rev().skip(1).chain(panes.iter().rev()) {
        h.key_press_modifiers(Modifiers::SHIFT, Key::F6);
        h.run();
        assert_eq!(focused_control(&h).as_ref(), Some(want), "Shift+F6");
    }
    // From a field too, and without the Building panel.
    h.state_mut().left_panel = false;
    h.key_press_modifiers(Modifiers::COMMAND, Key::F);
    h.run();
    h.key_press(Key::F6);
    h.run();
    assert_eq!(focused_control(&h).as_ref(), Some(&panes[0]), "map to text");
    h.key_press_modifiers(Modifiers::SHIFT, Key::F6);
    h.run();
    h.key_press_modifiers(Modifiers::SHIFT, Key::F6);
    h.run();
    assert_eq!(focused_control(&h).as_ref(), Some(&panes[2]), "skips it");
    assert_eq!(h.state().tabs[0].doc.boxes.len(), boxes, "nothing added");

    // The File tab's backstage is a pane of its own, in the map's place.
    h.key_press(Key::Escape);
    h.run();
    h.get_by_label(&t("ribbon.tab.file")).click();
    h.run();
    let back = (Role::Button, t("backstage.back"));
    // With nothing focused the backstage has the keys, so F6 goes on from it.
    let backstage = [panes[1].clone(), panes[2].clone(), back, panes[1].clone()];
    for want in &backstage {
        h.key_press(Key::F6);
        h.run();
        assert_eq!(
            focused_control(&h).as_ref(),
            Some(want),
            "F6 in the backstage"
        );
    }
    h.key_press_modifiers(Modifiers::SHIFT, Key::F6);
    h.run();
    assert_eq!(
        focused_control(&h).as_ref(),
        Some(&(Role::Button, t("backstage.back")))
    );
}

/// F6 into the ribbon lands on a hand-drawn button: it gets a focus
/// outline, which a click with the mouse does not leave behind.
#[test]
fn keyboard_focus_is_outlined_on_hand_drawn_controls() {
    use reasoncraft::a11y::focus_ring_rect;
    let mut h = harness(ReasonCraftApp::new_for_test());
    h.run();
    assert_eq!(focus_ring_rect(&h.ctx), None, "nothing focused");
    // F6 walks the panes; the ribbon comes round.
    for _ in 0..4 {
        h.key_press(Key::F6);
        h.run();
        if focus_ring_rect(&h.ctx).is_some_and(|r| r.top() < 200.0) {
            break;
        }
    }
    let ring = focus_ring_rect(&h.ctx).expect("a ring on the focused control");
    let name = focused_name(&h).expect("a control has the focus");
    let node = h.get_by_label(&name).rect();
    assert!(ring.contains(node.center()), "{name}: {ring:?} vs {node:?}");
    // A mouse click on a ribbon button leaves no ring.
    h.get_by_label(&t("ribbon.tab.view")).click();
    h.run();
    h.get_by_label(&t("action.zoom_fit")).click();
    h.run();
    assert_eq!(focus_ring_rect(&h.ctx), None, "no ring after a click");
}
