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
    // The building panel has items with the same names; hide it.
    h.state_mut().left_panel = false;
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
    assert_eq!(
        h.state().settings.theme,
        reasoncraft::theme::ThemeChoice::Dark
    );
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

// ---- Task 11: file operations ----

use reasoncraft::file_ops::FakeDialogs;

fn file_harness() -> (Harness<'static, ReasonCraftApp>, FakeDialogs) {
    let fake = FakeDialogs::default();
    let app = ReasonCraftApp::new_for_test_with(Box::new(fake.clone()));
    let h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .with_max_steps(32)
        .build_ui_state(|ui, app: &mut ReasonCraftApp| app.show(ui), app);
    (h, fake)
}

fn cmd(h: &mut Harness<'static, ReasonCraftApp>, key: egui::Key) {
    h.key_press_modifiers(egui::Modifiers::COMMAND, key);
    h.run();
}

/// Type `text` on the selected root claim and finish the edit.
fn type_claim(h: &mut Harness<'static, ReasonCraftApp>, text: &str) {
    type_text(h, text);
    h.key_press(egui::Key::Escape);
    h.run();
}

fn close_label(h: &Harness<'static, ReasonCraftApp>, i: usize) -> String {
    let mut args = reasoncraft::i18n::FluentArgs::new();
    args.set("name", h.state().tabs[i].title());
    reasoncraft::i18n::t_args("tabs.close", &args)
}

#[test]
fn save_then_open_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, fake) = file_harness();
    h.run();
    type_claim(&mut h, "Robots help");
    assert!(h.state().active_tab().is_dirty());

    // No extension given: Save forces `.rcraft`.
    fake.set_save(dir.path().join("robots"));
    cmd(&mut h, egui::Key::S);
    let saved = dir.path().join("robots.rcraft");
    assert!(saved.is_file());
    let tab = h.state().active_tab();
    assert_eq!(tab.path.as_deref(), Some(saved.as_path()));
    assert!(!tab.is_dirty());
    assert_eq!(tab.title(), "robots");

    // A clean tab closes without asking; the last tab is replaced by a new one.
    let label = close_label(&h, 0);
    h.get_by_label(&label).click();
    h.run();
    assert!(h.state().modal.is_none());
    assert_eq!(h.state().tabs.len(), 1);
    assert_eq!(h.state().tabs[0].path, None);

    // File → Open reuses the untouched new tab.
    fake.set_open(saved.clone());
    h.get_by_label(&t("ribbon.tab.file")).click();
    h.run();
    h.get_by_label(&t("action.open")).click();
    h.run();
    let app = h.state();
    assert_eq!(app.tabs.len(), 1);
    let tab = app.active_tab();
    assert_eq!(tab.doc.boxes[&tab.doc.root].text, "Robots help");
    assert_eq!(tab.path.as_deref(), Some(saved.as_path()));
    assert!(!tab.is_dirty());

    // Opening the same file again just activates its tab.
    h.get_by_label(&t("tabs.new")).click();
    h.run();
    type_claim(&mut h, "Other");
    assert_eq!(h.state().active, 1);
    fake.set_open(saved);
    cmd(&mut h, egui::Key::O);
    assert_eq!(h.state().tabs.len(), 2);
    assert_eq!(h.state().active, 0);
}

#[test]
fn failed_open_keeps_tabs() {
    let dir = tempfile::tempdir().unwrap();
    let garbage = dir.path().join("garbage.rcraft");
    std::fs::write(&garbage, b"this is not a zip file").unwrap();
    let (mut h, fake) = file_harness();
    h.run();
    type_claim(&mut h, "Unsaved work");
    h.get_by_label(&t("tabs.new")).click();
    h.run();
    assert_eq!(h.state().tabs.len(), 2);

    fake.set_open(garbage);
    cmd(&mut h, egui::Key::O);
    let app = h.state();
    assert_eq!(app.tabs.len(), 2);
    assert_eq!(app.active, 1);
    let first = &app.tabs[0];
    assert_eq!(first.doc.boxes[&first.doc.root].text, "Unsaved work");
    assert!(app.modal.is_some());
    h.get_by_label(&t("error.open.not_a_file"));
    h.get_by_label_contains("garbage.rcraft");

    h.get_by_label(&t("modal.ok")).click();
    h.run();
    assert!(h.state().modal.is_none());
    assert_eq!(h.state().tabs.len(), 2);
}

