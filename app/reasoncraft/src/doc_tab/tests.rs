use super::*;
use reasoncraft_core::{CardKind, Parent};

#[test]
fn title_marks_dirty_and_uses_file_stem() {
    let mut tab = DocTab::new();
    assert_eq!(tab.title(), "Untitled");
    let root = tab.doc.root;
    tab.edit(|d| d.add_card(Parent::Box(root), CardKind::Reason, ""))
        .unwrap();
    assert_eq!(tab.title(), "Untitled •");
    tab.path = Some(PathBuf::from("maps/robots.rcraft"));
    tab.history.mark_saved();
    assert_eq!(tab.title(), "robots");
}

#[test]
fn untitled_tab_is_named_by_its_document_title() {
    let mut tab = DocTab::new();
    tab.doc.meta.title = "  Robots at school ".to_owned();
    assert_eq!(tab.name(), "Robots at school");
    // A file suggestion leaves out what file names cannot hold.
    tab.doc.meta.title = "Why? A/B: \"tests\"".to_owned();
    assert_eq!(tab.file_stem(), "Why A B tests");
    tab.doc.meta.title = "???".to_owned();
    assert_eq!(tab.file_stem(), "Untitled");
    // A file's name wins.
    tab.path = Some(PathBuf::from("maps/robots.rcraft"));
    assert_eq!(tab.name(), "robots");
}

#[test]
fn outline_is_rebuilt_only_for_new_revision_or_options() {
    let mut tab = DocTab::new();
    let opts = OutlineOptions::default();
    assert_eq!(tab.outline(opts).len(), 1);
    // A change made behind the cache's back is not seen…
    let root = tab.doc.root;
    tab.doc
        .add_card(Parent::Box(root), CardKind::Reason, "")
        .unwrap();
    assert_eq!(tab.outline(opts).len(), 1);
    // …until the revision moves on, or the options change.
    tab.bump_revision();
    assert_eq!(tab.outline(opts).len(), 2);
    tab.doc
        .add_card(Parent::Box(root), CardKind::Reason, "")
        .unwrap();
    let bare = OutlineOptions {
        numbering: false,
        ..opts
    };
    assert_eq!(tab.outline(bare).len(), 3);
}

#[test]
fn issues_are_found_again_only_for_new_revision_or_checks() {
    use reasoncraft_core::checks::CheckKind;
    let mut tab = DocTab::new();
    let all = CheckSet::default();
    assert!(tab.issues(&all).is_empty());
    // A change made behind the cache's back is not seen…
    tab.doc.add_loose(reasoncraft_core::NewLoose::Category, "n");
    assert!(tab.issues(&all).is_empty());
    // …until the revision moves on, or the checks change.
    tab.bump_revision();
    assert_eq!(tab.issues(&all).len(), 1);
    let none = CheckSet {
        disabled: vec![CheckKind::NotConnected],
    };
    assert!(tab.issues(&none).is_empty());
    assert_eq!(tab.issues(&all).len(), 1);
}

#[test]
fn refused_edit_records_no_history() {
    let mut tab = DocTab::new();
    let out = tab.edit(|d| d.remove_item(Item::Box(d.root), reasoncraft_core::RemoveMode::Branch));
    assert!(out.is_none());
    assert!(tab.history.undo.is_empty());
    assert!(!tab.is_dirty());
}

#[test]
fn undo_dropping_inline_edit_clears_edit_focus() {
    let mut tab = DocTab::new();
    let root = tab.doc.root;
    let (_, b) = tab
        .edit(|d| d.add_card(Parent::Box(root), CardKind::Reason, ""))
        .unwrap();
    tab.begin_edit(b, true);
    assert!(tab.edit_focus);
    tab.undo();
    assert_eq!(tab.editing(), None);
    assert!(!tab.edit_focus);
}

/// A refused edit (a rule, nothing to change, a target that is gone)
/// changes nothing and records no step.
#[test]
fn refused_edits_record_nothing() {
    let mut tab = DocTab::from_document(Document::new("C"));
    for e in [EditError::NotAllowed, EditError::NotFound] {
        assert_eq!(tab.edit(|_| Err::<(), _>(e)), None);
    }
    assert!(tab.history.undo.is_empty());
}

/// An error no command should cause stops a debug build, so the missing
/// check is found instead of the command silently doing nothing.
#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "unexpected edit error")]
fn unexpected_edit_errors_stop_debug_builds() {
    let mut tab = DocTab::from_document(Document::new("C"));
    tab.edit(|_| Err::<(), _>(EditError::Cycle));
}

#[test]
fn issues_wait_for_typing_to_pause() {
    use std::time::{Duration, Instant};
    let mut tab = DocTab::new();
    let root = tab.doc.root;
    let all = CheckSet::default();
    let count = |tab: &mut DocTab, now| {
        let (found, due) = tab.settled_issues(&all, now);
        (found.len(), due.is_some())
    };
    assert_eq!(count(&mut tab, Instant::now()), (0, false));
    tab.begin_edit(root, false);
    tab.set_edit_text(root, "Robots?".into());
    // Typing just now: the last issues are kept, and fresh ones are due.
    assert_eq!(count(&mut tab, Instant::now()), (0, true));
    // After a pause the checks run again.
    let later = Instant::now() + ISSUE_SETTLE + Duration::from_millis(1);
    assert_eq!(count(&mut tab, later), (1, false));
    // Ending the session shows fresh issues at once.
    tab.set_edit_text(root, "Robots? Jobs and pay?".into());
    assert_eq!(count(&mut tab, Instant::now()), (1, true));
    tab.commit_session();
    assert_eq!(count(&mut tab, Instant::now()), (2, false));
}
