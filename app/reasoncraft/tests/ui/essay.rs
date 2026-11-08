//! UI tests for the Essay tab of the Text panel.

use crate::common::{app_with_doc, build};
use eframe::egui::{self, Key, accesskit::Role};
use egui_kittest::kittest::{NodeT, Queryable};
use reasoncraft::ReasonCraftApp;
use reasoncraft::actions::{Action, UnsavedChoice, apply};
use reasoncraft::doc_tab::{DocTab, EditSession};
use reasoncraft::file_ops::{FakeDialogs, Modal};
use reasoncraft::i18n::{FluentArgs, t, t_args};
use reasoncraft::settings::TextTab;
use reasoncraft_core::{
    CardId, CardKind, CardStrength, Document, EssaySlot, Item, NodeId, Parent, file,
};

use crate::common::H;

fn harness_with(doc: Document) -> H {
    let mut app = app_with_doc(doc);
    app.settings.text_tab = TextTab::Essay;
    build(app)
}

/// Claim with a weak reason, a strong objection, an unrated reason and a
/// nil reason, added in that order.
fn rated_doc() -> (Document, Vec<CardId>) {
    let mut d = Document::new("Cities should ban cars");
    let root = Parent::Box(d.root);
    let mut cards = Vec::new();
    for (kind, text, strength) in [
        (CardKind::Reason, "Weak reason", Some(CardStrength::Weak)),
        (
            CardKind::Objection,
            "Strong objection",
            Some(CardStrength::Strong),
        ),
        (CardKind::Reason, "Plain reason", None),
        (CardKind::Reason, "Nil reason", Some(CardStrength::Nil)),
    ] {
        let (c, _) = d.add_card(root, kind, text).unwrap();
        d.set_card_strength(c, strength).unwrap();
        cards.push(c);
    }
    (d, cards)
}

fn box_with_text(doc: &Document, text: &str) -> NodeId {
    *doc.boxes.iter().find(|(_, b)| b.text == text).unwrap().0
}

fn undo_len(h: &H) -> usize {
    h.state().tabs[0].history.undo.len()
}

/// The labels of the section headings, top to bottom.
fn headings(h: &H) -> Vec<String> {
    let mut found: Vec<(f32, String)> = h
        .query_all_by_role(Role::Heading)
        .map(|n| {
            (
                n.rect().min.y,
                n.accesskit_node().label().unwrap_or_default(),
            )
        })
        .collect();
    found.sort_by(|a, b| a.0.total_cmp(&b.0));
    found.into_iter().map(|(_, l)| l).collect()
}

fn paragraph_label(statement: &str) -> String {
    let mut args = FluentArgs::new();
    args.set("statement", statement);
    t_args("essay.paragraph_for", &args)
}

#[test]
fn essay_tab_lists_sections_strongest_first() {
    let (doc, _) = rated_doc();
    let reason = box_with_text(&doc, "Plain reason");
    let mut h = harness_with(doc);
    h.run();
    for key in [
        "essay.help.title",
        "essay.help",
        "essay.intro",
        "essay.conclusion",
        "essay.paragraph_for",
        "essay.section_with",
        "essay.unplaced",
        "essay.inactive",
        "essay.copy",
        "essay.remove",
    ] {
        assert_ne!(t(key), key, "missing ftl entry");
    }
    let s = |n: usize, premises: &str| {
        let mut args = FluentArgs::new();
        args.set("n", n);
        args.set("premises", premises);
        t_args("essay.section_with", &args)
    };
    assert_eq!(
        headings(&h),
        [
            t("essay.intro"),
            s(1, "Strong objection"),
            s(2, "Plain reason"),
            s(3, "Weak reason"),
            s(4, "Nil reason"),
            t("essay.conclusion"),
        ]
    );
    // The help note explains the structure.
    h.get_by_label(&t("essay.help.title"));

    // Clicking a statement selects it on the map and reveals it.
    h.state_mut().tabs[0].camera.offset = egui::vec2(-20_000.0, -20_000.0);
    let before = h.state().tabs[0].camera.offset;
    h.get_by_role_and_label(Role::ListItem, "Plain reason")
        .click();
    h.run();
    let tab = &h.state().tabs[0];
    assert_eq!(tab.selection, Some(Item::Box(reason)));
    assert_ne!(tab.camera.offset, before, "the canvas revealed the box");

    // The order is cached per revision, like the outline.
    let rev = h.state().tabs[0].revision();
    assert_eq!(h.state().tabs[0].essay_cache.key().copied(), Some(rev));
}