#[test]
fn undo_after_save_marks_dirty() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, fake) = file_harness();
    h.run();
    type_claim(&mut h, "Claim");
    fake.set_save(dir.path().join("map.rcraft"));
    cmd(&mut h, egui::Key::S);
    assert!(!h.state().active_tab().is_dirty());
    assert_eq!(h.state().active_tab().title(), "map");

    cmd(&mut h, egui::Key::Z);
    assert!(h.state().active_tab().is_dirty());
    assert_eq!(h.state().active_tab().title(), "map •");
    cmd(&mut h, egui::Key::Y);
    assert!(!h.state().active_tab().is_dirty());
}

#[test]
fn save_while_editing_commits_the_text() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, fake) = file_harness();
    h.run();
    type_text(&mut h, "Half typed");
    assert!(h.state().active_tab().editing.is_some());
    assert!(h.state().active_tab().is_dirty(), "a pending edit is dirty");
    let path = dir.path().join("typed.rcraft");
    fake.set_save(path.clone());
    cmd(&mut h, egui::Key::S);
    let tab = h.state().active_tab();
    assert_eq!(tab.editing, None);
    assert!(!tab.is_dirty());
    let doc = reasoncraft_core::file::open(&path).unwrap();
    assert_eq!(doc.boxes[&doc.root].text, "Half typed");
}

#[test]
fn close_dirty_tab_asks_to_save() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, fake) = file_harness();
    h.run();
    // Still editing: the pending text counts as an unsaved change.
    type_text(&mut h, "Draft");
    let label = close_label(&h, 0);
    h.get_by_label(&label).click();
    h.run();
    assert!(h.state().modal.is_some());
    h.get_by_label(&t("unsaved.message"));

    // Cancel keeps the tab and its work.
    h.get_by_label(&t("unsaved.cancel")).click();
    h.run();
    assert!(h.state().modal.is_none());
    let tab = h.state().active_tab();
    assert_eq!(tab.doc.boxes[&tab.doc.root].text, "Draft");
    assert!(tab.is_dirty());

    // Save writes the file, then closes the tab.
    let label = close_label(&h, 0);
    h.get_by_label(&label).click();
    h.run();
    let path = dir.path().join("draft.rcraft");
    fake.set_save(path.clone());
    h.get_by_label(&t("unsaved.save_changes")).click();
    h.run();
    assert!(h.state().modal.is_none());
    assert!(path.is_file());
    assert_eq!(h.state().tabs.len(), 1);
    assert_eq!(h.state().tabs[0].path, None);

    // Don't save discards the changes.
    type_claim(&mut h, "Throwaway");
    let label = close_label(&h, 0);
    h.get_by_label(&label).click();
    h.run();
    h.get_by_label(&t("unsaved.discard")).click();
    h.run();
    let tab = h.state().active_tab();
    assert_eq!(tab.doc.boxes[&tab.doc.root].text, "");
    assert!(!tab.is_dirty());
}

