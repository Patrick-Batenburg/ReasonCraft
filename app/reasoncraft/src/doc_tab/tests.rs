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
