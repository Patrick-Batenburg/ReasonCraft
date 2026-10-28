//! UI tests for find and replace: the find bar, its keys, Replace and
//! Replace all as history steps, and hit outlines staying off exports.

use crate::common;
use std::path::PathBuf;

use eframe::egui::{Key, Modifiers};
use egui_kittest::kittest::Queryable;
use reasoncraft::ReasonCraftApp;
use reasoncraft::actions::{Action, apply};
use reasoncraft::doc_tab::DocTab;
use reasoncraft::export::{ExportFormat, ExportOptions, ExportRequest, PageSetup};
use reasoncraft::file_ops::FakeDialogs;
use reasoncraft::i18n::t;
use reasoncraft::theme::Tokens;
use reasoncraft_core::MAX_BOX_CHARS;
use reasoncraft_core::{CardKind, Document, Item, NodeId, Parent};

use crate::common::H;

/// A claim with `claim` as its text, reasons "cat one" and "dog", and an
/// objection "black cat"; returns the doc and the three premises.
fn doc(claim: &str) -> (Document, [NodeId; 3]) {
    let mut d = Document::new(claim);
    let root = Parent::Box(d.root);
    let (_, one) = d.add_card(root, CardKind::Reason, "cat one").unwrap();
    let (_, dog) = d.add_card(root, CardKind::Reason, "dog").unwrap();
    let (_, two) = d.add_card(root, CardKind::Objection, "black cat").unwrap();
    (d, [one, dog, two])
}

fn harness(doc: Document, dialogs: FakeDialogs) -> H {
    let mut app = ReasonCraftApp::new_for_test_with(Box::new(dialogs));
    app.tabs[0] = DocTab::from_document(doc);
    common::harness(app)
}

fn text(h: &H, node: NodeId) -> String {
    h.state().tabs[0].doc.boxes[&node].text.clone()
}

fn focused(h: &H, label: &str) -> bool {
    h.get_by_label(&t(label)).is_focused()
}

#[test]
fn find_bar_keyboard() {
    let (d, [one, _, two]) = doc("Claim");
    let root = d.root;
    let mut h = harness(d, FakeDialogs::default());
    assert!(h.query_by_label(&t("find.query")).is_none(), "closed");
    h.key_press_modifiers(Modifiers::COMMAND, Key::F);
    h.run();
    assert!(h.state().tabs[0].find.open, "open");
    assert!(h.query_by_label(&t("find.match_case.tip")).is_some());
    assert!(focused(&h, "find.query"), "Ctrl+F focuses the field");
    assert!(h.query_by_label(&t("find.replacement")).is_none());
    assert_eq!(h.state().tabs[0].selection, Some(Item::Box(root)));

    h.get_by_label(&t("find.query")).type_text("cat");
    h.run();
    assert_eq!(
        h.state().tabs[0].selection,
        Some(Item::Box(one)),
        "first hit"
    );
    assert!(h.query_by_label("1 of 2").is_some());
    h.key_press(Key::F3);
    h.run();
    assert_eq!(h.state().tabs[0].selection, Some(Item::Box(two)), "F3");
    assert!(h.query_by_label("2 of 2").is_some());
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state().tabs[0].selection, Some(Item::Box(one)), "wraps");
    assert!(focused(&h, "find.query"), "Enter keeps the focus");
    h.key_press_modifiers(Modifiers::SHIFT, Key::Enter);
    h.run();
    assert_eq!(h.state().tabs[0].selection, Some(Item::Box(two)));
    h.key_press_modifiers(Modifiers::SHIFT, Key::F3);
    h.run();
    assert_eq!(h.state().tabs[0].selection, Some(Item::Box(one)));
    assert_eq!(text(&h, one), "cat one", "nothing typed into the map");

    h.key_press(Key::Escape);
    h.run();
    assert!(!h.state().tabs[0].find.open);
    assert!(h.query_by_label(&t("find.query")).is_none());
    assert!(!h.ctx.text_edit_focused());
    // The map has the keys again: Ctrl+0 zooms, Up selects the card.
    h.state_mut().tabs[0].camera.zoom = 0.5;
    h.key_press_modifiers(Modifiers::COMMAND, Key::Num0);
    h.run();
    assert_eq!(h.state().tabs[0].camera.zoom, 1.0);
    h.key_press(Key::ArrowUp);
    h.run();
    assert!(matches!(h.state().tabs[0].selection, Some(Item::Card(_))));
}

