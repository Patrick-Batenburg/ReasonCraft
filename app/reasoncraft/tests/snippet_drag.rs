//! UI tests for dragging snippet text onto the map.

use eframe::egui::{self, Event, Key, Modifiers};
use egui_kittest::{Harness, kittest::Queryable};
use reasoncraft::ReasonCraftApp;
use reasoncraft::canvas::measure::{EguiMeasure, MeasureCache};
use reasoncraft::doc_tab::DocTab;
use reasoncraft::i18n::t;
use reasoncraft::settings::TextTab;
use reasoncraft::shell::text_panel::chip_label;
use reasoncraft_core::{
    BoxKind, Document, Item, Layout, LayoutParams, NodeId, SnippetSource, SourceType, layout,
};

type H = Harness<'static, ReasonCraftApp>;

fn harness_with(doc: Document) -> H {
    let mut app = ReasonCraftApp::new_for_test();
    app.tabs[0] = DocTab::from_document(doc);
    app.settings.text_tab = TextTab::Snippets;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .with_max_steps(32)
        .build_ui_state(|ui, app: &mut ReasonCraftApp| app.show(ui), app);
    h.run();
    h
}

/// A document with one snippet, tagged with a web source.
fn doc_with_snippet(text: &str) -> Document {
    let mut d = Document::new("Robots help people");
    let id = d.add_snippet(text);
    let source = SnippetSource {
        source_type: SourceType::Web,
        citation: "Smith (2020)".into(),
        link: Some("https://example.org/robots".into()),
    };
    d.set_snippet_source(id, Some(source)).unwrap();
    d
}

fn layout_of(h: &H) -> Layout {
    let mut cache = MeasureCache::default();
    layout(
        &h.state().tabs[0].doc,
        &EguiMeasure::new(&h.ctx, &mut cache),
        &LayoutParams::default(),
    )
}

/// The screen centre of the root box.
fn root_on_screen(h: &H) -> egui::Pos2 {
    let tab = &h.state().tabs[0];
    tab.camera
        .world_to_screen(layout_of(h).boxes[&tab.doc.root].center())
}

/// Focus the snippet's text and select all of it.
fn select_all(h: &mut H) {
    h.get_by_label(&t("snippet.text")).click();
    h.run();
    h.key_press_modifiers(Modifiers::COMMAND, Key::A);
    h.run();
}

/// Press at `from`, move to `to` over several frames, release.
fn drag(h: &mut H, from: egui::Pos2, to: egui::Pos2) {
    h.event(Event::PointerMoved(from));
    h.run();
    h.drag_at(from);
    h.run();
    for i in 1..=8 {
        h.event(Event::PointerMoved(from.lerp(to, i as f32 / 8.0)));
        h.run();
    }
    h.drop_at(to);
    h.run();
}

/// The premise of the one card under the root, and the source under it.
fn dropped(h: &H) -> (NodeId, NodeId) {
    let doc = &h.state().tabs[0].doc;
    let [Item::Card(c)] = doc.boxes[&doc.root].children[..] else {
        panic!("one card under the root")
    };
    let p = doc.cards[&c].premises[0];
    let [Item::Box(s)] = doc.boxes[&p].children[..] else {
        panic!("one source under the premise")
    };
    (p, s)
}

#[test]
fn drag_chip_appears_for_selection() {
    let text = "Robots help people with heavy and repetitive work every day";
    let mut h = harness_with(doc_with_snippet(text));
    let label = chip_label(text);
    assert!(h.query_by_label(&label).is_none(), "no selection, no chip");
    select_all(&mut h);
    let chip = h.get_by_label(&label).rect().center();
    assert!(label.contains("Robots help people with heavy and"));
    assert!(label.contains('…'), "a long selection is cut short");

    let to = root_on_screen(&h);
    drag(&mut h, chip, to);
    let (p, s) = dropped(&h);
    let tab = &h.state().tabs[0];
    assert_eq!(tab.doc.boxes[&p].text, text);
    let src = &tab.doc.boxes[&s];
    assert_eq!(
        src.kind,
        BoxKind::Source {
            source_type: SourceType::Web
        }
    );
    assert_eq!(src.text, "Smith (2020)");
    assert_eq!(src.link.as_deref(), Some("https://example.org/robots"));
    assert_eq!(tab.selection, Some(Item::Box(p)));
    assert_eq!(tab.history.undo.len(), 1, "one step for card and source");
    assert_eq!(tab.doc.snippets[0].text, text, "the snippet is unchanged");
}

#[test]
fn blank_selection_shows_no_chip() {
    let mut h = harness_with(doc_with_snippet("   \n\t  "));
    select_all(&mut h);
    // Every chip's name starts like this one's.
    let any = chip_label("x");
    let prefix = &any[..any.find('x').unwrap()];
    assert!(h.query_by_label_contains(prefix).is_none());
    assert!(
        h.ctx.text_edit_focused(),
        "the text is focused and selected"
    );
}

#[test]
fn alt_drag_on_handle_carries_whole_snippet() {
    let mut h = harness_with(doc_with_snippet("  Robots save time  "));
    let handle = h.get_by_label(&t("snippet.drag")).rect().center();
    let to = root_on_screen(&h);
    h.event(Event::ModifiersChanged(Modifiers::ALT));
    drag(&mut h, handle, to);
    h.event(Event::ModifiersChanged(Modifiers::NONE));
    h.run();
    let (p, _) = dropped(&h);
    let tab = &h.state().tabs[0];
    assert_eq!(tab.doc.boxes[&p].text, "Robots save time");
    assert_eq!(tab.doc.snippets.len(), 1);
    assert_eq!(tab.history.undo.len(), 1, "no reorder step");

    // Without Alt the handle only reorders: nothing lands on the map.
    let mut h = harness_with(doc_with_snippet("Robots save time"));
    let handle = h.get_by_label(&t("snippet.drag")).rect().center();
    let to = root_on_screen(&h);
    drag(&mut h, handle, to);
    let doc = &h.state().tabs[0].doc;
    assert!(doc.boxes[&doc.root].children.is_empty());
}
