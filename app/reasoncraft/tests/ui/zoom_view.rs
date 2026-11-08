//! The view follows the keyboard: zooming keeps the selection in view,
//! and editing at a tiny zoom zooms in to a readable size first.

use crate::common::{H, app_with_doc, build};
use eframe::egui::{Key, Modifiers, Rect};
use reasoncraft::canvas::measure::EguiMeasure;
use reasoncraft_core::{Item, NodeId};

/// The 117-box sample map, fitted to the view.
fn sample() -> H {
    let doc = reasoncraft_core::fixtures::sample_iot_like(117);
    let mut h = build(app_with_doc(doc));
    h.run_steps(3);
    h.run();
    h
}

/// The screen rect of `item` now, and the map's view.
fn on_screen(h: &mut H, item: Item) -> (Rect, Rect) {
    let ctx = h.ctx.clone();
    let app = h.state_mut();
    let params = app.settings.layout_params();
    let tab = &app.tabs[0];
    let l = reasoncraft_core::layout(
        &tab.doc,
        &EguiMeasure::new(&ctx, &mut app.measure_cache),
        &params,
    );
    let world = match item {
        Item::Box(b) => l.boxes[&b],
        Item::Card(c) => l.cards[&c],
    };
    (
        tab.camera.world_rect_to_screen(world),
        tab.viewport.unwrap(),
    )
}

/// A box far from the middle of the map: the first parked item's.
fn far_box(h: &H) -> NodeId {
    let tab = &h.state().tabs[0];
    match tab.doc.parking[0] {
        Item::Box(b) => b,
        Item::Card(c) => tab.doc.cards[&c].premises[0],
    }
}

/// Ctrl+= and Ctrl+0 zoom about the middle of the view, then pan the
/// selected box back into view if it left it.
#[test]
fn keyboard_zoom_keeps_the_selection_in_view() {
    let mut h = sample();
    let b = far_box(&h);
    h.state_mut()
        .push(reasoncraft::actions::Action::Select(Some(Item::Box(b))));
    h.run();
    let (rect, view) = on_screen(&mut h, Item::Box(b));
    assert!(view.intersects(rect), "fitted: the box shows");
    for _ in 0..6 {
        h.key_press_modifiers(Modifiers::COMMAND, Key::Equals);
        h.run();
        let (rect, view) = on_screen(&mut h, Item::Box(b));
        assert!(view.contains_rect(rect), "Ctrl+=: {rect:?} not in {view:?}");
    }
    h.key_press_modifiers(Modifiers::COMMAND, Key::Minus);
    h.run();
    h.key_press_modifiers(Modifiers::COMMAND, Key::Num0);
    h.run();
    assert_eq!(h.state().tabs[0].camera.zoom, 1.0);
    let (rect, view) = on_screen(&mut h, Item::Box(b));
    assert!(view.contains_rect(rect), "Ctrl+0: {rect:?} not in {view:?}");
}

/// Enter on a box whose text shows smaller than 9 px zooms to 100 %,
/// centred on the box; at a readable zoom Enter leaves the view alone.
#[test]
fn editing_at_a_tiny_zoom_zooms_in() {
    let mut h = sample();
    let b = far_box(&h);
    h.state_mut().tabs[0].camera.zoom = 0.2;
    h.state_mut()
        .push(reasoncraft::actions::Action::Select(Some(Item::Box(b))));
    h.run();
    h.key_press(Key::Enter);
    h.run();
    let tab = &h.state().tabs[0];
    assert_eq!(tab.editing(), Some(b));
    assert_eq!(tab.camera.zoom, 1.0);
    let (rect, view) = on_screen(&mut h, Item::Box(b));
    assert!(
        (rect.center() - view.center()).length() < 1.0,
        "{rect:?} not centred in {view:?}"
    );
    // Readable already (13 px × 0.8): nothing moves.
    h.key_press(Key::Escape);
    h.run();
    h.state_mut().tabs[0].camera.zoom = 0.8;
    h.run();
    let before = h.state().tabs[0].camera;
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state().tabs[0].editing(), Some(b));
    let after = h.state().tabs[0].camera;
    assert_eq!((after.zoom, after.offset), (before.zoom, before.offset));
}
