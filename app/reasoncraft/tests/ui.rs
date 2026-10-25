//! UI tests for the app shell, driven through the accessibility tree.

use eframe::egui;
use egui_kittest::{
    Harness,
    kittest::{NodeT, Queryable},
};
use reasoncraft::ReasonCraftApp;
use reasoncraft::i18n::t;

fn harness() -> Harness<'static, ReasonCraftApp> {
    Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .with_max_steps(32)
        .build_ui_state(
            |ui, app: &mut ReasonCraftApp| app.show(ui),
            ReasonCraftApp::new_for_test(),
        )
}

#[test]
fn ribbon_add_reason_adds_card() {
    let mut h = harness();
    h.run();
    assert_eq!(h.state().tabs[0].doc.cards.len(), 0);
    let label = t("action.add_reason");
    assert_eq!(label, "Reason");
    h.get_by_label(&label).click();
    h.run();
    let tab = &h.state().tabs[0];
    assert_eq!(tab.doc.cards.len(), 1);
    assert_eq!(tab.history.undo.len(), 1);
}

#[test]
fn theme_switch_applies_dark_visuals() {
    let mut h = harness();
    h.run();
    // Start from light so the switch is observable.
    let view = t("ribbon.tab.view");
    h.get_by_label(&view).click();
    h.run();
    h.get_by_label(&t("theme.light")).click();
    h.run();
    assert!(!h.ctx.global_style().visuals.dark_mode);
    h.get_by_label(&t("theme.dark")).click();
    h.run();
    assert!(h.ctx.global_style().visuals.dark_mode);
    assert_eq!(h.state().theme, reasoncraft::theme::ThemeChoice::Dark);
}

#[test]
fn new_tab_button_opens_second_document() {
    let mut h = harness();
    h.run();
    assert_eq!(h.state().tabs.len(), 1);
    h.get_by_label(&t("tabs.new")).click();
    h.run();
    assert_eq!(h.state().tabs.len(), 2);
    assert_eq!(h.state().active, 1);
}

#[test]
fn missing_i18n_key_returns_key() {
    assert_eq!(t("no.such.key"), "no.such.key");
    assert_eq!(t("untitled"), "Untitled");
    assert_eq!(t("ribbon.coming_soon"), "Coming soon");
}

fn harness_with(doc: reasoncraft_core::Document) -> Harness<'static, ReasonCraftApp> {
    let mut app = ReasonCraftApp::new_for_test();
    app.tabs[0] = reasoncraft::doc_tab::DocTab::from_document(doc);
    Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .with_max_steps(32)
        .build_ui_state(|ui, app: &mut ReasonCraftApp| app.show(ui), app)
}

#[test]
fn canvas_renders_117_box_fixture_without_panic() {
    let doc = reasoncraft_core::fixtures::sample_iot_like(117);
    assert!(doc.boxes.len() >= 117);
    let mut h = harness_with(doc);
    h.run_steps(3);
    let tab = &h.state().tabs[0];
    let viewport = tab.viewport.expect("canvas reports its viewport");
    assert!(viewport.width() > 400.0 && viewport.height() > 300.0);
    // A new document is fitted on its first frame; this map is too wide for 100%.
    assert!(!tab.fit_pending);
    assert!(tab.camera.zoom < 1.0);
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

#[test]
fn canvas_click_selects_box_and_empty_canvas_clears() {
    use reasoncraft::canvas::measure::{EguiMeasure, MeasureCache};
    use reasoncraft_core::{Item, LayoutParams, layout};

    let doc = reasoncraft_core::fixtures::sample_iot_like(40);
    let mut h = harness_with(doc);
    h.run();
    let (target, screen, corner) = {
        let tab = &h.state().tabs[0];
        let mut cache = MeasureCache::default();
        let l = layout(
            &tab.doc,
            &EguiMeasure::new(&h.ctx, &mut cache),
            &LayoutParams::default(),
        );
        let (&id, rect) = l.boxes.iter().nth(5).unwrap();
        let screen = tab.camera.world_to_screen(rect.center());
        (id, screen, tab.viewport.unwrap().min + egui::vec2(4.0, 4.0))
    };
    click_at(&mut h, screen);
    assert_eq!(h.state().tabs[0].selection, Some(Item::Box(target)));
    click_at(&mut h, corner);
    assert_eq!(h.state().tabs[0].selection, None);
}

// ---- Task 10: editing interactions ----

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
        h.state().tabs[0].editing.is_some(),
        "a new reason is edited"
    );
    type_text(&mut h, "Reason one");
    h.key_press(Key::Escape);
    h.run();
    assert_eq!(h.state().tabs[0].editing, None);
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
    assert_eq!(h.state().tabs[0].editing, Some(root));
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
    assert_eq!(tab.editing, Some(premise));
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
    let old = h.state().tabs[0].editing.expect("editing the new reason");
    type_text(&mut h, "abc");
    one_frame(&mut h, vec![Event::Text("x".into()), key(Key::Tab)]);
    let tab = &h.state().tabs[0];
    assert_eq!(tab.doc.boxes[&old].text, "abcx");
    let new = tab.editing.expect("editing the second reason");
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
    assert_eq!(h.state().tabs[0].editing, Some(root));
    one_frame(&mut h, vec![Event::Text("x".into()), key(Key::Escape)]);
    let tab = &h.state().tabs[0];
    assert_eq!(tab.editing, None);
    assert_eq!(tab.doc.boxes[&root].text, "abx");
    assert_eq!(tab.history.undo.len(), 1);
    assert_eq!(tab.history.undo[0].boxes[&root].text, "");
}