#[test]
fn essay_typing_is_one_history_entry() {
    let (doc, _) = rated_doc();
    let reason = box_with_text(&doc, "Plain reason");
    let mut h = harness_with(doc);
    h.run();
    let label = paragraph_label("Plain reason");
    h.get_by_label(&label).click();
    h.run();
    h.get_by_label(&label).type_text("Cars are loud.");
    h.run();
    let slot = EssaySlot::Item(Item::Box(reason));
    assert_eq!(
        h.state().tabs[0].doc.essay.text(slot),
        Some("Cars are loud.")
    );
    assert_eq!(undo_len(&h), 0, "typing is live");
    assert_eq!(
        h.state().tabs[0].session(),
        Some(&EditSession::Essay { slot })
    );
    assert!(h.state().tabs[0].title().ends_with('•'), "unsaved typing");

    h.key_press(Key::Escape);
    h.run();
    assert!(!h.ctx.text_edit_focused());
    assert_eq!(undo_len(&h), 1, "the whole typing is one step");
    h.state_mut().pending.push(Action::Undo);
    h.run();
    assert_eq!(h.state().tabs[0].doc.essay.text(slot), None);

    // The introduction is a slot too.
    h.get_by_label(&t("essay.intro.field")).click();
    h.run();
    h.get_by_label(&t("essay.intro.field")).type_text("Intro");
    h.run();
    assert_eq!(
        h.state().tabs[0].doc.essay.text(EssaySlot::Intro),
        Some("Intro")
    );
}

#[test]
fn essay_session_is_narrowed_to_its_slot() {
    let mut tab = DocTab::from_document(Document::new("Claim"));
    tab.set_essay_text_live(EssaySlot::Intro, "Hello");
    assert!(tab.has_pending_session());
    // Back to the text it had: nothing to record, though the modified
    // time moved.
    tab.set_essay_text_live(EssaySlot::Intro, "");
    assert!(!tab.has_pending_session());
    assert!(!tab.commit_session());
    assert!(tab.history.undo.is_empty());
    // Card slots are not writable: no session starts.
    let mut d = Document::new("Claim");
    let (c, _) = d
        .add_card(Parent::Box(d.root), CardKind::Reason, "R")
        .unwrap();
    let mut tab = DocTab::from_document(d);
    tab.set_essay_text_live(EssaySlot::Item(Item::Card(c)), "x");
    assert_eq!(tab.session(), None);
}

fn run(app: &mut ReasonCraftApp, actions: impl IntoIterator<Item = Action>) {
    for a in actions {
        apply(app, a);
        // Saves are written on a worker: the next step sees them done.
        reasoncraft::file_ops::wait_for_saves(app);
    }
}

#[test]
fn essay_session_commits_on_save_and_close() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("essay.rcraft");
    let fake = FakeDialogs::default();
    fake.set_save(path.clone());
    let mut app = ReasonCraftApp::new_for_test_with(Box::new(fake.clone()));
    let edit = |text: &str| Action::EditEssayText {
        slot: EssaySlot::Conclusion,
        text: text.to_owned(),
    };
    run(&mut app, [edit("So"), edit("So cars"), edit("So cars go")]);
    assert!(app.active_tab().history.undo.is_empty());
    assert!(app.active_tab().is_dirty());

    // Saving commits the session as one step and writes the text.
    run(&mut app, [Action::Save]);
    assert!(app.modal.is_none(), "save failed: {:?}", app.modal);
    assert_eq!(app.active_tab().history.undo.len(), 1);
    assert_eq!(app.active_tab().session(), None);
    assert!(!app.active_tab().is_dirty());
    let saved = file::open_package(&path).unwrap();
    assert_eq!(
        saved.doc.essay.text(EssaySlot::Conclusion),
        Some("So cars go")
    );

    // Closing commits a running session and asks to save it.
    run(&mut app, [edit("So cars go away")]);
    run(&mut app, [Action::CloseTab(0)]);
    assert_eq!(app.active_tab().history.undo.len(), 2);
    assert!(matches!(
        app.modal,
        Some(Modal::Unsaved {
            tab,
            window: false,
            ..
        }) if tab == app.tabs[0].recovery_key
    ));
    run(&mut app, [Action::ResolveUnsaved(UnsavedChoice::Save)]);
    let saved = file::open_package(&path).unwrap();
    assert_eq!(
        saved.doc.essay.text(EssaySlot::Conclusion),
        Some("So cars go away")
    );
}

/// `key` with `name` as its `$name`.
fn named(key: &str, name: &str) -> String {
    let mut args = FluentArgs::new();
    args.set("name", name);
    t_args(key, &args)
}

