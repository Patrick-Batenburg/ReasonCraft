//! UI tests for editing the map: keyboard building, drags and comments.

use crate::common::{app_with_doc, build};
use eframe::egui;
use egui_kittest::{
    Harness,
    kittest::{NodeT, Queryable},
};
use reasoncraft::ReasonCraftApp;
use reasoncraft::i18n::t;

fn harness() -> Harness<'static, ReasonCraftApp> {
    build(ReasonCraftApp::new_for_test())
}

fn harness_with(doc: reasoncraft_core::Document) -> Harness<'static, ReasonCraftApp> {
    build(app_with_doc(doc))
}

/// Press and release the primary button at `pos`.
fn click_at(h: &mut Harness<'static, ReasonCraftApp>, pos: egui::Pos2) {
    h.event(egui::Event::PointerMoved(pos));
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run();
}

// ---- Editing interactions ----

fn selected_text(h: &Harness<'static, ReasonCraftApp>) -> String {
    let tab = &h.state().tabs[0];
    match tab.selection {
        Some(reasoncraft_core::Item::Box(b)) => tab.doc.boxes[&b].text.clone(),
        other => panic!("expected a selected box, got {other:?}"),
    }
}

fn type_text(h: &mut Harness<'static, ReasonCraftApp>, text: &str) {
    h.event(egui::Event::Text(text.to_owned()));
    h.run();
}

#[test]
fn keyboard_builds_map() {
    use egui::{Key, Modifiers};
    let mut h = harness();
    h.run();
    h.key_press(Key::Tab);
    h.run();
    assert!(
        h.state().tabs[0].editing().is_some(),
        "a new reason is edited"
    );
    type_text(&mut h, "Reason one");
    h.key_press(Key::Escape);
    h.run();
    assert_eq!(h.state().tabs[0].editing(), None);
    assert_eq!(selected_text(&h), "Reason one");

    h.key_press_modifiers(Modifiers::SHIFT, Key::Tab);
    h.run();
    type_text(&mut h, "Objection");
    h.key_press(Key::Escape);
    h.run();
    let tab = &h.state().tabs[0];
    assert_eq!(tab.doc.cards.len(), 2);
    assert_eq!(selected_text(&h), "Objection");
    // An add and its first text are one step.
    assert_eq!(h.state().tabs[0].history.undo.len(), 2);

    h.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    h.run();
    let tab = &h.state().tabs[0];
    assert_eq!(tab.doc.cards.len(), 1);
    let card = tab.doc.cards.values().next().unwrap();
    assert_eq!(tab.doc.boxes[&card.premises[0]].text, "Reason one");
}

#[test]
fn typing_on_selected_root_starts_editing_and_records_one_step() {
    use egui::Key;
    let mut h = harness();
    h.run();
    let root = h.state().tabs[0].doc.root;
    type_text(&mut h, "R");
    assert_eq!(h.state().tabs[0].editing(), Some(root));
    type_text(&mut h, "obots help");
    h.key_press(Key::Escape);
    h.run();
    let tab = &h.state().tabs[0];
    assert_eq!(tab.doc.boxes[&root].text, "Robots help");
    assert_eq!(tab.history.undo.len(), 1);
    assert_eq!(tab.history.undo[0].boxes[&root].text, "");
}

#[test]
fn whole_card_target_via_plus_menu() {
    use reasoncraft_core::{CardKind, Document, Item, Parent};
    let mut doc = Document::new("Claim");
    let root = doc.root;
    let (c, p) = doc
        .add_card(Parent::Box(root), CardKind::Reason, "One")
        .unwrap();
    let mut h = harness_with(doc);
    h.run();

    // With the root selected, Category is greyed out everywhere: the root holds a card.
    h.get_by_label(&t("plus.open")).click();
    h.run();
    let category = t("action.add_category");
    let nodes: Vec<_> = h.get_all_by_label(&category).collect();
    assert!(nodes.len() >= 2, "ribbon and plus menu both list Category");
    assert!(nodes.iter().all(|n| n.accesskit_node().is_disabled()));
    h.key_press(egui::Key::Escape);
    h.run();

    h.state_mut().tabs[0].selection = Some(Item::Box(p));
    h.run();
    h.get_by_label(&t("plus.open")).click();
    h.run();
    h.get_by_label(&t("plus.whole_card_objection")).click();
    h.run();
    let tab = &h.state().tabs[0];
    assert_eq!(tab.doc.cards[&c].children.len(), 1);
    let new = tab.doc.cards[&c].children[0];
    assert_eq!(tab.doc.cards[&new].kind, CardKind::Objection);
    let premise = tab.doc.cards[&new].premises[0];
    assert_eq!(tab.selection, Some(Item::Box(premise)));
    assert_eq!(tab.editing(), Some(premise));
}

