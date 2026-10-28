//! UI tests for reordering (drag and Alt+arrows), "Connect to…", and the
//! keys that must not be taken as commands (AltGr, secondary-button release
//! during a building-panel drag).

use crate::common::harness_with_doc;
use eframe::egui::{self, Key, Modifiers};
use egui_kittest::kittest::Queryable;
use reasoncraft::canvas::measure::{EguiMeasure, MeasureCache};
use reasoncraft::i18n::t;
use reasoncraft_core::{
    BoxKind, CardId, CardKind, Document, Item, Layout, LayoutParams, NewLoose, Parent, layout,
};

use crate::common::H;

fn layout_of(h: &H) -> Layout {
    let mut cache = MeasureCache::default();
    layout(
        &h.state().tabs[0].doc,
        &EguiMeasure::new(&h.ctx, &mut cache),
        &LayoutParams::default(),
    )
}

fn screen(h: &H, p: egui::Pos2) -> egui::Pos2 {
    h.state().tabs[0].camera.world_to_screen(p)
}

fn click_at(h: &mut H, pos: egui::Pos2) {
    h.event(egui::Event::PointerMoved(pos));
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
    }
    h.run();
}

/// Press at `from` and move to `to` over several frames (no release).
fn drag_to(h: &mut H, from: egui::Pos2, to: egui::Pos2) {
    h.event(egui::Event::PointerMoved(from));
    h.drag_at(from);
    h.run();
    for i in 1..=8 {
        h.event(egui::Event::PointerMoved(from.lerp(to, i as f32 / 8.0)));
        h.run();
    }
}

/// The root with reason cards "a", "b", "c".
fn three_cards() -> (Document, [CardId; 3]) {
    let mut d = Document::new("Claim");
    let root = d.root;
    let mut card = |t| {
        d.add_card(Parent::Box(root), CardKind::Reason, t)
            .unwrap()
            .0
    };
    let ids = [card("a"), card("b"), card("c")];
    (d, ids)
}

fn root_children(h: &H) -> Vec<Item> {
    let tab = &h.state().tabs[0];
    tab.doc.boxes[&tab.doc.root].children.clone()
}

#[test]
fn drag_between_siblings_reorders() {
    let (d, [a, b, c]) = three_cards();
    let mut h = harness_with_doc(d);
    let l = layout_of(&h);
    let from = screen(&h, l.cards[&c].center());
    let (ra, rb) = (l.cards[&a], l.cards[&b]);
    let gap = screen(
        &h,
        egui::pos2((ra.right() + rb.left()) / 2.0, ra.center().y),
    );
    drag_to(&mut h, from, gap);
    h.drop_at(gap);
    h.run();
    let card = Item::Card;
    assert_eq!(root_children(&h), vec![card(a), card(c), card(b)]);
    assert_eq!(h.state().tabs[0].history.undo.len(), 1);
    assert_eq!(h.state().tabs[0].selection, Some(card(c)));
}

#[test]
fn alt_right_reorders_selected_card() {
    let (d, [a, b, c]) = three_cards();
    let mut h = harness_with_doc(d);
    h.state_mut().tabs[0].selection = Some(Item::Card(a));
    h.key_press_modifiers(Modifiers::ALT, Key::ArrowRight);
    h.run();
    let card = Item::Card;
    assert_eq!(root_children(&h), vec![card(b), card(a), card(c)]);
    assert_eq!(h.state().tabs[0].selection, Some(card(a)));
    // A plain arrow still navigates.
    h.key_press(Key::ArrowRight);
    h.run();
    assert_eq!(h.state().tabs[0].selection, Some(card(c)));
}

/// A parked card and a note under the root: the note refuses cards.
fn parked_card_doc() -> (Document, CardId, reasoncraft_core::NodeId) {
    let mut d = Document::new("Claim");
    let note = d
        .add_box(Parent::Box(d.root), BoxKind::Note, "note")
        .unwrap();
    let Item::Card(parked) = d.add_loose(NewLoose::Card(CardKind::Objection), "parked") else {
        unreachable!()
    };
    (d, parked, note)
}

#[test]
fn connect_mode_click_connects_parking_card() {
    let (d, parked, note) = parked_card_doc();
    let root = d.root;
    let premise = d.cards[&parked].premises[0];
    let mut h = harness_with_doc(d);
    // Selecting the parked card's premise offers its card.
    h.state_mut().tabs[0].selection = Some(Item::Box(premise));
    h.run();
    h.key_press_modifiers(Modifiers::COMMAND | Modifiers::SHIFT, Key::K);
    h.run();
    assert_eq!(h.state().tabs[0].connect_mode, Some(Item::Card(parked)));
    h.get_by_label(&t("connect.hint"));

    // A click on the note (refuses cards) does nothing and keeps the mode.
    let l = layout_of(&h);
    let at_note = screen(&h, l.boxes[&note].center());
    h.event(egui::Event::PointerMoved(at_note));
    h.run();
    assert_eq!(
        h.output().platform_output.cursor_icon,
        egui::CursorIcon::NotAllowed
    );
    click_at(&mut h, at_note);
    let tab = &h.state().tabs[0];
    assert_eq!(tab.connect_mode, Some(Item::Card(parked)));
    assert_eq!(tab.history.undo.len(), 0);
    assert_eq!(
        tab.selection,
        Some(Item::Box(root)),
        "a click does not select"
    );

    // A click on the claim connects the card there and ends the mode.
    let at_root = screen(&h, l.boxes[&root].center());
    click_at(&mut h, at_root);
    let tab = &h.state().tabs[0];
    assert_eq!(tab.connect_mode, None);
    assert!(tab.doc.parking.is_empty());
    assert!(tab.doc.boxes[&root].children.contains(&Item::Card(parked)));
    assert_eq!(tab.history.undo.len(), 1);
    assert_eq!(tab.selection, Some(Item::Card(parked)));
    assert!(h.query_by_label(&t("connect.hint")).is_none());
}

