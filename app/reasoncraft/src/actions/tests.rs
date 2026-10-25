use super::*;

fn app() -> ReasonCraftApp {
    ReasonCraftApp::new_for_test()
}

fn tab(app: &ReasonCraftApp) -> &DocTab {
    &app.tabs[app.active]
}

#[test]
fn add_reason_to_root_selects_new_premise() {
    let mut a = app();
    apply(&mut a, Action::AddReason);
    let t = tab(&a);
    assert_eq!(t.doc.cards.len(), 1);
    let card = t.doc.cards.values().next().unwrap();
    assert_eq!(t.selection, Some(Item::Box(card.premises[0])));
    assert_eq!(t.history.undo.len(), 1);
}

#[test]
fn co_premise_goes_to_owning_card() {
    let mut a = app();
    apply(&mut a, Action::AddReason);
    apply(&mut a, Action::AddCoPremise);
    let t = tab(&a);
    let card = t.doc.cards.values().next().unwrap();
    assert_eq!(card.premises.len(), 2);
    assert_eq!(t.selection, Some(Item::Box(card.premises[1])));
}

#[test]
fn whole_card_target_and_card_selection() {
    let mut a = app();
    apply(&mut a, Action::AddReason);
    let c = *tab(&a).doc.cards.keys().next().unwrap();
    apply(&mut a, Action::TargetWholeCard(CardKind::Objection));
    assert_eq!(tab(&a).doc.cards[&c].children.len(), 1);
    apply(&mut a, Action::Select(Some(Item::Card(c))));
    apply(&mut a, Action::AddReason);
    assert_eq!(tab(&a).doc.cards[&c].children.len(), 2);
}

#[test]
fn nothing_selected_adds_loose_items() {
    let mut a = app();
    apply(&mut a, Action::Select(None));
    apply(&mut a, Action::AddObjection);
    apply(&mut a, Action::Select(None));
    apply(&mut a, Action::AddNote);
    assert_eq!(tab(&a).doc.parking.len(), 2);
}

#[test]
fn refused_edit_records_nothing() {
    let mut a = app();
    // Root takes cards, so a category is refused afterwards.
    apply(&mut a, Action::AddReason);
    let root = tab(&a).doc.root;
    apply(&mut a, Action::Select(Some(Item::Box(root))));
    apply(&mut a, Action::AddCategory);
    assert_eq!(tab(&a).history.undo.len(), 1);
    apply(&mut a, Action::Delete);
    assert_eq!(tab(&a).history.undo.len(), 1, "root cannot be deleted");
}

#[test]
fn undo_restores_and_fixes_selection() {
    let mut a = app();
    apply(&mut a, Action::AddReason);
    apply(&mut a, Action::Undo);
    let t = tab(&a);
    assert!(t.doc.cards.is_empty());
    assert_eq!(t.selection, Some(Item::Box(t.doc.root)));
    apply(&mut a, Action::Redo);
    assert_eq!(tab(&a).doc.cards.len(), 1);
}

#[test]
fn delete_selects_owner() {
    let mut a = app();
    apply(&mut a, Action::AddReason);
    apply(&mut a, Action::Delete);
    let t = tab(&a);
    assert!(t.doc.cards.is_empty());
    assert_eq!(t.selection, Some(Item::Box(t.doc.root)));
}

#[test]
fn close_last_tab_leaves_a_fresh_one() {
    let mut a = app();
    apply(&mut a, Action::New);
    assert_eq!(a.active, 1);
    apply(&mut a, Action::CloseTab(1));
    assert_eq!((a.tabs.len(), a.active), (1, 0));
    apply(&mut a, Action::CloseTab(0));
    assert_eq!((a.tabs.len(), a.active), (1, 0));
}

#[test]
fn zoom_is_clamped() {
    let mut a = app();
    apply(&mut a, Action::SetZoom(100.0));
    assert_eq!(tab(&a).camera.zoom, ZOOM_MAX);
    apply(&mut a, Action::Zoom100);
    apply(&mut a, Action::ZoomIn);
    assert!((tab(&a).camera.zoom - 1.25).abs() < 1e-6);
}

#[test]
fn zoom_buttons_keep_view_centre_fixed() {
    use eframe::egui::{Rect, pos2, vec2};
    let mut a = app();
    let view = Rect::from_min_size(pos2(220.0, 140.0), vec2(800.0, 500.0));
    a.tabs[0].viewport = Some(view);
    a.tabs[0].camera.offset = vec2(300.0, 180.0);
    let world = a.tabs[0].camera.screen_to_world(view.center());
    apply(&mut a, Action::ZoomIn);
    apply(&mut a, Action::ZoomIn);
    apply(&mut a, Action::ZoomOut);
    let cam = tab(&a).camera;
    assert!((cam.zoom - 1.25).abs() < 1e-5);
    assert!((cam.world_to_screen(world) - view.center()).length() < 1e-3);
    a.tabs[0].fit_pending = false;
    apply(&mut a, Action::ZoomFit);
    assert!(tab(&a).fit_pending);
}

