//! UI tests for the Snippets tab of the Text panel.

use std::time::{Duration, Instant};

use eframe::egui::{self, Key};
use egui_kittest::{Harness, kittest::Queryable};
use reasoncraft::ReasonCraftApp;
use reasoncraft::doc_tab::DocTab;
use reasoncraft::i18n::t;
use reasoncraft::settings::TextTab;
use reasoncraft_core::{CardKind, Document, Item, MAX_SNIPPET_CHARS, Parent};

type H = Harness<'static, ReasonCraftApp>;

fn harness_with(doc: Document) -> H {
    let mut app = ReasonCraftApp::new_for_test();
    app.tabs[0] = DocTab::from_document(doc);
    app.settings.text_tab = TextTab::Snippets;
    Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .with_max_steps(32)
        .build_ui_state(|ui, app: &mut ReasonCraftApp| app.show(ui), app)
}

fn doc_with(texts: &[&str]) -> Document {
    let mut d = Document::new("Robots help people");
    for text in texts {
        d.add_snippet(text);
    }
    d
}

fn texts(h: &H) -> Vec<String> {
    let doc = &h.state().tabs[0].doc;
    doc.snippets.iter().map(|s| s.text.clone()).collect()
}

fn undo_len(h: &H) -> usize {
    h.state().tabs[0].history.undo.len()
}

/// The centres of the widgets labelled `label`, top to bottom.
fn centres(h: &H, label: &str) -> Vec<egui::Pos2> {
    let mut at: Vec<egui::Pos2> = h
        .get_all_by_label(label)
        .map(|n| n.rect().center())
        .collect();
    at.sort_by(|a, b| a.y.total_cmp(&b.y));
    at
}

fn focused_text_field(h: &H) -> bool {
    h.ctx.text_edit_focused()
}

#[test]
fn snippet_edit_is_one_history_entry() {
    let mut h = harness_with(doc_with(&["Robots"]));
    h.run();
    // Click first: a typed character with no field focused types into the
    // selected box.
    h.get_by_label(&t("snippet.text")).click();
    h.run();
    h.get_by_label(&t("snippet.text")).type_text(" help");
    h.run();
    assert_eq!(texts(&h), ["Robots help"]);
    assert_eq!(undo_len(&h), 0, "typing is live");
    assert!(h.state().tabs[0].title().ends_with('•'), "unsaved typing");

    h.key_press(Key::Escape);
    h.run();
    assert!(!focused_text_field(&h));
    assert_eq!(undo_len(&h), 1);
    h.state_mut()
        .pending
        .push(reasoncraft::actions::Action::Undo);
    h.run();
    assert_eq!(texts(&h), ["Robots"]);
}

#[test]
fn oversized_paste_is_capped() {
    let mut h = harness_with(doc_with(&[""]));
    h.run();
    assert!(h.query_by_label(&t("snippet.capped")).is_none());
    h.get_by_label(&t("snippet.text")).click();
    h.run();
    assert!(focused_text_field(&h));

    let start = Instant::now();
    h.event(egui::Event::Paste("word ".repeat(24_000)));
    h.run();
    let paste_time = start.elapsed();
    let len = h.state().tabs[0].doc.snippets[0].text.chars().count();
    assert_eq!(len, MAX_SNIPPET_CHARS);
    h.get_by_label(&t("snippet.capped"));

    // Later frames with the long snippet stay fast.
    let start = Instant::now();
    for i in 0..5 {
        h.event(egui::Event::PointerMoved(egui::pos2(
            700.0,
            300.0 + i as f32,
        )));
        h.step();
    }
    let frame = start.elapsed() / 5;
    eprintln!("paste {paste_time:?}, frame {frame:?}");
    assert!(frame < Duration::from_millis(100), "frame took {frame:?}");
    assert!(
        paste_time < Duration::from_secs(2),
        "paste took {paste_time:?}"
    );
}

