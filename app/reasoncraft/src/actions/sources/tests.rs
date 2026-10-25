use reasoncraft_core::{BoxKind, CardKind, Document, Item, Parent, SourceType};

use crate::actions::{Action, apply, is_enabled};
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

fn kind_of(t: &DocTab, item: Option<Item>) -> BoxKind {
    let Some(Item::Box(b)) = item else {
        panic!("no box selected")
    };
    t.doc.boxes[&b].kind.clone()
}

#[test]
fn add_source_targets_selection() {
    let mut d = Document::new("Claim");
    let (_, premise) = d
        .add_card(Parent::Box(d.root), CardKind::Reason, "p")
        .unwrap();
    let mut a = app_with(d);
    let add = Action::AddSource(SourceType::ExpertOpinion);

    // A premise: the source goes under it, selected and in edit mode.
    apply(&mut a, Action::Select(Some(Item::Box(premise))));
    assert!(is_enabled(tab(&a), &add));
    apply(&mut a, add.clone());
    let t = tab(&a);
    let Some(Item::Box(s)) = t.selection else {
        panic!("source not selected")
    };
    assert_eq!(t.doc.boxes[&premise].children, vec![Item::Box(s)]);
    assert_eq!(
        kind_of(t, t.selection),
        BoxKind::Source {
            source_type: SourceType::ExpertOpinion
        }
    );
    assert_eq!(t.editing(), Some(s));
    assert_eq!(t.history.undo.len(), 1);

    // The claim takes sources too.
    let root = t.doc.root;
    apply(&mut a, Action::Select(Some(Item::Box(root))));
    apply(&mut a, Action::AddSource(SourceType::Law));
    assert_eq!(tab(&a).doc.boxes[&root].children.len(), 2);

    // Nothing selected: a loose source in the parking area.
    apply(&mut a, Action::Select(None));
    apply(&mut a, Action::AddSource(SourceType::Web));
    let t = tab(&a);
    assert_eq!(t.doc.parking, vec![t.selection.unwrap()]);
    assert_eq!(
        kind_of(t, t.selection),
        BoxKind::Source {
            source_type: SourceType::Web
        }
    );

    // A category, a card, a source or a note takes no source.
    let mut d = Document::new("Claim");
    let cat = d
        .add_box(Parent::Box(d.root), BoxKind::Category, "c")
        .unwrap();
    let mut a = app_with(d);
    apply(&mut a, Action::Select(Some(Item::Box(cat))));
    assert!(!is_enabled(tab(&a), &add));
    apply(&mut a, add.clone());
    assert_eq!(tab(&a).history.undo.len(), 0);
    assert!(tab(&a).doc.boxes[&cat].children.is_empty());
}

#[test]
fn set_source_type_changes_type() {
    let mut d = Document::new("Claim");
    let s = d
        .add_box(
            Parent::Box(d.root),
            BoxKind::Source {
                source_type: SourceType::Source,
            },
            "s",
        )
        .unwrap();
    let mut a = app_with(d);
    let set = Action::SetSourceType(SourceType::Statistic);

    // Only a selected source can change type.
    assert!(!is_enabled(tab(&a), &set), "the claim is selected");
    apply(&mut a, set.clone());
    assert_eq!(tab(&a).history.undo.len(), 0);

    apply(&mut a, Action::Select(Some(Item::Box(s))));
    assert!(is_enabled(tab(&a), &set));
    apply(&mut a, set.clone());
    let t = tab(&a);
    assert_eq!(
        t.doc.boxes[&s].kind,
        BoxKind::Source {
            source_type: SourceType::Statistic
        }
    );
    assert_eq!(t.history.undo.len(), 1);

    // The same type again changes nothing and records nothing.
    apply(&mut a, set);
    assert_eq!(tab(&a).history.undo.len(), 1);
    apply(&mut a, Action::Undo);
    assert_eq!(
        tab(&a).doc.boxes[&s].kind,
        BoxKind::Source {
            source_type: SourceType::Source
        }
    );
}