#[test]
fn is_enabled_follows_attach_rules() {
    let mut a = app();
    let root = tab(&a).doc.root;
    // Root selected and empty: anything may be added; no co-premise without a card.
    assert!(is_enabled(tab(&a), &Action::AddCategory));
    assert!(!is_enabled(tab(&a), &Action::AddCoPremise));
    assert!(!is_enabled(tab(&a), &Action::Delete));
    assert!(!is_enabled(tab(&a), &Action::Undo));
    apply(&mut a, Action::AddReason);
    // A premise is selected: co-premise and whole-card targets apply, category does not.
    assert!(is_enabled(tab(&a), &Action::AddCoPremise));
    assert!(is_enabled(
        tab(&a),
        &Action::TargetWholeCard(CardKind::Reason)
    ));
    assert!(!is_enabled(tab(&a), &Action::AddCategory));
    assert!(is_enabled(tab(&a), &Action::Delete));
    assert!(is_enabled(tab(&a), &Action::Undo));
    let c = *tab(&a).doc.cards.keys().next().unwrap();
    apply(&mut a, Action::Select(Some(Item::Card(c))));
    assert!(!is_enabled(tab(&a), &Action::AddCategory));
    assert!(!is_enabled(tab(&a), &Action::AddNote));
    assert!(is_enabled(tab(&a), &Action::AddObjection));
    // The root now holds a card, so it refuses categories.
    apply(&mut a, Action::Select(Some(Item::Box(root))));
    assert!(!is_enabled(tab(&a), &Action::AddCategory));
    // Nothing selected: loose items, but no co-premise.
    apply(&mut a, Action::Select(None));
    assert!(is_enabled(tab(&a), &Action::AddCategory));
    assert!(!is_enabled(tab(&a), &Action::AddCoPremise));
    // Moves: the root never moves; a card may go to parking and back.
    let to_parking = Action::Move {
        item: Item::Card(c),
        to: Parent::Parking,
    };
    assert!(is_enabled(tab(&a), &to_parking));
    let root_move = Action::Move {
        item: Item::Box(root),
        to: Parent::Parking,
    };
    assert!(!is_enabled(tab(&a), &root_move));
}

#[test]
fn add_starts_editing_and_text_joins_the_add_step() {
    let mut a = app();
    apply(&mut a, Action::AddReason);
    let p = tab(&a).editing.expect("new premise is edited");
    apply(
        &mut a,
        Action::EditText {
            node: p,
            text: "Because".into(),
        },
    );
    apply(&mut a, Action::StopEditing);
    let t = tab(&a);
    assert_eq!(t.doc.boxes[&p].text, "Because");
    assert_eq!(t.history.undo.len(), 1);
    apply(&mut a, Action::Undo);
    assert!(tab(&a).doc.cards.is_empty());
}

#[test]
fn edit_session_records_one_step_only_when_text_changed() {
    let mut a = app();
    let root = tab(&a).doc.root;
    apply(&mut a, Action::Edit);
    apply(&mut a, edit_text(root, "A"));
    apply(&mut a, edit_text(root, "AB"));
    apply(&mut a, Action::StopEditing);
    assert_eq!(tab(&a).history.undo.len(), 1);
    assert_eq!(tab(&a).doc.boxes[&root].text, "AB");
    // Unchanged text: no step.
    apply(&mut a, Action::Edit);
    apply(&mut a, edit_text(root, "ABC"));
    apply(&mut a, edit_text(root, "AB"));
    apply(&mut a, Action::StopEditing);
    assert_eq!(tab(&a).history.undo.len(), 1);
    // Any other action commits first, so undo restores the pre-edit text.
    apply(&mut a, Action::TypeText("!".into()));
    assert_eq!(tab(&a).doc.boxes[&root].text, "AB!");
    apply(&mut a, Action::Undo);
    assert_eq!(tab(&a).doc.boxes[&root].text, "AB");
    assert_eq!(tab(&a).editing, None);
}

#[test]
fn navigate_moves_selection_and_move_action_reattaches() {
    let mut a = app();
    let root = tab(&a).doc.root;
    apply(&mut a, Action::AddReason);
    apply(&mut a, Action::StopEditing);
    apply(&mut a, Action::Navigate(Dir::Up));
    assert_eq!(tab(&a).selection, Some(Item::Box(root)));
    apply(&mut a, Action::Navigate(Dir::Up));
    assert_eq!(
        tab(&a).selection,
        Some(Item::Box(root)),
        "edges keep selection"
    );
    let c = *tab(&a).doc.cards.keys().next().unwrap();
    apply(
        &mut a,
        Action::Move {
            item: Item::Card(c),
            to: Parent::Parking,
        },
    );
    assert_eq!(tab(&a).doc.parking, vec![Item::Card(c)]);
    assert_eq!(tab(&a).selection, Some(Item::Card(c)));
    apply(
        &mut a,
        Action::Move {
            item: Item::Card(c),
            to: Parent::Box(root),
        },
    );
    assert!(tab(&a).doc.parking.is_empty());
    assert_eq!(tab(&a).history.undo.len(), 3);
}

fn edit_text(node: reasoncraft_core::NodeId, text: &str) -> Action {
    Action::EditText {
        node,
        text: text.into(),
    }
}

#[test]
fn edit_text_for_another_box_is_dropped() {
    let mut a = app();
    let root = tab(&a).doc.root;
    apply(&mut a, Action::AddReason);
    let p = tab(&a).editing.unwrap();
    // A late update for the root must not land on the new premise.
    apply(&mut a, edit_text(root, "stale"));
    assert_eq!(tab(&a).doc.boxes[&p].text, "");
    assert_eq!(tab(&a).doc.boxes[&root].text, "");
    assert_eq!(tab(&a).edit_text, "");
}

#[test]
fn whole_card_target_enabled_matches_attach_rules() {
    let mut a = app();
    apply(&mut a, Action::AddReason);
    for kind in [CardKind::Reason, CardKind::Objection] {
        assert!(is_enabled(tab(&a), &Action::TargetWholeCard(kind)));
    }
    apply(&mut a, Action::Select(None));
    assert!(!is_enabled(
        tab(&a),
        &Action::TargetWholeCard(CardKind::Reason)
    ));
}
