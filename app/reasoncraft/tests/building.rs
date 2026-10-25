//! UI tests for the building panel (click and drag-and-drop) and for
//! presses on comment badges and around the comment popover.

use eframe::egui;
use egui_kittest::{Harness, kittest::Queryable};
use reasoncraft::ReasonCraftApp;
use reasoncraft::canvas::measure::{EguiMeasure, MeasureCache};
use reasoncraft::i18n::t;
use reasoncraft_core::{BoxKind, CardKind, Document, Item, Layout, LayoutParams, Parent, layout};

type H = Harness<'static, ReasonCraftApp>;

fn harness_with(doc: Document) -> H {
    let mut app = ReasonCraftApp::new_for_test();
    app.tabs[0] = reasoncraft::doc_tab::DocTab::from_document(doc);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .with_max_steps(32)
        .build_ui_state(|ui, app: &mut ReasonCraftApp| app.show(ui), app);
    h.run();
    h
}

/// The active document's layout, measured like the canvas does.
fn layout_of(h: &H) -> Layout {
    let mut cache = MeasureCache::default();
    layout(
        &h.state().tabs[0].doc,
        &EguiMeasure::new(&h.ctx, &mut cache),
        &LayoutParams::default(),
    )
}

/// Screen position of world point `p` in the active tab.
fn screen(h: &H, p: egui::Pos2) -> egui::Pos2 {
    h.state().tabs[0].camera.world_to_screen(p)
}

/// The centre of the building-panel item called `label` (the lowest of
/// the widgets with that name, so not the ribbon's).
fn panel_item(h: &H, label: &str) -> egui::Pos2 {
    h.get_all_by_label(label)
        .max_by(|a, b| a.rect().top().total_cmp(&b.rect().top()))
        .expect("panel item")
        .rect()
        .center()
}

/// Press at `from`, move to `to` over several frames, release.
fn drag(h: &mut H, from: egui::Pos2, to: egui::Pos2) {
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
fn panel_click_adds_note_to_selection() {
    let mut h = harness_with(Document::new("Claim"));
    let root = h.state().tabs[0].doc.root;
    h.state_mut().tabs[0].selection = Some(Item::Box(root));
    h.run();
    let at = panel_item(&h, &t("action.add_note"));
    assert!(at.x < 240.0, "in the left panel: {at:?}");
    h.event(egui::Event::PointerMoved(at));
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run();
    let tab = &h.state().tabs[0];
    let [Item::Box(n)] = tab.doc.boxes[&root].children[..] else {
        panic!("one child")
    };
    assert_eq!(tab.doc.boxes[&n].kind, BoxKind::Note);
    assert_eq!(tab.selection, Some(Item::Box(n)));
    assert_eq!(tab.editing, Some(n));
    assert_eq!(tab.history.undo.len(), 1);
}

#[test]
fn refused_panel_drop_changes_nothing() {
    let mut doc = Document::new("Claim");
    let cat = doc
        .add_box(Parent::Box(doc.root), BoxKind::Category, "Category")
        .unwrap();
    let mut h = harness_with(doc);
    let before = h.state().tabs[0].doc.clone();
    let to = screen(&h, layout_of(&h).boxes[&cat].center());
    let from = panel_item(&h, &t("source.web"));

    // Hovering over the category shows "not allowed".
    h.event(egui::Event::PointerMoved(from));
    h.drag_at(from);
    h.run();
    for i in 1..=8 {
        h.event(egui::Event::PointerMoved(from.lerp(to, i as f32 / 8.0)));
        h.run();
    }
    assert_eq!(
        h.output().platform_output.cursor_icon,
        egui::CursorIcon::NotAllowed
    );
    h.drop_at(to);
    h.run();
    let tab = &h.state().tabs[0];
    assert_eq!(tab.doc, before);
    assert_eq!(tab.history.undo.len(), 0);
}

#[test]
fn panel_drop_inserts_at_the_target() {
    let mut doc = Document::new("Claim");
    let (c, p) = doc
        .add_card(Parent::Box(doc.root), CardKind::Reason, "Premise")
        .unwrap();
    let mut h = harness_with(doc);
    // A source onto the premise.
    let to = screen(&h, layout_of(&h).boxes[&p].center());
    let from = panel_item(&h, &t("source.web"));
    drag(&mut h, from, to);
    {
        let tab = &h.state().tabs[0];
        let [Item::Box(s)] = tab.doc.boxes[&p].children[..] else {
            panic!("source under the premise")
        };
        assert!(matches!(tab.doc.boxes[&s].kind, BoxKind::Source { .. }));
        assert_eq!(tab.editing, Some(s));
        assert_eq!(tab.history.undo.len(), 1);
    }
    // A comment preset onto the premise: added directly, no popover.
    // (Collapse the sources so the last presets are in view.)
    h.key_press(egui::Key::Escape);
    h.get_by_label(&t("building.sources")).click();
    h.run();
    let to = screen(&h, layout_of(&h).boxes[&p].center());
    let from = panel_item(&h, &t("comment.preset.unclear"));
    drag(&mut h, from, to);
    let tab = &h.state().tabs[0];
    assert_eq!(tab.doc.boxes[&p].comments.len(), 1);
    assert_eq!(tab.comments_open, None);
    assert_eq!(tab.doc.cards[&c].premises.len(), 1);
}

#[test]
fn dragging_a_comment_badge_moves_nothing() {
    use reasoncraft::canvas::comments::badge_rect;
    let mut doc = Document::new("Claim");
    let root = doc.root;
    let (c, p) = doc
        .add_card(Parent::Box(root), CardKind::Reason, "Premise")
        .unwrap();
    doc.add_comment(p, "Ann", "Hm.", None).unwrap();
    let mut h = harness_with(doc);
    let badge = screen(&h, badge_rect(layout_of(&h).boxes[&p], 1).center());
    let offset = h.state().tabs[0].camera.offset;
    let view = h.state().tabs[0].viewport.unwrap();
    drag(&mut h, badge, view.left_bottom() + egui::vec2(40.0, -40.0));
    let tab = &h.state().tabs[0];
    assert_eq!(tab.doc.boxes[&root].children, vec![Item::Card(c)]);
    assert!(tab.doc.parking.is_empty());
    assert_eq!(tab.history.undo.len(), 0);
    assert_eq!(tab.camera.offset, offset, "no pan");
}

#[test]
fn panning_keeps_comment_popover_open() {
    let mut h = harness_with(Document::new("Claim"));
    let root = h.state().tabs[0].doc.root;
    h.state_mut().tabs[0].selection = Some(Item::Box(root));
    h.run();
    h.get_by_label(&t("action.comment")).click();
    h.run();
    assert_eq!(h.state().tabs[0].comments_open, Some(root));
    let offset = h.state().tabs[0].camera.offset;
    let from = h.state().tabs[0].viewport.unwrap().left_bottom() + egui::vec2(60.0, -60.0);
    for button in [egui::PointerButton::Secondary, egui::PointerButton::Middle] {
        let button_event = |pos, pressed| egui::Event::PointerButton {
            pos,
            button,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        h.event(egui::Event::PointerMoved(from));
        h.event(button_event(from, true));
        h.run();
        for i in 1..=4 {
            h.event(egui::Event::PointerMoved(
                from + egui::vec2(20.0 * i as f32, 0.0),
            ));
            h.run();
        }
        h.event(button_event(from + egui::vec2(80.0, 0.0), false));
        h.run();
        assert_eq!(h.state().tabs[0].comments_open, Some(root), "{button:?}");
    }
    assert_ne!(h.state().tabs[0].camera.offset, offset, "the map panned");
}
