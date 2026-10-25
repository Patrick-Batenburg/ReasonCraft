use reasoncraft_core::fixtures::FixedMeasure;
use reasoncraft_core::{
    BoxKind, CardKind, Document, Item, LayoutParams, NodeId, Parent, SnippetSource, SourceType,
    layout,
};

use crate::actions::{Action, Insertable, apply, is_enabled};
use crate::app::ReasonCraftApp;
use crate::doc_tab::DocTab;

fn app_with(d: Document) -> ReasonCraftApp {
    let mut a = ReasonCraftApp::new_for_test();
    a.tabs[0] = DocTab::from_document(d);
    a
}

fn tab(app: &ReasonCraftApp) -> &DocTab {
    &app.tabs[app.active]
}

fn source() -> SnippetSource {
    SnippetSource {
        source_type: SourceType::Web,
        citation: "Smith (2020)".into(),
        link: Some("https://example.org/robots".into()),
    }
}

fn text(text: &str, source: Option<SnippetSource>) -> Insertable {
    Insertable::Text {
        text: text.into(),
        source,
    }
}

fn drop_on(what: Insertable, to: Parent) -> Action {
    Action::Insert {
        what,
        to: Some(to),
        on: None,
    }
}

/// A claim with a reason (premise `p`) under it.
fn claim_with_reason() -> (Document, NodeId) {
    let mut d = Document::new("Claim");
    let (_, p) = d
        .add_card(Parent::Box(d.root), CardKind::Reason, "p")
        .unwrap();
    (d, p)
}

/// The premise of the only reason card under `parent`'s children.
fn new_premise(d: &Document, children: &[Item]) -> NodeId {
    let [Item::Card(c)] = children else {
        panic!("one new card, got {children:?}")
    };
    assert_eq!(d.cards[c].kind, CardKind::Reason);
    let [p] = d.cards[c].premises[..] else {
        panic!("one premise")
    };
    p
}

/// The premise carries the text and, under it, a source box with the
/// snippet's type, citation and link.
fn assert_text_with_source(d: &Document, premise: NodeId, text: &str) {
    let node = &d.boxes[&premise];
    assert_eq!(node.text, text);
    let [Item::Box(s)] = node.children[..] else {
        panic!("one source under the premise")
    };
    let s = &d.boxes[&s];
    assert_eq!(
        s.kind,
        BoxKind::Source {
            source_type: SourceType::Web
        }
    );
    assert_eq!(s.text, "Smith (2020)");
    assert_eq!(s.link.as_deref(), Some("https://example.org/robots"));
}

#[test]
fn text_drop_on_premise_adds_reason_with_source_in_one_step() {
    let (d, p) = claim_with_reason();
    let before = d.clone();
    let mut a = app_with(d);
    let action = drop_on(
        text("  Robots save time \n", Some(source())),
        Parent::Box(p),
    );
    assert!(is_enabled(tab(&a), &action));
    apply(&mut a, action);
    let t = tab(&a);
    let premise = new_premise(&t.doc, &t.doc.boxes[&p].children);
    assert_text_with_source(&t.doc, premise, "Robots save time");
    assert_eq!(t.history.undo.len(), 1, "card and source: one step");
    assert_eq!(t.selection, Some(Item::Box(premise)));
    assert_eq!(t.editing, None, "the text is already there");

    apply(&mut a, Action::Undo);
    let t = tab(&a);
    assert_eq!(t.doc.boxes.len(), before.boxes.len(), "undo removes both");
    assert!(t.doc.boxes[&p].children.is_empty());
}

#[test]
fn text_drop_on_claim_card_and_empty_canvas() {
    let (d, p) = claim_with_reason();
    let root = d.root;
    let card = *d.cards.keys().next().unwrap();
    let mut a = app_with(d);
    apply(
        &mut a,
        drop_on(text("on claim", Some(source())), Parent::Box(root)),
    );
    apply(
        &mut a,
        drop_on(text("whole card", None), Parent::Card(card)),
    );
    apply(
        &mut a,
        drop_on(text("loose", Some(source())), Parent::Parking),
    );
    let t = tab(&a);
    let on_claim = t.doc.boxes[&root].children[1];
    let premise = new_premise(&t.doc, &[on_claim]);
    assert_text_with_source(&t.doc, premise, "on claim");
    let kids: Vec<Item> = t.doc.cards[&card]
        .children
        .iter()
        .map(|&k| Item::Card(k))
        .collect();
    let whole = new_premise(&t.doc, &kids);
    assert_eq!(t.doc.boxes[&whole].text, "whole card");
    assert!(t.doc.boxes[&whole].children.is_empty(), "no source given");
    let loose = new_premise(&t.doc, &t.doc.parking);
    assert_text_with_source(&t.doc, loose, "loose");
    assert_eq!(t.history.undo.len(), 3);
    assert!(t.doc.boxes[&p].children.is_empty());
}

