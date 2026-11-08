//! The map for screen readers: the canvas says what is selected, has a
//! node per box and card on screen with the selection as its active
//! descendant, describes "Move to…", and the inline editor is named by
//! the box it edits.

use super::*;
use reasoncraft_core::Acceptance;

/// The canvas node: its value and its description.
fn canvas(h: &H) -> (String, Option<String>) {
    let node = h.get_by_role(accesskit::Role::Canvas);
    let n = node.accesskit_node();
    (n.value().unwrap_or_default(), n.description())
}

/// The label of the canvas's active descendant, if it has one.
fn active(h: &H) -> Option<String> {
    let node = h.get_by_role(accesskit::Role::Canvas);
    let n = node.accesskit_node();
    n.active_descendant().and_then(|d| d.label())
}

fn reason_text(n: usize) -> String {
    let mut args = FluentArgs::new();
    args.set("n", n);
    args.set("total", 2);
    t_args("a11y.map.position", &args)
}

#[test]
fn the_canvas_says_what_is_selected() {
    let (mut h, reason) = sample_harness();
    act(&mut h, Action::Select(None));
    let (value, _) = canvas(&h);
    assert_eq!(value, t("a11y.map.nothing"));
    assert_eq!(active(&h), None);

    act(&mut h, Action::Select(Some(Item::Box(reason))));
    act(&mut h, Action::SetAcceptance(Some(Acceptance::Accept)));
    let (value, _) = canvas(&h);
    for part in [
        t("a11y.map.supports"),
        t("a11y.map.accept"),
        "They recieve love".to_owned(),
    ] {
        assert!(value.contains(&part), "{part:?} in {value:?}");
    }
    // The card of the reason is first of the root's two cards.
    let card = h.state().tabs[0]
        .doc
        .cards
        .values()
        .find(|c| c.premises.contains(&reason))
        .unwrap()
        .id;
    act(&mut h, Action::Select(Some(Item::Card(card))));
    let (value, _) = canvas(&h);
    assert!(value.contains(&reason_text(1)), "{value:?}");
    let mut args = FluentArgs::new();
    args.set("role", t("a11y.map.supports"));
    assert!(value.contains(&t_args("a11y.map.card", &args)), "{value:?}");
}

#[test]
fn boxes_on_screen_are_nodes_and_the_selection_is_active() {
    let (mut h, _) = sample_harness();
    let root = h.state().tabs[0].doc.root;
    // A node per box and card shown, under the canvas, named by its
    // description.
    let canvas = h.get_by_role(accesskit::Role::Canvas);
    let items: Vec<String> = canvas
        .children()
        .filter(|n| n.accesskit_node().role() == accesskit::Role::TreeItem)
        .map(|n| n.accesskit_node().label().unwrap_or_default())
        .collect();
    assert!(
        items.iter().any(|l| l.contains("They recieve love")),
        "{items:?}"
    );
    assert!(
        items.iter().any(|l| l.contains("They scratch")),
        "{items:?}"
    );
    let claim = items.iter().find(|l| l.contains("Cats are fine pets"));
    assert!(
        claim.is_some_and(|l| l.starts_with(&t("a11y.map.claim"))),
        "{items:?}"
    );
    // The arrows move the active descendant.
    act(&mut h, Action::Select(Some(Item::Box(root))));
    assert!(active(&h).is_some_and(|l| l.contains("Cats are fine pets")));
    h.key_press(Key::ArrowDown);
    h.run();
    let now = active(&h).expect("an active descendant");
    assert!(!now.contains("Cats are fine pets"), "{now:?}");
}

#[test]
fn move_to_is_described_and_the_editor_is_named() {
    let (mut h, reason) = sample_harness();
    act(&mut h, Action::Select(Some(Item::Box(reason))));
    act(&mut h, Action::StartConnect);
    let (_, description) = canvas(&h);
    let description = description.expect("Move to… is described");
    assert!(description.contains("They recieve love"), "{description:?}");
    act(&mut h, Action::CancelConnect);
    assert_eq!(canvas(&h).1, None);

    act(&mut h, Action::Select(Some(Item::Box(reason))));
    act(&mut h, Action::Edit);
    let mut args = FluentArgs::new();
    args.set("what", t("a11y.map.supports"));
    let name = t_args("a11y.map.editing", &args);
    let editor = h.get_by_role(accesskit::Role::MultilineTextInput);
    assert_eq!(editor.accesskit_node().label(), Some(name));
}