#[test]
fn window_close_asks_per_dirty_tab() {
    let (mut h, _fake) = file_harness();
    h.run();
    type_claim(&mut h, "One");
    h.get_by_label(&t("tabs.new")).click();
    h.run();
    h.get_by_label(&t("tabs.new")).click();
    h.run();
    type_claim(&mut h, "Three");

    // The OS asks to close: ask about tab 0 first, then tab 2 (tab 1 is clean).
    h.input_mut()
        .viewports
        .get_mut(&egui::ViewportId::ROOT)
        .unwrap()
        .events
        .push(egui::ViewportEvent::Close);
    h.run();
    assert!(h.state().modal.is_some());
    assert!(!h.state().close_confirmed());
    assert_eq!(h.state().active, 0, "the tab in question is shown");
    h.get_by_label(&t("unsaved.discard")).click();
    h.run();
    assert_eq!(h.state().active, 2);
    // Cancel aborts the whole close.
    h.get_by_label(&t("unsaved.cancel")).click();
    h.run();
    assert!(h.state().modal.is_none());
    assert!(!h.state().close_confirmed());
    assert_eq!(h.state().tabs.len(), 3);

    // The title bar's close button goes the same way.
    h.state_mut()
        .push(reasoncraft::actions::Action::CloseWindow);
    h.run();
    h.get_by_label(&t("unsaved.discard")).click();
    h.run();
    h.get_by_label(&t("unsaved.discard")).click();
    h.run();
    assert!(h.state().modal.is_none());
    assert!(h.state().close_confirmed());
}

#[test]
fn window_title_uses_i18n_separator() {
    let mut args = reasoncraft::i18n::FluentArgs::new();
    args.set("doc", "robots");
    args.set("app", t("app.name"));
    assert_eq!(
        reasoncraft::i18n::t_args("window.title", &args),
        "robots — ReasonCraft"
    );
}

fn replace_title(name: &str) -> String {
    let mut args = reasoncraft::i18n::FluentArgs::new();
    args.set("file", name);
    reasoncraft::i18n::t_args("replace.title", &args)
}

fn root_text_in(path: &std::path::Path) -> String {
    let doc = reasoncraft_core::file::open(path).unwrap();
    doc.boxes[&doc.root].text.clone()
}

/// The dialog checked `robots` for overwriting, not `robots.rcraft`: the app
/// asks before replacing the file the appended extension points at.
#[test]
fn appended_extension_asks_before_replacing() {
    let dir = tempfile::tempdir().unwrap();
    let existing = dir.path().join("robots.rcraft");
    reasoncraft_core::file::save(&reasoncraft_core::Document::new("Old"), &existing).unwrap();
    let before = std::fs::read(&existing).unwrap();
    let (mut h, fake) = file_harness();
    h.run();
    type_claim(&mut h, "New");

    fake.set_save(dir.path().join("robots"));
    cmd(&mut h, egui::Key::S);
    assert!(h.state().modal.is_some());
    h.get_by_label(&replace_title("robots.rcraft"));
    h.get_by_label(&t("replace.cancel")).click();
    h.run();
    assert!(h.state().modal.is_none());
    assert_eq!(std::fs::read(&existing).unwrap(), before);
    let tab = h.state().active_tab();
    assert!(tab.is_dirty());
    assert_eq!(tab.path, None);

    fake.set_save(dir.path().join("robots"));
    cmd(&mut h, egui::Key::S);
    h.get_by_label(&t("replace.confirm")).click();
    h.run();
    assert!(h.state().modal.is_none());
    assert_eq!(root_text_in(&existing), "New");
    let tab = h.state().active_tab();
    assert!(!tab.is_dirty());
    assert_eq!(tab.path.as_deref(), Some(existing.as_path()));
}

/// A name that already ends in `.rcraft` was checked by the dialog itself.
#[test]
fn chosen_rcraft_name_saves_without_extra_confirm() {
    let dir = tempfile::tempdir().unwrap();
    let existing = dir.path().join("robots.rcraft");
    reasoncraft_core::file::save(&reasoncraft_core::Document::new("Old"), &existing).unwrap();
    let (mut h, fake) = file_harness();
    h.run();
    type_claim(&mut h, "New");
    fake.set_save(existing.clone());
    cmd(&mut h, egui::Key::S);
    assert!(h.state().modal.is_none());
    assert_eq!(root_text_in(&existing), "New");
    assert!(!h.state().active_tab().is_dirty());
}

fn two_dirty_tabs() -> (Harness<'static, ReasonCraftApp>, FakeDialogs) {
    let (mut h, fake) = file_harness();
    h.run();
    type_claim(&mut h, "One");
    h.get_by_label(&t("tabs.new")).click();
    h.run();
    type_claim(&mut h, "Two");
    (h, fake)
}