#[test]
fn invalid_snippet_link_not_stored() {
    let mut h = harness_with(doc_with(&["Quote"]));
    h.run();
    h.get_by_label(&t("snippet.link")).click();
    h.run();
    h.get_by_label(&t("snippet.link"))
        .type_text("javascript:alert(1)");
    h.run();
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state().tabs[0].doc.snippets[0].source, None);
    assert_eq!(undo_len(&h), 0);
    h.get_by_label(&t("link.invalid"));

    // A citation is stored when its field loses focus.
    h.get_by_label(&t("snippet.citation")).click();
    h.run();
    h.get_by_label(&t("snippet.citation"))
        .type_text("  Doe 2020 ");
    h.run();
    assert_eq!(undo_len(&h), 0, "not while typing");
    h.key_press(Key::Enter);
    h.run();
    let source = h.state().tabs[0].doc.snippets[0].source.clone().unwrap();
    assert_eq!(source.citation, "Doe 2020");
    assert_eq!(source.link, None);
    assert_eq!(undo_len(&h), 1);
}

#[test]
fn snippet_reorder() {
    let mut h = harness_with(doc_with(&["a", "b", "c"]));
    h.run();
    let handles = centres(&h, &t("snippet.drag"));
    assert_eq!(handles.len(), 3);
    let (from, to) = (handles[2], handles[0] - egui::vec2(0.0, 20.0));
    h.event(egui::Event::PointerMoved(from));
    h.drag_at(from);
    h.run();
    for i in 1..=8 {
        h.event(egui::Event::PointerMoved(from.lerp(to, i as f32 / 8.0)));
        h.run();
    }
    h.drop_at(to);
    h.run();
    assert_eq!(texts(&h), ["c", "a", "b"]);
    assert_eq!(undo_len(&h), 1);
}

#[test]
fn delete_key_in_snippet_keeps_selected_box() {
    let mut d = doc_with(&["Some text"]);
    let (_, premise) = d
        .add_card(Parent::Box(d.root), CardKind::Reason, "Premise")
        .unwrap();
    let mut h = harness_with(d);
    h.state_mut().tabs[0].selection = Some(Item::Box(premise));
    h.run();
    h.get_by_label(&t("snippet.text")).click();
    h.run();
    for key in [Key::Delete, Key::Backspace] {
        h.key_press(key);
        h.run();
    }
    let tab = &h.state().tabs[0];
    assert!(tab.doc.boxes.contains_key(&premise));
    assert_eq!(tab.selection, Some(Item::Box(premise)));
    assert_eq!(texts(&h).len(), 1);
}

#[test]
fn new_snippet_takes_focus_and_ctrl_v_pastes_into_it() {
    let mut h = harness_with(doc_with(&[]));
    h.run();
    h.get_by_label(&t("snippet.empty"));
    h.get_by_label(&t("snippet.new")).click();
    h.run();
    assert_eq!(texts(&h), [""]);
    assert!(focused_text_field(&h), "the new snippet has focus");

    h.event(egui::Event::Paste("Robots do dull work.".into()));
    h.run();
    assert_eq!(texts(&h), ["Robots do dull work."]);
    h.key_press(Key::Escape);
    h.run();
    assert_eq!(undo_len(&h), 1, "the add and its text are one step");
}

#[test]
fn paste_over_tab_adds_snippet() {
    let mut h = harness_with(doc_with(&[]));
    h.run();
    // Ctrl+V with the pointer over the tab and no text field focused.
    let over = h.get_by_label(&t("snippet.empty")).rect().center();
    h.event(egui::Event::PointerMoved(over));
    h.event(egui::Event::Paste("First".into()));
    h.run();
    assert_eq!(texts(&h), ["First"]);
    assert_eq!(undo_len(&h), 1);

    // "Paste as snippet": the window answers with a paste event.
    h.get_by_label(&t("snippet.paste")).click();
    h.step();
    h.event(egui::Event::PointerMoved(egui::pos2(600.0, 400.0)));
    h.event(egui::Event::Paste("Second".into()));
    h.run();
    assert_eq!(texts(&h), ["First", "Second"]);

    // Over the map, a paste is not a snippet.
    h.event(egui::Event::Paste("Third".into()));
    h.run();
    assert_eq!(texts(&h).len(), 2);
}
