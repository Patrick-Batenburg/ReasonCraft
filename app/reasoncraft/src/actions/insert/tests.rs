use reasoncraft_core::{BoxKind, CardKind, Document, Item, NodeId, Parent, SourceType, Verdict};

use super::Insertable;
use crate::actions::{Action, apply, is_enabled};
use crate::app::ReasonCraftApp;
use crate::canvas::comments::PRESETS;
use crate::doc_tab::DocTab;
use crate::i18n::t;

fn app_with(d: Document) -> ReasonCraftApp {
    let mut a = ReasonCraftApp::new_for_test();
    a.tabs[0] = DocTab::from_document(d);
    a
}

fn tab(app: &ReasonCraftApp) -> &DocTab {
    &app.tabs[app.active]
}

/// The document's structure without ids: every item from the root, then
/// the parking area, with kinds, texts and comments.
fn shape(d: &Document) -> String {
    fn item(d: &Document, i: Item, out: &mut String) {
        match i {
            Item::Box(b) => {
                let n = &d.boxes[&b];
                out.push_str(&format!("({:?} {:?}", n.kind, n.text));
                for c in &n.comments {
                    out.push_str(&format!(" #{:?}/{:?}/{:?}", c.author, c.text, c.verdict));
                }
                for &ch in &n.children {
                    item(d, ch, out);
                }
            }
            Item::Card(c) => {
                let k = &d.cards[&c];
                out.push_str(&format!("[{:?}", k.kind));
                for &p in &k.premises {
                    item(d, Item::Box(p), out);
                }
                for &ch in &k.children {
                    item(d, Item::Card(ch), out);
                }
            }
        }
        out.push(')');
    }
    let mut out = String::new();
    item(d, Item::Box(d.root), &mut out);
    out.push_str(" parked:");
    for &p in &d.parking {
        item(d, p, &mut out);
    }
    out
}

/// A claim with a reason (premise `p`) and a category-free root.
fn claim_with_reason() -> (Document, NodeId) {
    let mut d = Document::new("Claim");
    let (_, p) = d
        .add_card(Parent::Box(d.root), CardKind::Reason, "p")
        .unwrap();
    (d, p)
}

/// Inserting at the selection gives the same document, selection state
/// and history as the ribbon command.
#[test]
fn insert_at_selection_matches_ribbon() {
    let (d, p) = claim_with_reason();
    let cases = [
        (Insertable::Reason, Action::AddReason),
        (Insertable::Objection, Action::AddObjection),
        (Insertable::CoPremise, Action::AddCoPremise),
        (Insertable::Category, Action::AddCategory),
        (Insertable::Note, Action::AddNote),
        (
            Insertable::Source(SourceType::Law),
            Action::AddSource(SourceType::Law),
        ),
    ];
    for select in [Some(Item::Box(d.root)), Some(Item::Box(p)), None] {
        for (what, ribbon) in cases.clone() {
            let run = |action: Action| {
                let mut a = app_with(d.clone());
                apply(&mut a, Action::Select(select));
                apply(&mut a, action);
                apply(&mut a, Action::StopEditing);
                let t = tab(&a);
                (shape(&t.doc), t.history.undo.len(), t.selection.is_some())
            };
            let insert = Action::Insert {
                what: what.clone(),
                to: None,
                on: None,
            };
            assert_eq!(
                is_enabled(tab(&app_with(d.clone())), &insert),
                is_enabled(tab(&app_with(d.clone())), &ribbon),
            );
            assert_eq!(run(insert), run(ribbon), "{what:?} at {select:?}");
        }
    }
}

