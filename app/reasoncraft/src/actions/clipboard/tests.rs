use std::sync::{Arc, Mutex};

use reasoncraft_core::{BoxKind, CardKind, Document, ImageRef, Item, NodeId, Parent};

use crate::actions::{Action, apply, can_run, is_enabled};
use crate::app::ReasonCraftApp;
use crate::doc_tab::DocTab;
use crate::export::FakeClipboard;

/// root ← card A [a1]; a1 ← a note. Returns the app, card A and a1.
fn app() -> (ReasonCraftApp, reasoncraft_core::CardId, NodeId) {
    let mut d = Document::new("Claim");
    let (ca, a1) = d
        .add_card(Parent::Box(d.root), CardKind::Reason, "a1")
        .unwrap();
    d.add_box(Parent::Box(a1), BoxKind::Note, "note").unwrap();
    let mut a = ReasonCraftApp::new_for_test();
    a.tabs[0] = DocTab::from_document(d);
    (a, ca, a1)
}

fn tab(a: &ReasonCraftApp) -> &DocTab {
    &a.tabs[a.active]
}

fn select(a: &mut ReasonCraftApp, item: Item) {
    apply(a, Action::Select(Some(item)));
}

/// The system clipboard as other programs left it.
fn system_text(a: &mut ReasonCraftApp, text: &str) {
    let fake = FakeClipboard::default();
    *fake.text.lock().unwrap() = Some(text.to_owned());
    a.export.clipboard = Arc::new(Mutex::new(fake));
}

#[test]
fn copy_and_paste_a_card_under_the_root() {
    let (mut a, ca, _) = app();
    let root = tab(&a).doc.root;
    select(&mut a, Item::Card(ca));
    apply(&mut a, Action::Copy);
    assert!(tab(&a).history.undo.is_empty(), "copy changes nothing");
    select(&mut a, Item::Box(root));
    assert!(can_run(&a, &Action::Paste(None)));
    apply(&mut a, Action::Paste(None));
    let t = tab(&a);
    assert_eq!(t.doc.boxes[&root].children.len(), 2);
    assert_eq!(t.doc.cards.len(), 2);
    assert_eq!(t.doc.boxes.len(), 5, "claim, two premises, two notes");
    assert_eq!(t.history.undo.len(), 1, "one step");
    let pasted = *t.doc.boxes[&root].children.last().unwrap();
    assert_eq!(t.selection, Some(pasted));
    assert_ne!(pasted, Item::Card(ca));
}

#[test]
fn cut_deletes_the_branch_and_paste_brings_it_back() {
    let (mut a, ca, a1) = app();
    select(&mut a, Item::Card(ca));
    apply(&mut a, Action::Cut);
    assert!(tab(&a).doc.cards.is_empty());
    assert!(!tab(&a).doc.boxes.contains_key(&a1));
    let root = tab(&a).doc.root;
    select(&mut a, Item::Box(root));
    apply(&mut a, Action::Paste(None));
    assert_eq!(tab(&a).doc.cards.len(), 1);
    assert_eq!(tab(&a).doc.boxes.len(), 3);
    assert_eq!(tab(&a).history.undo.len(), 2, "the cut and the paste");
}

#[test]
fn copy_and_cut_need_a_selection_other_than_the_root() {
    let (mut a, _, a1) = app();
    let root = tab(&a).doc.root;
    for sel in [None, Some(Item::Box(root))] {
        a.tabs[0].selection = sel;
        assert!(!is_enabled(tab(&a), &Action::Copy), "{sel:?}");
        assert!(!is_enabled(tab(&a), &Action::Cut), "{sel:?}");
    }
    select(&mut a, Item::Box(a1));
    assert!(is_enabled(tab(&a), &Action::Copy));
    assert!(is_enabled(tab(&a), &Action::Cut));
}