#[test]
fn replace_current_then_next() {
    let (d, [one, _, two]) = doc("Claim");
    let mut h = harness(d, FakeDialogs::default());
    h.key_press_modifiers(Modifiers::COMMAND, Key::H);
    h.run();
    assert!(focused(&h, "find.query"));
    h.get_by_label(&t("find.query")).type_text("cat");
    h.run();
    h.get_by_label(&t("find.replacement")).click();
    h.run();
    h.get_by_label(&t("find.replacement")).type_text("dog");
    h.run();
    assert_eq!(h.state().tabs[0].selection, Some(Item::Box(one)));

    // Enter in the replacement field replaces and moves on.
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(text(&h, one), "dog one");
    assert_eq!(text(&h, two), "black cat");
    assert_eq!(h.state().tabs[0].selection, Some(Item::Box(two)), "next");
    assert_eq!(h.state().tabs[0].history.undo.len(), 1);
    assert!(h.query_by_label("1 of 1").is_some());

    apply(h.state_mut(), Action::ReplaceCurrent);
    h.run();
    assert_eq!(text(&h, two), "black dog");
    assert_eq!(h.state().tabs[0].history.undo.len(), 2);
    assert!(h.state().tabs[0].find.current.is_none());
    assert!(h.query_by_label(&t("find.none")).is_some());
    // Nothing left: Replace is a no-op, not a step.
    apply(h.state_mut(), Action::ReplaceCurrent);
    assert_eq!(h.state().tabs[0].history.undo.len(), 2);
}

/// A replacement that contains the query goes past the text it put in.
#[test]
fn replace_current_skips_its_own_text() {
    let d = Document::new("a a");
    let root = d.root;
    let mut h = harness(d, FakeDialogs::default());
    let app = h.state_mut();
    apply(app, Action::OpenFind { replace: true });
    apply(app, Action::SetFindQuery("a".into()));
    apply(app, Action::SetReplaceText("aa".into()));
    apply(app, Action::ReplaceCurrent);
    assert_eq!(app.tabs[0].doc.boxes[&root].text, "aa a");
    let current = app.tabs[0].find.current.clone().unwrap();
    assert_eq!(current.range, 3..4, "the second a, not the inserted one");
    h.run();
}

#[test]
fn replace_all_single_history_entry() {
    let (d, [one, dog, two]) = doc("cat claim");
    let root = d.root;
    let mut h = harness(d, FakeDialogs::default());
    let app = h.state_mut();
    apply(app, Action::OpenFind { replace: true });
    apply(app, Action::SetFindQuery("CAT".into()));
    apply(app, Action::SetReplaceText("dog".into()));
    apply(app, Action::ReplaceAll);
    let tab = &app.tabs[0];
    assert_eq!(tab.doc.boxes[&root].text, "dog claim");
    assert_eq!(tab.doc.boxes[&one].text, "dog one");
    assert_eq!(tab.doc.boxes[&two].text, "black dog");
    assert_eq!(tab.history.undo.len(), 1, "one step");
    assert_eq!(app.status_note.as_deref(), Some("Replaced 3"));
    h.run();
    assert!(
        h.query_by_label("Replaced 3").is_some(),
        "status note shows"
    );

    apply(h.state_mut(), Action::Undo);
    let tab = &h.state().tabs[0];
    assert_eq!(tab.doc.boxes[&root].text, "cat claim");
    assert_eq!(tab.doc.boxes[&one].text, "cat one");
    assert_eq!(tab.doc.boxes[&two].text, "black cat");
    assert_eq!(tab.doc.boxes[&dog].text, "dog");

    // No hits: no step.
    apply(h.state_mut(), Action::SetFindQuery("zebra".into()));
    apply(h.state_mut(), Action::ReplaceAll);
    assert_eq!(h.state().tabs[0].history.undo.len(), 0);
}

/// Export `h`'s map as SVG to `path`, waiting for the background job.
fn export_svg(h: &mut H, dialogs: &FakeDialogs, path: PathBuf) -> String {
    dialogs.set_export(path.clone());
    let request = ExportRequest {
        format: ExportFormat::Svg,
        options: ExportOptions {
            svg_outlines: false,
            ..ExportOptions::default()
        },
        page: PageSetup::default(),
    };
    apply(h.state_mut(), Action::RunExport(request));
    crate::common::step_until(h, |app| !app.export.busy(), "the export");
    std::fs::read_to_string(path).unwrap()
}

#[test]
fn find_highlight_not_in_exports() {
    let dir = tempfile::tempdir().unwrap();
    let dialogs = FakeDialogs::default();
    let (d, _) = doc("Claim");
    let mut h = harness(d, dialogs.clone());
    let plain = export_svg(&mut h, &dialogs, dir.path().join("plain.svg"));
    apply(h.state_mut(), Action::OpenFind { replace: false });
    apply(h.state_mut(), Action::SetFindQuery("cat".into()));
    apply(h.state_mut(), Action::Select(None));
    h.run();
    assert_eq!(h.state_mut().tabs[0].find_hits().len(), 2, "hits on screen");
    let found = export_svg(&mut h, &dialogs, dir.path().join("found.svg"));
    assert_eq!(plain, found, "find changes nothing in an export");
    for t in [Tokens::dark(), Tokens::light()] {
        let [r, g, b, _] = t.find_hit.to_srgba_unmultiplied();
        let hex = format!("#{r:02x}{g:02x}{b:02x}");
        assert!(!found.contains(&hex), "{hex} in the export");
    }
}

