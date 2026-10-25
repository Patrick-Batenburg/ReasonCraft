//! UI tests for editing the map: keyboard building, drags and comments.

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

fn harness_with(doc: reasoncraft_core::Document) -> Harness<'static, ReasonCraftApp> {
    let mut app = ReasonCraftApp::new_for_test();
    app.tabs[0] = reasoncraft::doc_tab::DocTab::from_document(doc);
    Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .with_max_steps(32)
        .build_ui_state(|ui, app: &mut ReasonCraftApp| app.show(ui), app)
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

// ---- Step 2, Task 7: comments ----

/// A claim, selected, with its comment popover opened from the Home ribbon.
fn comments_harness(doc: reasoncraft_core::Document) -> Harness<'static, ReasonCraftApp> {
    let root = doc.root;
    let mut h = harness_with(doc);
    // The building panel has items with the same names; hide it.
    h.state_mut().left_panel = false;
    h.state_mut().tabs[0].selection = Some(reasoncraft_core::Item::Box(root));
    h.run();
    h.get_by_label(&t("action.comment")).click();
    h.run();
    assert_eq!(h.state().tabs[0].comments_open, Some(root));
    h
}

#[test]
fn preset_adds_comment_and_badge_shows_count() {
    use reasoncraft_core::{Document, Verdict};
    let mut h = comments_harness(Document::new("Claim"));
    let root = h.state().tabs[0].doc.root;
    let add_preset = |h: &mut Harness<'static, ReasonCraftApp>, key: &str| {
        h.get_by_label(&t("comment.presets")).click();
        h.run();
        h.get_by_label(&t(key)).click();
        h.run();
    };
    add_preset(&mut h, "comment.preset.great_work");
    {
        let tab = &h.state().tabs[0];
        let comments = &tab.doc.boxes[&root].comments;
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].text, t("comment.preset.great_work"));
        assert_eq!(comments[0].verdict, Some(Verdict::Positive));
        assert_eq!(comments[0].author, t("comment.anonymous"));
        assert_eq!(tab.history.undo.len(), 1);
        // Picking from the menu keeps the popover open.
        assert_eq!(tab.comments_open, Some(root));
    }
    assert!(h.query_by_label("1 comment").is_some());
    add_preset(&mut h, "comment.preset.unclear");
    assert_eq!(
        h.state().tabs[0].doc.boxes[&root].comments[1].verdict,
        Some(Verdict::Problem)
    );
    assert!(h.query_by_label("2 comments").is_some());

    // Esc closes the popover; the badge opens it again.
    h.key_press(egui::Key::Escape);
    h.run();
    assert_eq!(h.state().tabs[0].comments_open, None);
    h.state_mut().tabs[0].selection = None;
    h.run();
    h.get_by_label("2 comments").click();
    h.run();
    let tab = &h.state().tabs[0];
    assert_eq!(tab.comments_open, Some(root));
    assert_eq!(tab.selection, Some(reasoncraft_core::Item::Box(root)));

    // A press on the empty canvas closes it.
    let corner = tab.viewport.unwrap().min + egui::vec2(4.0, 4.0);
    click_at(&mut h, corner);
    assert_eq!(h.state().tabs[0].comments_open, None);
}

#[test]
fn long_comment_wraps_in_popover() {
    use reasoncraft::canvas::comments::{POPOVER_WIDTH, popover_id};
    use reasoncraft_core::Document;
    let mut doc = Document::new("Claim");
    let root = doc.root;
    let long: String = "Every claim deserves a reason and a source. "
        .repeat(14)
        .chars()
        .take(600)
        .collect();
    assert_eq!(long.chars().count(), 600);
    doc.add_comment(root, "Ann", &long, None).unwrap();
    doc.add_comment(root, "Bob", &"x".repeat(600), None)
        .unwrap();
    let mut h = comments_harness(doc);
    h.run();
    let area = egui::AreaState::load(&h.ctx, popover_id())
        .expect("popover shown")
        .rect();
    assert!(
        (area.width() - POPOVER_WIDTH).abs() < 1.0,
        "popover width {}",
        area.width()
    );
    for text in [long.as_str(), &"x".repeat(600)] {
        let label = h.get_by_label(text).rect();
        assert!(label.left() >= area.left() && label.right() <= area.right());
        assert!(label.width() <= POPOVER_WIDTH - 20.0, "{}", label.width());
    }
    let first = h.get_by_label(long.as_str()).rect();
    assert!(first.height() > 60.0, "wrapped over many rows: {first:?}");
}

#[test]
fn typed_comment_takes_the_picked_verdict_and_blank_is_refused() {
    use reasoncraft_core::{Document, Verdict};
    let mut h = comments_harness(Document::new("Claim"));
    let root = h.state().tabs[0].doc.root;
    let add = t("comment.add");
    let disabled =
        |h: &Harness<'static, ReasonCraftApp>| h.get_by_label(&add).accesskit_node().is_disabled();
    assert!(disabled(&h));
    let input = || egui::accesskit::Role::MultilineTextInput;
    h.get_by_role(input()).focus();
    h.run();
    h.get_by_role(input()).type_text("   ");
    h.run();
    assert!(disabled(&h), "blank text cannot be added");
    h.get_by_label(&t("comment.verdict.needs_work")).click();
    h.run();
    h.get_by_role(input()).focus();
    h.run();
    h.get_by_role(input()).type_text("Say who found this.");
    h.run();
    assert!(!disabled(&h));
    h.get_by_label(&add).click();
    h.run();
    let tab = &h.state().tabs[0];
    let c = &tab.doc.boxes[&root].comments;
    assert_eq!(c.len(), 1);
    assert_eq!(c[0].text, "Say who found this.");
    assert_eq!(c[0].verdict, Some(Verdict::NeedsWork));
    assert_eq!(tab.history.undo.len(), 1);
    // The input is cleared; delete removes the comment again.
    assert!(disabled(&h));
    h.get_by_label(&t("comment.delete")).click();
    h.run();
    let tab = &h.state().tabs[0];
    assert!(tab.doc.boxes[&root].comments.is_empty());
    assert_eq!(tab.history.undo.len(), 2);
}