#[test]
fn comment_preset_at_selection_and_on_node() {
    let (d, p) = claim_with_reason();
    let mut a = app_with(d);
    let at_selection = Action::Insert {
        what: Insertable::Comment(8),
        to: None,
        on: None,
    };
    apply(&mut a, Action::Select(None));
    assert!(!is_enabled(tab(&a), &at_selection), "no box selected");
    apply(&mut a, at_selection.clone());
    assert_eq!(tab(&a).history.undo.len(), 0);

    apply(&mut a, Action::Select(Some(Item::Box(p))));
    assert!(is_enabled(tab(&a), &at_selection));
    apply(&mut a, at_selection);
    let root = tab(&a).doc.root;
    apply(
        &mut a,
        Action::Insert {
            what: Insertable::Comment(0),
            to: None,
            on: Some(root),
        },
    );
    let tb = tab(&a);
    let c = &tb.doc.boxes[&p].comments;
    assert_eq!(c.len(), 1);
    assert_eq!(c[0].text, t(PRESETS[8].0));
    assert_eq!(c[0].verdict, Some(Verdict::Problem));
    assert_eq!(c[0].author, "");
    let c = &tb.doc.boxes[&root].comments;
    assert_eq!(c[0].verdict, Some(Verdict::Positive));
    assert_eq!(tb.selection, Some(Item::Box(root)));
    assert_eq!(tb.comments_open, None, "a preset drop opens no popover");
    assert_eq!(tb.history.undo.len(), 2);
}

#[test]
fn insert_at_target_and_refusals() {
    let (mut d, p) = claim_with_reason();
    let note = d.add_box(Parent::Box(p), BoxKind::Note, "n").unwrap();
    let card = *d.cards.keys().next().unwrap();
    let mut a = app_with(d);
    let before = shape(&tab(&a).doc);
    apply(&mut a, Action::Select(Some(Item::Box(note))));
    let refused = [
        (
            Insertable::Source(SourceType::Web),
            Some(Parent::Card(card)),
            None,
        ),
        (Insertable::Reason, Some(Parent::Box(note)), None),
        (Insertable::CoPremise, None, Some(tab(&a).doc.root)),
        (Insertable::Comment(0), Some(Parent::Parking), None),
        (Insertable::Comment(99), None, Some(p)),
        (Insertable::Note, Some(Parent::Box(p)), Some(p)),
    ];
    for (what, to, on) in refused {
        let action = Action::Insert { what, to, on };
        assert!(!is_enabled(tab(&a), &action), "{action:?}");
        apply(&mut a, action);
    }
    let t = tab(&a);
    assert_eq!(shape(&t.doc), before);
    assert_eq!(t.history.undo.len(), 0);
    assert_eq!(t.selection, Some(Item::Box(note)), "selection kept");

    // A whole-card objection, a co-premise and a parked source.
    let drops = [
        (Insertable::Objection, Some(Parent::Card(card)), None),
        (Insertable::CoPremise, None, Some(p)),
        (
            Insertable::Source(SourceType::Web),
            Some(Parent::Parking),
            None,
        ),
    ];
    for (what, to, on) in drops {
        apply(&mut a, Action::Insert { what, to, on });
    }
    let t = tab(&a);
    assert_eq!(t.doc.cards[&card].children.len(), 1);
    assert_eq!(t.doc.cards[&card].premises.len(), 2);
    let [Item::Box(s)] = t.doc.parking[..] else {
        panic!("one parked source")
    };
    assert_eq!(
        t.doc.boxes[&s].kind,
        BoxKind::Source {
            source_type: SourceType::Web
        }
    );
    assert_eq!(t.selection, Some(Item::Box(s)));
    assert_eq!(t.editing(), Some(s), "a new box is edited");
    assert_eq!(t.history.undo.len(), 3);
}

#[test]
fn targeted_insert_elsewhere_closes_the_comment_popover() {
    let (d, p) = claim_with_reason();
    let root = d.root;
    let mut a = app_with(d);
    apply(&mut a, Action::OpenComments(root));
    let on = |node| Action::Insert {
        what: Insertable::Comment(0),
        to: None,
        on: Some(node),
    };
    apply(&mut a, on(root));
    assert_eq!(tab(&a).comments_open, Some(root), "same box: stays open");
    apply(&mut a, on(p));
    assert_eq!(tab(&a).comments_open, None);
    assert_eq!(tab(&a).doc.boxes[&p].comments.len(), 1);
}