#[test]
fn deleting_branch_shows_unplaced_note() {
    let (mut doc, cards) = rated_doc();
    let reason = box_with_text(&doc, "Weak reason");
    doc.set_essay_text(EssaySlot::Item(Item::Box(reason)), "Written for it")
        .unwrap();
    let mut h = harness_with(doc);
    h.run();
    assert!(h.query_by_label(&t("essay.unplaced")).is_none());

    h.state_mut().pending.extend([
        Action::Select(Some(Item::Card(cards[0]))),
        Action::DeleteBranch,
    ]);
    h.run();
    let doc = &h.state().tabs[0].doc;
    assert_eq!(
        doc.essay.unplaced,
        [reasoncraft_core::UnplacedNote::new(
            "Weak reason".to_owned(),
            "Written for it".to_owned()
        )]
    );
    h.get_by_label(&t("essay.unplaced"));
    h.get_by_label("Written for it");
    // The buttons name the note they act on.
    h.get_by_label(&named("essay.copy_name", "Weak reason"));

    // Remove deletes the note as one undoable step.
    let steps = undo_len(&h);
    let remove = named("essay.remove_name", "Weak reason");
    h.get_by_label(&remove).scroll_to_me();
    h.run();
    h.get_by_label(&remove).click();
    h.run();
    assert!(h.state().tabs[0].doc.essay.unplaced.is_empty());
    assert_eq!(undo_len(&h), steps + 1);
    assert!(h.query_by_label(&t("essay.unplaced")).is_none());

    // Undo brings the note back, and undo again the branch with its text.
    h.state_mut().pending.push(Action::Undo);
    h.run();
    h.get_by_label("Written for it");
    h.state_mut().pending.push(Action::Undo);
    h.run();
    let doc = &h.state().tabs[0].doc;
    assert!(doc.essay.unplaced.is_empty());
    assert_eq!(
        doc.essay.text(EssaySlot::Item(Item::Box(reason))),
        Some("Written for it")
    );
}

#[test]
fn parked_paragraph_is_listed_as_not_in_the_essay() {
    let (mut doc, cards) = rated_doc();
    let reason = box_with_text(&doc, "Plain reason");
    doc.set_essay_text(EssaySlot::Item(Item::Box(reason)), "Kept aside")
        .unwrap();
    let mut h = harness_with(doc);
    h.run();
    assert!(h.query_by_label(&t("essay.inactive")).is_none());

    // Park the branch: its text is kept, shown apart from the sections.
    h.state_mut().pending.push(Action::Move {
        item: Item::Card(cards[2]),
        to: Parent::Parking,
        index: None,
    });
    h.run();
    assert!(h.state().tabs[0].doc.essay.unplaced.is_empty());
    h.get_by_label(&t("essay.inactive"));
    h.get_by_label("Kept aside");
    assert!(h.query_by_label(&paragraph_label("Plain reason")).is_none());

    // Back under the claim, it is a slot again.
    let root = h.state().tabs[0].doc.root;
    h.state_mut().pending.push(Action::Move {
        item: Item::Card(cards[2]),
        to: Parent::Box(root),
        index: None,
    });
    h.run();
    assert!(h.query_by_label(&t("essay.inactive")).is_none());
    h.get_by_label(&paragraph_label("Plain reason"));
}

#[test]
fn deleting_branch_during_typing_keeps_the_typed_text() {
    let (doc, cards) = rated_doc();
    let reason = box_with_text(&doc, "Weak reason");
    let slot = EssaySlot::Item(Item::Box(reason));
    let mut h = harness_with(doc);
    h.run();
    let label = paragraph_label("Weak reason");
    h.get_by_label(&label).click();
    h.run();
    h.get_by_label(&label).type_text("Typed just now");
    h.run();
    assert!(h.state().tabs[0].has_pending_session(), "session is open");

    h.state_mut().pending.extend([
        Action::Select(Some(Item::Card(cards[0]))),
        Action::DeleteBranch,
    ]);
    h.run();
    let doc = &h.state().tabs[0].doc;
    assert_eq!(
        doc.essay.unplaced,
        [reasoncraft_core::UnplacedNote::new(
            "Weak reason".to_owned(),
            "Typed just now".to_owned()
        )]
    );
    h.get_by_label("Typed just now");

    // The first undo restores the branch with its paragraph in the slot.
    h.state_mut().pending.push(Action::Undo);
    h.run();
    let doc = &h.state().tabs[0].doc;
    assert!(doc.essay.unplaced.is_empty());
    assert_eq!(doc.essay.text(slot), Some("Typed just now"));
    // The second undo takes back the typing itself.
    h.state_mut().pending.push(Action::Undo);
    h.run();
    assert_eq!(h.state().tabs[0].doc.essay.text(slot), None);
    assert!(h.state().tabs[0].doc.essay.unplaced.is_empty());
}