/// Press at `from`, move to `to` over several frames, release.
fn drag(h: &mut Harness<'static, ReasonCraftApp>, from: egui::Pos2, to: egui::Pos2) {
    h.event(egui::Event::PointerMoved(from));
    h.drag_at(from);
    h.run();
    for i in 1..=8 {
        h.event(egui::Event::PointerMoved(from.lerp(to, i as f32 / 8.0)));
        h.run();
    }
    h.drop_at(to);
    h.run();
}

#[test]
fn drop_on_empty_canvas_detaches_to_parking() {
    use reasoncraft::canvas::measure::{EguiMeasure, MeasureCache};
    use reasoncraft_core::{CardKind, Document, Item, LayoutParams, Parent, layout};
    let mut doc = Document::new("Claim");
    let root = doc.root;
    let (c, p) = doc
        .add_card(Parent::Box(root), CardKind::Reason, "One")
        .unwrap();
    let mut h = harness_with(doc);
    h.run();
    let (from, to) = {
        let tab = &h.state().tabs[0];
        let mut cache = MeasureCache::default();
        let l = layout(
            &tab.doc,
            &EguiMeasure::new(&h.ctx, &mut cache),
            &LayoutParams::default(),
        );
        let from = tab.camera.world_to_screen(l.boxes[&p].center());
        let view = tab.viewport.unwrap();
        (from, view.left_bottom() + egui::vec2(40.0, -40.0))
    };
    drag(&mut h, from, to);
    let tab = &h.state().tabs[0];
    assert_eq!(tab.doc.parking, vec![Item::Card(c)]);
    assert!(tab.doc.boxes[&root].children.is_empty());
    assert_eq!(tab.history.undo.len(), 1);

    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    let tab = &h.state().tabs[0];
    assert!(tab.doc.parking.is_empty());
    assert_eq!(tab.doc.boxes[&root].children, vec![Item::Card(c)]);
}

/// Queue `events` so they all arrive in the same frame (kittest's own
/// `event` helpers give each event a frame of its own).
fn one_frame(h: &mut Harness<'static, ReasonCraftApp>, events: Vec<egui::Event>) {
    h.input_mut().events.extend(events);
    h.run();
}

fn key(key: egui::Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }
}

/// A typed character and a command key in the same frame: the character
/// belongs to the box being edited, before the command runs.
#[test]
fn text_and_tab_in_one_frame_finish_the_old_box() {
    use egui::{Event, Key};
    let mut h = harness();
    h.run();
    h.key_press(Key::Tab);
    h.run();
    let old = h.state().tabs[0].editing().expect("editing the new reason");
    type_text(&mut h, "abc");
    one_frame(&mut h, vec![Event::Text("x".into()), key(Key::Tab)]);
    let tab = &h.state().tabs[0];
    assert_eq!(tab.doc.boxes[&old].text, "abcx");
    let new = tab.editing().expect("editing the second reason");
    assert_ne!(new, old);
    assert_eq!(tab.doc.boxes[&new].text, "");
    assert_eq!(tab.edit_text, "");
    // Two adds, each carrying its own first text.
    assert_eq!(tab.history.undo.len(), 2);
}

#[test]
fn text_and_escape_in_one_frame_keep_the_character() {
    use egui::{Event, Key};
    let mut h = harness();
    h.run();
    let root = h.state().tabs[0].doc.root;
    type_text(&mut h, "ab");
    assert_eq!(h.state().tabs[0].editing(), Some(root));
    one_frame(&mut h, vec![Event::Text("x".into()), key(Key::Escape)]);
    let tab = &h.state().tabs[0];
    assert_eq!(tab.editing(), None);
    assert_eq!(tab.doc.boxes[&root].text, "abx");
    assert_eq!(tab.history.undo.len(), 1);
    assert_eq!(tab.history.undo[0].boxes[&root].text, "");
}

mod comments;