/// A replacement never makes a box longer than the inline editor allows:
/// the box is cut in the same step, with the "text was cut" note.
#[test]
fn replace_is_capped_at_the_box_limit() {
    let full = format!("x{}", "a".repeat(MAX_BOX_CHARS - 1));
    let mut d = Document::new(&full);
    let root = d.root;
    let (_, p) = d
        .add_card(Parent::Box(root), CardKind::Reason, &full)
        .unwrap();
    let mut h = harness(d, FakeDialogs::default());
    let app = h.state_mut();
    apply(app, Action::OpenFind { replace: true });
    apply(app, Action::SetFindQuery("x".into()));
    apply(app, Action::SetReplaceText("yyy".into()));
    apply(app, Action::ReplaceCurrent);
    let root_text = app.tabs[0].doc.boxes[&root].text.clone();
    assert_eq!(root_text.chars().count(), MAX_BOX_CHARS);
    assert!(root_text.starts_with("yyya"));
    assert_eq!(app.tabs[0].history.undo.len(), 1);
    assert_eq!(
        app.status_note.as_deref(),
        Some(t("status.text_cut").as_str())
    );

    apply(app, Action::Undo);
    apply(app, Action::ReplaceAll);
    for node in [root, p] {
        let text = &app.tabs[0].doc.boxes[&node].text;
        assert_eq!(text.chars().count(), MAX_BOX_CHARS);
        assert!(text.starts_with("yyya"));
    }
    assert_eq!(app.tabs[0].history.undo.len(), 1, "one step");
    assert_eq!(
        app.status_note.as_deref(),
        Some(t("status.text_cut").as_str())
    );
    apply(app, Action::Undo);
    assert_eq!(app.tabs[0].doc.boxes[&p].text, full);
    h.run();
}

/// Replace all during an inline edit: the edit is committed first as its
/// own step, then the replacement covers the typed text too.
#[test]
fn replace_all_while_editing() {
    let (d, _) = doc("Claim");
    let root = d.root;
    let mut h = harness(d, FakeDialogs::default());
    let app = h.state_mut();
    apply(app, Action::OpenFind { replace: true });
    apply(app, Action::SetFindQuery("claim".into()));
    apply(app, Action::SetReplaceText("Thesis".into()));
    apply(app, Action::Select(Some(Item::Box(root))));
    apply(app, Action::TypeText(" typed claim".into()));
    assert_eq!(app.tabs[0].editing(), Some(root));
    apply(app, Action::ReplaceAll);
    let tab = &app.tabs[0];
    assert_eq!(tab.editing(), None);
    assert_eq!(tab.doc.boxes[&root].text, "Thesis typed Thesis");
    assert_eq!(tab.history.undo.len(), 2, "the typing, then the replace");
    apply(app, Action::Undo);
    assert_eq!(app.tabs[0].doc.boxes[&root].text, "Claim typed claim");
    h.run();
}

/// When the current hit's box text changes, F3 and Shift+F3 go on from
/// where that hit was, not from the first hit.
#[test]
fn stale_hit_resumes_from_its_place() {
    let (d, [one, _, two]) = doc("Claim");
    let mut h = harness(d, FakeDialogs::default());
    let app = h.state_mut();
    apply(app, Action::OpenFind { replace: false });
    apply(app, Action::SetFindQuery("cat".into()));
    apply(app, Action::FindNext);
    assert_eq!(app.tabs[0].find.current.as_ref().unwrap().node, two);
    let stale = |app: &mut ReasonCraftApp, text: &str| {
        app.tabs[0].edit(|d| d.set_text(two, text));
        assert_eq!(app.tabs[0].find_position(), None, "stale");
    };

    stale(app, "the black cat");
    apply(app, Action::FindNext);
    let hit = app.tabs[0].find.current.clone().unwrap();
    assert_eq!((hit.node, hit.range), (two, 10..13), "not back to hit 1");
    assert_eq!(app.tabs[0].selection, Some(Item::Box(two)));

    stale(app, "a very black cat");
    apply(app, Action::FindPrev);
    let hit = app.tabs[0].find.current.clone().unwrap();
    assert_eq!((hit.node, hit.range), (one, 0..3), "the hit before it");
    h.run();
}
