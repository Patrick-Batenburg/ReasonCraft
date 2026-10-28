//! UI tests for keyboard navigation between boxes and cards.

use crate::common::{app_with_doc, build};
use eframe::egui;
use egui_kittest::Harness;
use reasoncraft::ReasonCraftApp;

fn harness() -> Harness<'static, ReasonCraftApp> {
    build(ReasonCraftApp::new_for_test())
}

fn type_text(h: &mut Harness<'static, ReasonCraftApp>, text: &str) {
    h.event(egui::Event::Text(text.to_owned()));
    h.run();
}

fn harness_with(doc: reasoncraft_core::Document) -> Harness<'static, ReasonCraftApp> {
    build(app_with_doc(doc))
}

/// Up from a premise reaches its card; Ctrl+Shift+O and Ctrl+Shift+R add
/// whole-card attacks from a premise or from the card itself.
#[test]
fn keyboard_reaches_cards_and_whole_card_targets() {
    use egui::{Key, Modifiers};
    let cmd_shift = Modifiers::COMMAND | Modifiers::SHIFT;
    let mut h = harness();
    h.run();
    h.key_press(Key::Tab);
    h.run();
    type_text(&mut h, "Reason");
    h.key_press(Key::Escape);
    h.run();
    let card = *h.state().tabs[0].doc.cards.keys().next().unwrap();
    h.key_press(Key::ArrowUp);
    h.run();
    assert_eq!(
        h.state().tabs[0].selection,
        Some(reasoncraft_core::Item::Card(card))
    );
    h.key_press_modifiers(cmd_shift, Key::O);
    h.run();
    assert_eq!(h.state().tabs[0].doc.cards[&card].children.len(), 1);
    // While editing the new attacker's premise: finish and attack its card.
    type_text(&mut h, "Does not follow");
    h.key_press_modifiers(cmd_shift, Key::R);
    h.run();
    let tab = &h.state().tabs[0];
    let attacker = tab.doc.cards[&card].children[0];
    assert_eq!(tab.doc.cards[&attacker].children.len(), 1);
    let kind = &tab.doc.cards[&tab.doc.cards[&attacker].children[0]].kind;
    assert_eq!(*kind, reasoncraft_core::CardKind::Reason);
}

/// After Ctrl+Shift+P at 100% the first parked item, right of a map far
/// wider than the view, is panned into view.
#[test]
fn view_follows_keyboard_selection() {
    use egui::{Key, Modifiers};
    use reasoncraft::canvas::measure::EguiMeasure;
    let doc = reasoncraft_core::fixtures::sample_iot_like(117);
    assert!(!doc.parking.is_empty(), "the sample has parked items");
    let mut h = harness_with(doc);
    h.run_steps(3);
    h.key_press_modifiers(Modifiers::COMMAND, Key::Num0);
    h.run();
    h.key_press_modifiers(Modifiers::COMMAND | Modifiers::SHIFT, Key::P);
    h.run();
    let ctx = h.ctx.clone();
    let app = h.state_mut();
    let tab = &app.tabs[0];
    assert_eq!(tab.camera.zoom, 1.0);
    let selected = tab.selection.expect("a selection");
    assert_eq!(Some(&selected), tab.doc.parking.first());
    let l = reasoncraft_core::layout(
        &tab.doc,
        &EguiMeasure::new(&ctx, &mut app.measure_cache),
        &reasoncraft_core::LayoutParams::default(),
    );
    let rect = match selected {
        reasoncraft_core::Item::Box(b) => l.boxes[&b],
        reasoncraft_core::Item::Card(c) => l.cards[&c],
    };
    let on_screen = tab.camera.world_rect_to_screen(rect);
    let viewport = tab.viewport.unwrap();
    assert!(
        viewport.shrink(23.0).contains_rect(on_screen),
        "{on_screen:?} not inside {viewport:?}"
    );
}