#[test]
fn failed_save_during_window_close_aborts_it() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, fake) = two_dirty_tabs();
    h.state_mut()
        .push(reasoncraft::actions::Action::CloseWindow);
    h.run();
    fake.set_save(dir.path().join("no/such/dir/one.rcraft"));
    h.get_by_label(&t("unsaved.save_changes")).click();
    h.run();
    assert!(matches!(
        h.state().modal,
        Some(reasoncraft::file_ops::Modal::Error { .. })
    ));
    assert!(!h.state().close_confirmed());
    h.get_by_label(&t("modal.ok")).click();
    h.run();
    let app = h.state();
    assert!(app.modal.is_none());
    assert!(!app.close_confirmed());
    assert_eq!(app.tabs.len(), 2);
    assert!(app.tabs.iter().all(|tab| tab.is_dirty()));
}

#[test]
fn replace_prompt_during_window_close() {
    let dir = tempfile::tempdir().unwrap();
    let existing = dir.path().join("one.rcraft");
    reasoncraft_core::file::save(&reasoncraft_core::Document::new("Old"), &existing).unwrap();
    let (mut h, fake) = two_dirty_tabs();

    // Cancel at the replace prompt aborts the whole close.
    h.state_mut()
        .push(reasoncraft::actions::Action::CloseWindow);
    h.run();
    fake.set_save(dir.path().join("one"));
    h.get_by_label(&t("unsaved.save_changes")).click();
    h.run();
    h.get_by_label(&t("replace.cancel")).click();
    h.run();
    assert!(h.state().modal.is_none());
    assert!(!h.state().close_confirmed());
    assert_eq!(root_text_in(&existing), "Old");

    // Replace saves tab 0, then the close moves on to tab 1.
    h.state_mut()
        .push(reasoncraft::actions::Action::CloseWindow);
    h.run();
    fake.set_save(dir.path().join("one"));
    h.get_by_label(&t("unsaved.save_changes")).click();
    h.run();
    h.get_by_label(&t("replace.confirm")).click();
    h.run();
    assert_eq!(root_text_in(&existing), "One");
    assert_eq!(h.state().active, 1);
    h.get_by_label(&t("unsaved.discard")).click();
    h.run();
    assert!(h.state().close_confirmed());
}

/// Up from a premise reaches its card; Ctrl+Shift+Tab and Ctrl+Shift+R add
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
    h.key_press_modifiers(cmd_shift, Key::Tab);
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

#[test]
fn evaluate_tab_rates_selected_card() {
    use reasoncraft_core::{CardKind, CardStrength, Document, Item, Parent};
    let mut d = Document::new("Claim");
    let root = d.root;
    let (card, _) = d
        .add_card(Parent::Box(root), CardKind::Reason, "Premise")
        .unwrap();
    let mut h = harness_with(d);
    h.state_mut().tabs[0].selection = Some(Item::Card(card));
    h.run();
    h.get_by_label(&t("ribbon.tab.evaluate")).click();
    h.run();
    let strong = t("eval.strong");
    assert_eq!(strong, "Strong");
    let toggled =
        |h: &Harness<'static, ReasonCraftApp>| h.get_by_label(&strong).accesskit_node().toggled();
    assert_eq!(toggled(&h), Some(egui::accesskit::Toggled::False));
    // A card takes no acceptance: that group is greyed out.
    assert!(
        h.get_by_label(&t("eval.accept"))
            .accesskit_node()
            .is_disabled()
    );
    h.get_by_label(&strong).click();
    h.run();
    let tab = &h.state().tabs[0];
    assert_eq!(tab.doc.cards[&card].strength, Some(CardStrength::Strong));
    assert_eq!(tab.history.undo.len(), 1);
    assert_eq!(toggled(&h), Some(egui::accesskit::Toggled::True));
}