#[test]
fn connect_mode_esc_cancels() {
    let (d, parked, _) = parked_card_doc();
    let mut h = harness_with_doc(d);
    h.state_mut().left_panel = false;
    h.state_mut().tabs[0].selection = Some(Item::Card(parked));
    h.run();
    h.get_by_label(&t("action.connect")).click();
    h.run();
    assert_eq!(h.state().tabs[0].connect_mode, Some(Item::Card(parked)));
    h.key_press(Key::Escape);
    h.run();
    let tab = &h.state().tabs[0];
    assert_eq!(tab.connect_mode, None);
    assert_eq!(tab.doc.parking, vec![Item::Card(parked)]);
    assert_eq!(tab.history.undo.len(), 0);
}

#[test]
fn connect_mode_enter_connects_to_selection() {
    let (d, parked, note) = parked_card_doc();
    let root = d.root;
    let mut h = harness_with_doc(d);
    h.state_mut().tabs[0].selection = Some(Item::Card(parked));
    h.key_press_modifiers(Modifiers::COMMAND | Modifiers::SHIFT, Key::K);
    h.run();
    // Down from the claim reaches the note: Enter there is refused.
    h.key_press(Key::ArrowDown);
    h.run();
    assert_eq!(h.state().tabs[0].selection, Some(Item::Box(note)));
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state().tabs[0].connect_mode, Some(Item::Card(parked)));
    assert_eq!(h.state().tabs[0].editing(), None, "Enter does not edit");
    h.key_press(Key::ArrowUp);
    h.key_press(Key::Enter);
    h.run();
    let tab = &h.state().tabs[0];
    assert_eq!(tab.connect_mode, None);
    assert!(tab.doc.boxes[&root].children.contains(&Item::Card(parked)));
}

/// AltGr (reported as Ctrl+Alt) with a digit types a character: it is not
/// a rating, and on a selected box it starts editing like other typing.
#[test]
fn altgr_digit_types_text_instead_of_rating() {
    let mut h = harness_with_doc(Document::new(""));
    let root = h.state().tabs[0].doc.root;
    h.state_mut().tabs[0].selection = Some(Item::Box(root));
    h.run();
    let altgr = Modifiers::CTRL | Modifiers::ALT;
    for pressed in [true, false] {
        h.event(egui::Event::Key {
            key: Key::Num2,
            physical_key: Some(Key::Num2),
            pressed,
            repeat: false,
            modifiers: altgr,
        });
        if pressed {
            h.event(egui::Event::Text("²".into()));
        }
    }
    h.run();
    let tab = &h.state().tabs[0];
    assert_eq!(tab.editing(), Some(root));
    assert_eq!(tab.doc.boxes[&root].text, "²");
    assert_eq!(tab.doc.boxes[&root].acceptance, None, "no rating");
}

/// Releasing the secondary button during a building-panel drag over the
/// canvas is no drop.
#[test]
fn panel_drop_needs_primary_release() {
    let d = Document::new("Claim");
    let root = d.root;
    let mut h = harness_with_doc(d);
    let to = screen(&h, layout_of(&h).boxes[&root].center());
    let from = h
        .get_all_by_label(&t("action.add_note"))
        .max_by(|a, b| a.rect().top().total_cmp(&b.rect().top()))
        .expect("panel item")
        .rect()
        .center();
    drag_to(&mut h, from, to);
    h.event(egui::Event::PointerButton {
        pos: to,
        button: egui::PointerButton::Secondary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    h.run();
    let tab = &h.state().tabs[0];
    assert!(tab.doc.boxes[&root].children.is_empty());
    assert_eq!(tab.history.undo.len(), 0);
}

/// Inline-editing a parked box, then clicking the ribbon's "Connect to…":
/// the editor losing focus must not end the mode; the edit is committed.
#[test]
fn connect_from_ribbon_while_editing_stays_on() {
    let mut d = Document::new("Claim");
    let note = d.add_loose(NewLoose::Note, "n");
    let Item::Box(nb) = note else { unreachable!() };
    let mut h = harness_with_doc(d);
    h.state_mut().left_panel = false;
    h.state_mut().tabs[0].selection = Some(note);
    h.run();
    h.key_press(Key::F2);
    h.run();
    assert_eq!(h.state().tabs[0].editing(), Some(nb));
    h.event(egui::Event::Text("x".into()));
    h.run();
    h.get_by_label(&t("action.connect")).click();
    h.run();
    h.run();
    let tab = &h.state().tabs[0];
    assert_eq!(tab.connect_mode, Some(note));
    assert_eq!(tab.editing(), None);
    assert_eq!(tab.doc.boxes[&nb].text, "nx");
    assert_eq!(tab.history.undo.len(), 1);
    h.get_by_label(&t("connect.hint"));
}