/// Categories take no sources, so the source is left off.
#[test]
fn text_drop_on_category_adds_category() {
    let mut d = Document::new("Claim");
    let cat = d
        .add_box(Parent::Box(d.root), BoxKind::Category, "Costs")
        .unwrap();
    let boxes = d.boxes.len();
    let mut a = app_with(d);
    apply(
        &mut a,
        drop_on(text("Energy", Some(source())), Parent::Box(cat)),
    );
    let t = tab(&a);
    let [Item::Box(new)] = t.doc.boxes[&cat].children[..] else {
        panic!("one new child")
    };
    assert_eq!(t.doc.boxes[&new].kind, BoxKind::Category);
    assert_eq!(t.doc.boxes[&new].text, "Energy");
    assert!(t.doc.boxes[&new].children.is_empty());
    assert_eq!(t.doc.boxes.len(), boxes + 1, "no source box");
    assert_eq!(t.history.undo.len(), 1);
    assert_eq!(t.selection, Some(Item::Box(new)));
}

#[test]
fn blank_selection_drop_refused() {
    let (mut d, p) = claim_with_reason();
    let note = d.add_box(Parent::Box(p), BoxKind::Note, "n").unwrap();
    let src = d
        .add_box(
            Parent::Box(p),
            BoxKind::Source {
                source_type: SourceType::Law,
            },
            "b",
        )
        .unwrap();
    let before = d.clone();
    let mut a = app_with(d);
    apply(&mut a, Action::Select(Some(Item::Box(note))));
    let refused = [
        drop_on(text(" \n\t ", Some(source())), Parent::Box(p)),
        drop_on(text("", None), Parent::Parking),
        drop_on(text("text", None), Parent::Box(note)),
        drop_on(text("text", Some(source())), Parent::Box(src)),
        // Text needs a parent, not a box to go on.
        Action::Insert {
            what: text("text", None),
            to: None,
            on: Some(p),
        },
        Action::Insert {
            what: text("text", None),
            to: None,
            on: None,
        },
    ];
    for action in refused {
        assert!(!is_enabled(tab(&a), &action), "{action:?}");
        apply(&mut a, action);
    }
    let t = tab(&a);
    assert_eq!(t.doc.boxes.len(), before.boxes.len());
    assert_eq!(t.history.undo.len(), 0);
    assert_eq!(t.selection, Some(Item::Box(note)), "selection kept");
}

#[test]
fn long_selection_drop_keeps_text() {
    let (d, p) = claim_with_reason();
    let long = "Robots save time. ".repeat(167);
    let long = long.trim();
    assert!(long.chars().count() >= 3_000);
    let mut a = app_with(d);
    apply(&mut a, drop_on(text(long, None), Parent::Box(p)));
    let t = tab(&a);
    let premise = new_premise(&t.doc, &t.doc.boxes[&p].children);
    assert_eq!(t.doc.boxes[&premise].text, long);
    let l = layout(&t.doc, &FixedMeasure, &LayoutParams::default());
    assert!(
        l.boxes[&premise].height() > 10.0 * l.boxes[&p].height(),
        "the box grows to show all the text"
    );
}

#[test]
fn unsafe_snippet_link_is_dropped_from_the_new_source() {
    let (d, p) = claim_with_reason();
    let mut a = app_with(d);
    let mut src = source();
    src.link = Some("javascript:alert(1)".into());
    apply(&mut a, drop_on(text("t", Some(src)), Parent::Box(p)));
    let t = tab(&a);
    let premise = new_premise(&t.doc, &t.doc.boxes[&p].children);
    let [Item::Box(s)] = t.doc.boxes[&premise].children[..] else {
        panic!("one source")
    };
    assert_eq!(t.doc.boxes[&s].text, "Smith (2020)");
    assert_eq!(t.doc.boxes[&s].link, None, "unsafe link not copied");
}
