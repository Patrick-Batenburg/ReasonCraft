use std::path::PathBuf;

use super::*;
use crate::actions::{Action, apply};
use crate::file_ops::FakeDialogs;
use crate::i18n::t;
use reasoncraft_core::ImportNoteKind;

fn app_with_import(name: &str, text: &str) -> (ReasonCraftApp, tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(name);
    std::fs::write(&path, text).unwrap();
    let fake = FakeDialogs::default();
    let app = ReasonCraftApp::new_for_test_with(Box::new(fake.clone()));
    fake.set_import(path.clone());
    (app, dir, path)
}

#[test]
fn import_opens_new_dirty_tab_with_report() {
    let text = "[A]: Coffee is good.\n    >< [B]: Coffee is bad.\n";
    let (mut app, _dir, path) = app_with_import("coffee.argdown", text);
    let before = app.tabs.len();
    apply(&mut app, Action::ImportArgdown);
    assert_eq!(app.tabs.len(), before + 1);
    assert_eq!(app.active, app.tabs.len() - 1);
    let tab = app.active_tab();
    assert_eq!(tab.path, None, "only Save As names the file");
    assert!(tab.is_dirty());
    assert_eq!(tab.doc.meta.title, "coffee");
    assert_eq!(tab.doc.boxes[&tab.doc.root].text, "Coffee is good.");
    let Some(Modal::ImportReport(notes)) = &app.modal else {
        panic!("{:?}", app.modal)
    };
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].kind, ImportNoteKind::Contradiction);
    let body = report_text(notes);
    assert!(body.contains('2') && !body.contains("argdown."), "{body}");
    // The source file is never written.
    assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
}

#[test]
fn clean_import_shows_no_report_and_keeps_frontmatter_title() {
    let text = "===\ntitle: \"Coffee\"\n===\n[A]: Coffee is good.\n";
    let (mut app, _dir, _) = app_with_import("c.argdown", text);
    apply(&mut app, Action::ImportArgdown);
    assert_eq!(app.modal, None);
    assert_eq!(app.active_tab().doc.meta.title, "Coffee");
}

#[test]
fn failed_or_cancelled_import_changes_no_tabs() {
    let (mut app, _dir, _) = app_with_import("empty.argdown", "// nothing here\n");
    apply(&mut app, Action::ImportArgdown);
    assert_eq!(app.tabs.len(), 1);
    let Some(Modal::Error { title, message }) = &app.modal else {
        panic!("{:?}", app.modal)
    };
    assert!(title.contains("empty.argdown"), "{title}");
    assert!(!message.starts_with("argdown."), "{message}");
    apply(&mut app, Action::DismissModal);
    // Nothing queued: the dialog counts as cancelled.
    apply(&mut app, Action::ImportArgdown);
    assert_eq!((app.tabs.len(), &app.modal), (1, &None));
}

#[test]
fn every_note_and_error_is_translated() {
    use reasoncraft_core::ArgdownError as E;
    let kinds = [
        ImportNoteKind::Contradiction,
        ImportNoteKind::ExtraEquivalentWording,
        ImportNoteKind::SharedStatementCopied,
        ImportNoteKind::CycleBroken,
        ImportNoteKind::UnknownReference("T".into()),
        ImportNoteKind::IntermediateConclusion,
        ImportNoteKind::InferenceRuleDropped,
        ImportNoteKind::TagsDropped,
        ImportNoteKind::UndercutOnStatement,
        ImportNoteKind::EmptyArgument,
        ImportNoteKind::MovedToParking,
        ImportNoteKind::RelationDropped,
        ImportNoteKind::LinkDropped,
    ];
    for kind in kinds {
        let text = note_text(&kind);
        assert!(!text.starts_with("argdown."), "{text}");
    }
    let unknown = note_text(&ImportNoteKind::UnknownReference("Some title".into()));
    assert!(unknown.contains("Some title"), "{unknown}");
    for err in [
        E::TooLarge,
        E::TooManyBoxes,
        E::TooDeep { line: 7 },
        E::Empty,
    ] {
        let text = ImportError::Argdown(err).message();
        assert!(!text.starts_with("argdown."), "{text}");
    }
    assert_ne!(t("argdown.report.title"), "argdown.report.title");
}

#[test]
fn long_reports_are_grouped_and_capped() {
    let note = |line, kind| ImportNote { line, kind };
    let mut notes: Vec<ImportNote> = (1..=12)
        .map(|l| note(l, ImportNoteKind::Contradiction))
        .collect();
    notes.push(note(20, ImportNoteKind::UnknownReference("A".into())));
    notes.push(note(21, ImportNoteKind::UnknownReference("B".into())));
    let body = report_text(&notes);
    // One line per kind, after the introduction.
    assert_eq!(body.lines().count(), 3, "{body}");
    assert!(
        body.contains("1, 2, 3, 4, 5") && body.contains('7'),
        "{body}"
    );
    assert!(!body.contains(", 6"), "{body}");
    assert!(body.contains("20, 21"), "{body}");
}
