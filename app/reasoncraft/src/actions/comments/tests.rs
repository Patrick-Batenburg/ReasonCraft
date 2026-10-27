use reasoncraft_core::{Item, NodeId, Verdict};

use crate::actions::{Action, apply, is_enabled};
use crate::app::ReasonCraftApp;
use crate::doc_tab::DocTab;

fn tab(app: &ReasonCraftApp) -> &DocTab {
    &app.tabs[app.active]
}

fn app_and_root() -> (ReasonCraftApp, NodeId) {
    let a = ReasonCraftApp::new_for_test();
    let root = tab(&a).doc.root;
    (a, root)
}

fn add(text: &str, node: NodeId, verdict: Option<Verdict>) -> Action {
    Action::AddComment {
        node,
        text: text.to_owned(),
        verdict,
    }
}

#[test]
fn add_comment_uses_author_or_stores_none() {
    let (mut a, root) = app_and_root();
    apply(
        &mut a,
        add("Needs a source", root, Some(Verdict::NeedsWork)),
    );
    let c = &tab(&a).doc.boxes[&root].comments[0];
    // No author name: stored empty, shown translated (see the popover).
    assert_eq!(c.author, "");
    assert_eq!(c.text, "Needs a source");
    assert_eq!(c.verdict, Some(Verdict::NeedsWork));
    assert_eq!(tab(&a).history.undo.len(), 1);

    apply(&mut a, Action::SetAuthor("  Ann Smith ".to_owned()));
    assert_eq!(a.settings.author, "Ann Smith");
    apply(&mut a, add("Good", root, None));
    let c = &tab(&a).doc.boxes[&root].comments[1];
    assert_eq!(c.author, "Ann Smith");
    assert_eq!(c.verdict, None);

    // A blank author counts as none.
    apply(&mut a, Action::SetAuthor("   ".to_owned()));
    assert_eq!(a.settings.author, "");
    apply(&mut a, add("Third", root, None));
    assert_eq!(tab(&a).doc.boxes[&root].comments[2].author, "");
}

#[test]
fn blank_comment_is_refused() {
    let (mut a, root) = app_and_root();
    let blank = add(" \n\t ", root, Some(Verdict::Positive));
    assert!(!is_enabled(tab(&a), &blank));
    apply(&mut a, blank);
    assert!(tab(&a).doc.boxes[&root].comments.is_empty());
    assert!(tab(&a).history.undo.is_empty());
    assert!(is_enabled(tab(&a), &add("x", root, None)));
    // A box that does not exist takes no comment either.
    let ghost = add("x", NodeId::new(), None);
    assert!(!is_enabled(tab(&a), &ghost));
    apply(&mut a, ghost);
    assert!(tab(&a).history.undo.is_empty());
}

#[test]
fn remove_comment_records_history() {
    let (mut a, root) = app_and_root();
    apply(&mut a, add("One", root, None));
    apply(&mut a, add("Two", root, Some(Verdict::Problem)));
    let first = tab(&a).doc.boxes[&root].comments[0].clone();
    let steps = tab(&a).history.undo.len();
    apply(
        &mut a,
        Action::RemoveComment {
            node: root,
            comment: first.id,
        },
    );
    assert_eq!(tab(&a).doc.boxes[&root].comments.len(), 1);
    assert_eq!(tab(&a).history.undo.len(), steps + 1);
    apply(&mut a, Action::Undo);
    let comments = &tab(&a).doc.boxes[&root].comments;
    assert_eq!(comments.len(), 2);
    assert_eq!(comments[0], first);
    // Removing it again after a redo, then a missing one: no extra history.
    apply(&mut a, Action::Redo);
    let steps = tab(&a).history.undo.len();
    apply(
        &mut a,
        Action::RemoveComment {
            node: root,
            comment: first.id,
        },
    );
    assert_eq!(tab(&a).history.undo.len(), steps);
}

#[test]
fn open_and_close_comments() {
    let (mut a, root) = app_and_root();
    apply(&mut a, Action::Select(None));
    assert!(!is_enabled(tab(&a), &Action::CommentSelected));
    apply(&mut a, Action::OpenComments(root));
    assert_eq!(tab(&a).comments_open, Some(root));
    assert_eq!(tab(&a).selection, Some(Item::Box(root)));
    apply(&mut a, Action::CloseComments);
    assert_eq!(tab(&a).comments_open, None);
    assert!(tab(&a).history.undo.is_empty());

    // The ribbon button and Ctrl+Shift+M open them for the selected box.
    assert!(is_enabled(tab(&a), &Action::CommentSelected));
    apply(&mut a, Action::CommentSelected);
    assert_eq!(tab(&a).comments_open, Some(root));

    // A box that goes away closes its comments.
    apply(&mut a, Action::AddReason);
    apply(&mut a, Action::StopEditing);
    let Some(Item::Box(premise)) = tab(&a).selection else {
        panic!("premise selected")
    };
    apply(&mut a, Action::OpenComments(premise));
    apply(&mut a, Action::Undo);
    assert_eq!(tab(&a).comments_open, None);
    // A missing box does not open.
    apply(&mut a, Action::OpenComments(NodeId::new()));
    assert_eq!(tab(&a).comments_open, None);
}
