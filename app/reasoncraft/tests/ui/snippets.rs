//! UI tests for the Snippets tab of the Text panel.

use crate::common::{app_with_doc, build};
use std::time::{Duration, Instant};

use eframe::egui::{self, Key};
use egui_kittest::kittest::Queryable;
use reasoncraft::actions::Action;
use reasoncraft::canvas::input::{Os, paste_shortcut, shortcut_text};
use reasoncraft::i18n::{FluentArgs, t, t_args};
use reasoncraft::settings::TextTab;
use reasoncraft::shell::text_panel::capped_notice;
use reasoncraft_core::{CardKind, Document, Item, MAX_SNIPPET_CHARS, Parent};

use crate::common::H;

fn harness_with(doc: Document) -> H {
    let mut app = app_with_doc(doc);
    app.settings.text_tab = TextTab::Snippets;
    build(app)
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
    h.state_mut().pending.push(Action::Undo);
    h.run();
    assert_eq!(texts(&h), ["Robots"]);
}

#[test]
fn oversized_paste_is_capped() {
    let mut h = harness_with(doc_with(&[""]));
    h.run();
    assert!(h.query_by_label(&capped_notice()).is_none());
    h.get_by_label(&t("snippet.text")).click();
    h.run();
    assert!(focused_text_field(&h));

    let start = Instant::now();
    h.event(egui::Event::Paste("word ".repeat(24_000)));
    h.run();
    let paste_time = start.elapsed();
    let len = h.state().tabs[0].doc.snippets[0].text.chars().count();
    assert_eq!(len, MAX_SNIPPET_CHARS);
    h.get_by_label(&capped_notice());

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
    // Generous bounds (a debug build, other tests running alongside): a
    // long snippet laid out again in full each frame takes far longer.
    assert!(frame < Duration::from_secs(1), "frame took {frame:?}");
    assert!(
        paste_time < Duration::from_secs(10),
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
    // The typed text shows with a hint while the field has focus…
    h.get_by_label(&t("link.invalid"));
    assert_eq!(h.state().tabs[0].doc.snippets[0].source, None);
    h.key_press(Key::Enter);
    h.run();
    // …and is not stored.
    assert_eq!(h.state().tabs[0].doc.snippets[0].source, None);
    assert_eq!(undo_len(&h), 0);
    assert!(h.query_by_label(&t("link.invalid")).is_none());
    assert!(!h.state().tabs[0].is_dirty());
}

#[test]
fn citation_typing_is_live_and_kept_on_tab_switch() {
    let mut h = harness_with(doc_with(&["Quote"]));
    h.run();
    h.get_by_label(&t("snippet.citation")).click();
    h.run();
    h.get_by_label(&t("snippet.citation"))
        .type_text("  Doe 2020 ");
    h.run();
    let citation = |h: &H| {
        let s = h.state().tabs[0].doc.snippets[0].source.as_ref();
        s.map(|s| s.citation.clone())
    };
    assert_eq!(citation(&h).as_deref(), Some("  Doe 2020 "), "live");
    assert_eq!(undo_len(&h), 0, "not while typing");
    assert!(h.state().tabs[0].is_dirty());

    // Another Text tab, without the field losing focus first.
    h.state_mut()
        .pending
        .push(Action::SetTextTab(TextTab::Outline));
    h.run();
    assert_eq!(citation(&h).as_deref(), Some("Doe 2020"));
    assert_eq!(undo_len(&h), 1);
    h.state_mut().pending.push(Action::Undo);
    h.run();
    assert_eq!(h.state().tabs[0].doc.snippets[0].source, None);

    // Back on the tab, the field shows the stored citation (none).
    h.state_mut()
        .pending
        .push(Action::SetTextTab(TextTab::Snippets));
    h.run();
    let label = t("snippet.citation");
    let value = h.get_by_label(&label).value();
    assert_eq!(value.as_deref().unwrap_or(""), "");
}

#[test]
fn closing_window_while_typing_citation_asks_to_save() {
    let mut h = harness_with(doc_with(&["Quote"]));
    h.run();
    h.get_by_label(&t("snippet.citation")).click();
    h.run();
    h.get_by_label(&t("snippet.citation")).type_text("Doe");
    h.run();
    h.state_mut().pending.push(Action::CloseWindow);
    h.run();
    assert!(h.state().modal.is_some(), "unsaved prompt");
    assert!(!h.state().close_confirmed());
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
fn snippet_dropped_outside_the_list_stays() {
    let mut h = harness_with(doc_with(&["a", "b", "c"]));
    h.run();
    let handles = centres(&h, &t("snippet.drag"));
    // Over the map, left of the Text panel.
    let (from, to) = (handles[2], egui::pos2(200.0, 450.0));
    h.event(egui::Event::PointerMoved(from));
    h.drag_at(from);
    h.run();
    for i in 1..=8 {
        h.event(egui::Event::PointerMoved(from.lerp(to, i as f32 / 8.0)));
        h.run();
    }
    h.drop_at(to);
    h.run();
    assert_eq!(texts(&h), ["a", "b", "c"]);
    assert_eq!(undo_len(&h), 0, "nothing moved");
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

/// The empty Snippets tab's text (tests run with the Windows keys).
fn empty_label() -> String {
    let mut keys = FluentArgs::new();
    keys.set("paste", shortcut_text(paste_shortcut(), Os::Windows));
    t_args("snippet.empty", &keys)
}

#[test]
fn new_snippet_takes_focus_and_ctrl_v_pastes_into_it() {
    let mut h = harness_with(doc_with(&[]));
    h.run();
    h.get_by_label(&empty_label());
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
    let over = h.get_by_label(&empty_label()).rect().center();
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