/// A note takes nothing: neither the copied card nor text can go there.
#[test]
fn paste_is_off_where_nothing_fits() {
    let (mut a, ca, a1) = app();
    select(&mut a, Item::Card(ca));
    apply(&mut a, Action::Copy);
    let Item::Box(note) = tab(&a).doc.boxes[&a1].children[0] else {
        panic!("the note")
    };
    select(&mut a, Item::Box(note));
    assert!(!can_run(&a, &Action::Paste(None)));
    apply(&mut a, Action::Paste(None));
    assert_eq!(tab(&a).doc.cards.len(), 1);
}

/// A premise pastes as the premise of a new card in another tab, with its
/// image: the bytes come along to that tab.
#[test]
fn paste_into_another_tab_brings_the_images() {
    let (mut a, _, a1) = app();
    let key = a.tabs[0].assets.insert("png", vec![1, 2, 3]);
    a.tabs[0]
        .edit(|d| d.set_image(a1, Some(ImageRef::new(key.clone(), 1.0))))
        .unwrap();
    select(&mut a, Item::Box(a1));
    apply(&mut a, Action::Copy);
    apply(&mut a, Action::New);
    assert_eq!(a.active, 1);
    apply(&mut a, Action::Select(None));
    apply(&mut a, Action::Paste(None));
    let t = tab(&a);
    assert_eq!(t.doc.parking.len(), 1);
    let Item::Card(c) = t.doc.parking[0] else {
        panic!("a card")
    };
    let premise = &t.doc.boxes[&t.doc.cards[&c].premises[0]];
    assert_eq!(premise.text, "a1");
    assert_eq!(premise.image.as_ref().unwrap().asset, key);
    assert!(t.assets.get(&key).is_some());
}

/// Text another program put on the clipboard pastes as a new box; the
/// text Copy put there pastes the copied item.
#[test]
fn foreign_text_pastes_as_a_box() {
    let (mut a, ca, a1) = app();
    select(&mut a, Item::Card(ca));
    apply(&mut a, Action::Copy);
    let ours = a.clipboard.as_ref().unwrap().text.clone();
    assert_eq!(ours, "a1\n  note\n");

    select(&mut a, Item::Box(a1));
    apply(&mut a, Action::Paste(Some("Robots work.".into())));
    let t = tab(&a);
    let texts: Vec<_> = t.doc.boxes.values().map(|b| b.text.as_str()).collect();
    assert!(texts.contains(&"Robots work."), "{texts:?}");
    assert_eq!(t.doc.cards.len(), 2);

    select(&mut a, Item::Box(a1));
    apply(&mut a, Action::Paste(Some(ours.replace('\n', "\r\n"))));
    assert_eq!(tab(&a).doc.cards.len(), 3);
    assert_eq!(tab(&a).doc.boxes.len(), 6, "the copied card, note and all");

    // From the ribbon the clipboard is read: other text wins there too.
    system_text(&mut a, "From elsewhere");
    select(&mut a, Item::Box(a1));
    apply(&mut a, Action::Paste(None));
    let t = tab(&a);
    assert!(t.doc.boxes.values().any(|b| b.text == "From elsewhere"));
}

/// An item without text still puts text on the system clipboard: egui
/// sends no paste for an empty clipboard, so Ctrl+V would do nothing.
/// Pasting that text brings the item back.
#[test]
fn an_empty_item_copies_marker_text_and_pastes_back() {
    let (mut a, _, a1) = app();
    let note = tab(&a).doc.boxes[&a1].children[0];
    let Item::Box(note_id) = note else {
        panic!("a note")
    };
    a.tabs[0].doc.boxes[&note_id].text.clear();
    select(&mut a, note);
    apply(&mut a, Action::Copy);
    let text = a.clipboard.as_ref().unwrap().text.clone();
    assert!(!text.trim().is_empty(), "{text:?}");
    let before = tab(&a).doc.boxes.len();
    select(&mut a, Item::Box(a1));
    apply(&mut a, Action::Paste(Some(text)));
    let t = tab(&a);
    assert_eq!(t.doc.boxes.len(), before + 1);
    let pasted = t.doc.boxes[&a1].children.last().unwrap();
    assert!(matches!(pasted, Item::Box(b) if t.doc.boxes[b].text.is_empty()));
}