#[test]
fn view_tab_toggles_evaluation_display() {
    let mut h = harness();
    h.run();
    h.get_by_label(&t("ribbon.tab.view")).click();
    h.run();
    h.get_by_label(&t("eval.show_markers")).click();
    h.run();
    assert!(!h.state().settings.show_markers);
    h.get_by_label(&t("eval.shading")).click();
    h.run();
    assert!(!h.state().settings.eval_shading);
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

#[test]
fn settings_from_old_storage_fill_defaults() {
    use eframe::Storage as _;
    use reasoncraft::settings::{SETTINGS_KEY, Settings};
    use reasoncraft::theme::ThemeChoice;
    let s: Settings = serde_json::from_str(r#"{"theme":"Light"}"#).unwrap();
    assert_eq!(s.theme, ThemeChoice::Light);
    assert_eq!(s.author, "");
    assert!(s.show_markers && s.eval_shading);

    // The same through eframe's own storage format, as a step-1 build wrote it.
    let mut storage = MemStorage::default();
    storage.set_string(SETTINGS_KEY, "(theme:Light)".to_owned());
    let s: Settings = eframe::get_value(&storage, SETTINGS_KEY).unwrap();
    assert_eq!(
        s,
        Settings {
            theme: ThemeChoice::Light,
            ..Settings::default()
        }
    );
    let d = Settings::default();
    assert_eq!(d.theme, ThemeChoice::Dark);
    assert!(d.author.is_empty() && d.show_markers && d.eval_shading);

    // The whole struct round-trips.
    let custom = Settings {
        theme: ThemeChoice::System,
        author: "Ann".to_owned(),
        show_markers: false,
        eval_shading: true,
    };
    eframe::set_value(&mut storage, SETTINGS_KEY, &custom);
    assert_eq!(
        eframe::get_value::<Settings>(&storage, SETTINGS_KEY),
        Some(custom)
    );
}

/// Alt+1 rates the selected claim; the "1" the OS also sends is not typed.
#[test]
fn alt_digit_rates_without_typing() {
    use reasoncraft_core::{Acceptance, Document};
    let d = Document::new("Claim");
    let root = d.root;
    let mut h = harness_with(d);
    h.state_mut().tabs[0].selection = Some(reasoncraft_core::Item::Box(root));
    h.run();
    let alt = |k| egui::Event::Key {
        key: k,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::ALT,
    };
    one_frame(
        &mut h,
        vec![alt(egui::Key::Num1), egui::Event::Text("1".to_owned())],
    );
    let tab = &h.state().tabs[0];
    assert_eq!(tab.doc.boxes[&root].acceptance, Some(Acceptance::Accept));
    assert_eq!(tab.doc.boxes[&root].text, "Claim");
    assert_eq!(tab.editing, None);
    assert_eq!(tab.history.undo.len(), 1);
    one_frame(&mut h, vec![alt(egui::Key::Num0)]);
    assert_eq!(h.state().tabs[0].doc.boxes[&root].acceptance, None);
}

/// A map with every kind of rating paints, with markers and shading on and off.
#[test]
fn rated_map_renders() {
    use reasoncraft_core::{Acceptance, BoxKind, CardStrength, SourceRating};
    let mut doc = reasoncraft_core::fixtures::sample_iot_like(40);
    let strengths = [
        Some(CardStrength::Strong),
        Some(CardStrength::Weak),
        Some(CardStrength::Nil),
        None,
    ];
    for (card, s) in doc.cards.values_mut().zip(strengths.into_iter().cycle()) {
        card.strength = s;
    }
    let acceptances = [Acceptance::Accept, Acceptance::Unsure, Acceptance::Reject];
    let ratings = [SourceRating::Solid, SourceRating::Shaky, SourceRating::Nil];
    let (mut a, mut r) = (0, 0);
    for b in doc.boxes.values_mut() {
        match b.kind {
            BoxKind::Claim | BoxKind::Premise => {
                b.acceptance = Some(acceptances[a % 3]);
                a += 1;
            }
            BoxKind::Source { .. } => {
                b.source_rating = Some(ratings[r % 3]);
                r += 1;
            }
            _ => {}
        }
    }
    assert!(a >= 3 && r >= 3, "the sample has premises and sources");
    let mut h = harness_with(doc);
    h.run_steps(3);
    // At 100% every label and marker is drawn (fitting may hide small text).
    h.state_mut().tabs[0].camera.zoom = 1.0;
    h.run_steps(3);
    h.state_mut().settings.show_markers = false;
    h.state_mut().settings.eval_shading = false;
    h.run_steps(3);
}

#[test]
fn plus_menu_source_submenu_adds_expert_opinion() {
    use reasoncraft_core::{BoxKind, CardKind, Document, Item, Parent, SourceType};
    let mut doc = Document::new("Claim");
    let (_, p) = doc
        .add_card(Parent::Box(doc.root), CardKind::Reason, "One")
        .unwrap();
    let mut h = harness_with(doc);
    // The building panel has items with the same names; hide it.
    h.state_mut().left_panel = false;
    h.state_mut().tabs[0].selection = Some(Item::Box(p));
    h.run();
    h.get_by_label(&t("plus.open")).click();
    h.run();
    // The ribbon's "Source" dropdown has the same name; the submenu is the lower one.
    let source = t("plus.source");
    let submenu = h
        .get_all_by_label(&source)
        .max_by(|a, b| a.rect().center().y.total_cmp(&b.rect().center().y))
        .expect("Source submenu in the plus menu");
    submenu.hover();
    h.run();
    h.get_by_label(&t("source.expert_opinion")).click();
    h.run();
    let tab = &h.state().tabs[0];
    let children = &tab.doc.boxes[&p].children;
    assert_eq!(children.len(), 1);
    let Item::Box(s) = children[0] else {
        panic!("not a box")
    };
    assert_eq!(
        tab.doc.boxes[&s].kind,
        BoxKind::Source {
            source_type: SourceType::ExpertOpinion
        }
    );
    assert_eq!(tab.selection, Some(Item::Box(s)));
    assert_eq!(tab.editing, Some(s));
    assert_eq!(tab.history.undo.len(), 1);
}

#[test]
fn ribbon_source_dropdown_adds_law_to_claim() {
    use reasoncraft_core::{BoxKind, Item, SourceType};
    let mut h = harness();
    // The building panel has items with the same names; hide it.
    h.state_mut().left_panel = false;
    h.run();
    h.get_by_label(&t("action.add_source")).click();
    h.run();
    h.get_by_label(&t("source.law")).click();
    h.run();
    let tab = &h.state().tabs[0];
    let root = tab.doc.root;
    let Some(&Item::Box(s)) = tab.doc.boxes[&root].children.first() else {
        panic!("no source under the claim")
    };
    assert_eq!(
        tab.doc.boxes[&s].kind,
        BoxKind::Source {
            source_type: SourceType::Law
        }
    );
    assert_eq!(tab.editing, Some(s));
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

#[test]
fn backstage_settings_set_author() {
    let mut h = harness();
    h.run();
    h.get_by_label(&t("ribbon.tab.file")).click();
    h.run();
    h.get_by_label(&t("backstage.settings")).click();
    h.run();
    // The lowest text field: the title bar has the command search.
    fn field<'a>(h: &'a Harness<'static, ReasonCraftApp>) -> egui_kittest::Node<'a> {
        h.get_all_by_role(egui::accesskit::Role::TextInput)
            .max_by(|a, b| a.rect().top().total_cmp(&b.rect().top()))
            .expect("author field")
    }
    field(&h).focus();
    h.run();
    field(&h).type_text("Ann ");
    h.run();
    assert_eq!(h.state().settings.author, "Ann");
    // The trailing space stays in the field while typing.
    field(&h).type_text("Lee");
    h.run();
    assert_eq!(h.state().settings.author, "Ann Lee");
    // The theme can be chosen here too.
    h.get_by_label(&t("theme.light")).click();
    h.run();
    assert_eq!(
        h.state().settings.theme,
        reasoncraft::theme::ThemeChoice::Light
    );
}
