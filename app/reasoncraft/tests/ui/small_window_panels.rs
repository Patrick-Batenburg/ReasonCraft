//! The panels in the smallest window: a squeezed panel gets its width
//! back, an edited box stays in view, and a dragged width survives a
//! restart.

use eframe::egui;
use egui_kittest::{Harness, kittest::Queryable};
use reasoncraft::ReasonCraftApp;
use reasoncraft::i18n::t;

use crate::ui_small_window::harness;

/// A squeezed panel gets its width back when the window has room again.
#[test]
fn squeezed_panels_get_their_width_back() {
    let mut h = harness(1.5);
    let text_left =
        |h: &Harness<'static, ReasonCraftApp>| h.get_by_label(&t("text.tab.outline")).rect().left();
    let right = |h: &Harness<'static, ReasonCraftApp>| h.ctx.content_rect().right();
    let squeezed = right(&h) - text_left(&h);
    h.state_mut().settings.ui_scale = 1.0;
    for _ in 0..3 {
        h.run();
    }
    let back = right(&h) - text_left(&h);
    assert!(
        back > squeezed + 100.0,
        "the Text panel is {back} wide again (was {squeezed})"
    );
}

/// Shrinking the window while a box is being edited keeps that box in
/// view (and in edit mode).
#[test]
fn edited_box_stays_in_view_when_the_window_shrinks() {
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_ui_state(
            |ui, app: &mut ReasonCraftApp| app.show(ui),
            ReasonCraftApp::new_for_test(),
        );
    h.run();
    let root = h.state().tabs[0].doc.root;
    h.state_mut().tabs[0].begin_edit(root, false);
    h.run();
    // Put the box near the right edge of the large view.
    let view = h.state().tabs[0].viewport.expect("map shown");
    // The box's text field, where the box is drawn.
    let shown = |h: &Harness<'static, ReasonCraftApp>| {
        h.get_by_role(egui::accesskit::Role::MultilineTextInput)
            .rect()
    };
    let now = shown(&h);
    h.state_mut().tabs[0].camera.offset.x += view.right() - 30.0 - now.right();
    h.run();
    h.set_size(egui::vec2(800.0, 500.0));
    for _ in 0..3 {
        h.run();
    }
    let view = h.state().tabs[0].viewport.expect("map shown");
    let rect = shown(&h);
    assert!(view.contains_rect(rect), "{rect:?} outside {view:?}");
    assert_eq!(h.state().tabs[0].editing(), Some(root));
}

/// eframe storage held in memory.
#[derive(Default)]
struct MemStorage(std::collections::HashMap<String, String>);

impl eframe::Storage for MemStorage {
    fn get_string(&self, key: &str) -> Option<String> {
        self.0.get(key).cloned()
    }
    fn set_string(&mut self, key: &str, value: String) {
        self.0.insert(key.to_owned(), value);
    }
    fn remove_string(&mut self, key: &str) {
        self.0.remove(key);
    }
    fn flush(&mut self) {}
}

/// Drag the Text panel's edge from its left side to `x`.
fn drag_text_panel_to(h: &mut Harness<'static, ReasonCraftApp>, x: f32) {
    use reasoncraft::shell::panes::Pane;
    let rect = h.state().pane_rect(Pane::Text).expect("Text panel shown");
    let from = egui::pos2(rect.left() + 2.0, rect.center().y);
    h.event(egui::Event::PointerMoved(from));
    h.run();
    h.drag_at(from);
    h.run();
    for at in [egui::pos2(from.x - 20.0, from.y), egui::pos2(x, from.y)] {
        h.event(egui::Event::PointerMoved(at));
        h.run();
    }
    h.event(egui::Event::PointerButton {
        pos: egui::pos2(x, from.y),
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    h.run();
}

/// A width the user dragged survives a restart in a small window: egui's
/// memory (as eframe saves it) brings it back once there is room.
#[test]
fn a_dragged_panel_width_survives_a_restart_in_a_small_window() {
    use reasoncraft::shell::panes::Pane;
    let build = |size: egui::Vec2, scale: f32| {
        let mut app = ReasonCraftApp::new_for_test();
        app.settings.ui_scale = scale;
        Harness::builder()
            .with_size(size)
            .build_ui_state(|ui, app: &mut ReasonCraftApp| app.show(ui), app)
    };
    let big = egui::vec2(1280.0, 800.0);
    let mut h = build(big, 1.0);
    h.run();
    drag_text_panel_to(&mut h, 1280.0 - 420.0);
    let width =
        |h: &Harness<'static, ReasonCraftApp>| h.state().pane_rect(Pane::Text).unwrap().width();
    let dragged = width(&h);
    assert!(dragged > 380.0, "dragged to {dragged}");
    // Closed and started again in a small window, egui's memory saved
    // and loaded the way eframe does it (temporary data is dropped).
    let mut storage = MemStorage::default();
    eframe::set_value(&mut storage, "egui", &h.ctx.memory(|m| m.clone()));
    let mut h = build(egui::vec2(800.0, 500.0), 1.5);
    let memory: egui::Memory = eframe::get_value(&storage, "egui").unwrap();
    h.ctx.memory_mut(|m| *m = memory);
    h.run();
    h.run();
    eframe::set_value(&mut storage, "egui", &h.ctx.memory(|m| m.clone()));
    // And again in the large window.
    let mut h = build(big, 1.0);
    let memory: egui::Memory = eframe::get_value(&storage, "egui").unwrap();
    h.ctx.memory_mut(|m| *m = memory);
    h.run();
    h.run();
    assert!(
        (width(&h) - dragged).abs() < 1.0,
        "back to {}, not {dragged}",
        width(&h)
    );
}
