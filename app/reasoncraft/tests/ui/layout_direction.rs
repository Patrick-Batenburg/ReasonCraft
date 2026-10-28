//! UI tests for the per-document layout direction: the View ribbon's
//! "Layout direction" menu, undo and arrow keys.

use crate::common::harness_with_doc;
use eframe::egui::Key;
use egui_kittest::kittest::Queryable;
use reasoncraft::ReasonCraftApp;
use reasoncraft::actions::{Action, apply};
use reasoncraft::doc_tab::DocTab;
use reasoncraft::i18n::t;
use reasoncraft_core::{CardKind, Document, Item, LayoutDirection, NodeId, Parent};

use crate::common::H;

/// "Claim" with one reason card; returns the document and its premise.
fn doc() -> (Document, NodeId) {
    let mut doc = Document::new("Claim");
    let (_, premise) = doc
        .add_card(Parent::Box(doc.root), CardKind::Reason, "Reason")
        .unwrap();
    (doc, premise)
}

#[test]
fn set_direction_is_one_undo_step() {
    let mut app = ReasonCraftApp::new_for_test();
    app.tabs[0] = DocTab::from_document(doc().0);
    let undo_len = |app: &ReasonCraftApp| app.tabs[0].history.undo.len();
    let direction = |app: &ReasonCraftApp| app.tabs[0].doc.layout_direction;

    apply(
        &mut app,
        Action::SetLayoutDirection(LayoutDirection::LeftRight),
    );
    assert_eq!(direction(&app), LayoutDirection::LeftRight);
    assert_eq!(undo_len(&app), 1);
    // The same direction again records nothing.
    apply(
        &mut app,
        Action::SetLayoutDirection(LayoutDirection::LeftRight),
    );
    assert_eq!(undo_len(&app), 1);

    apply(&mut app, Action::Undo);
    assert_eq!(direction(&app), LayoutDirection::TopDown);
    apply(&mut app, Action::Redo);
    assert_eq!(direction(&app), LayoutDirection::LeftRight);
}

#[test]
fn ribbon_menu_sets_direction() {
    let mut h = harness_with_doc(doc().0);
    h.get_by_label(&t("ribbon.tab.view")).click();
    h.run();
    h.get_by_label(&t("action.layout_direction")).click();
    h.run();
    h.get_by_label(&t("layout_direction.bottom_up")).click();
    h.run();
    let tab = &h.state().tabs[0];
    assert_eq!(tab.doc.layout_direction, LayoutDirection::BottomUp);
    assert_eq!(tab.history.undo.len(), 1);
}

/// Arrow keys keep their on-screen meaning: left-to-right, Left goes
/// toward the parent and Right back to the child; bottom-up, Down goes
/// toward the parent.
#[test]
fn arrow_keys_follow_direction() {
    let (mut d, premise) = doc();
    let card = *d.cards.keys().next().unwrap();
    let root = d.root;
    d.layout_direction = LayoutDirection::LeftRight;
    let mut h = harness_with_doc(d);
    let selection = |h: &H| h.state().tabs[0].selection;
    h.state_mut().push(Action::Select(Some(Item::Box(premise))));
    h.run();
    h.key_press(Key::ArrowLeft);
    h.run();
    assert_eq!(selection(&h), Some(Item::Card(card)));
    h.key_press(Key::ArrowLeft);
    h.run();
    assert_eq!(
        selection(&h),
        Some(Item::Box(root)),
        "Left reaches the claim"
    );
    h.key_press(Key::ArrowRight);
    h.run();
    assert_eq!(selection(&h), Some(Item::Card(card)));

    h.state_mut().tabs[0].doc.layout_direction = LayoutDirection::BottomUp;
    h.key_press(Key::ArrowDown);
    h.run();
    assert_eq!(
        selection(&h),
        Some(Item::Box(root)),
        "Down reaches the claim"
    );
    h.key_press(Key::ArrowUp);
    h.run();
    assert_eq!(selection(&h), Some(Item::Card(card)));
}
