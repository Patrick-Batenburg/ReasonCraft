use reasoncraft_core::checks::CheckKind;
use reasoncraft_core::{Document, NewLoose};

use crate::actions::{Action, apply};
use crate::app::ReasonCraftApp;
use crate::doc_tab::DocTab;
use crate::export::Notice;
use crate::settings::OutlineToggle;

/// An app whose only tab has a parked note in "Move to…", with a status
/// note and an export notice showing.
fn connecting_app() -> ReasonCraftApp {
    let mut d = Document::new("Claim");
    let note = d.add_loose(NewLoose::Note, "n");
    let mut a = ReasonCraftApp::new_for_test();
    a.tabs[0] = DocTab::from_document(d);
    a.tabs[0].selection = Some(note);
    apply(&mut a, Action::StartConnect);
    assert_eq!(a.tabs[0].connect_mode, Some(note));
    a.status_note = Some("A note".into());
    a.export.notice = Some(Notice::Saved(std::env::temp_dir().join("map.png")));
    a
}

/// Personal settings change no document: like the view preferences, they
/// leave the notes and "Move to…" alone (they used to drift apart).
#[test]
fn settings_keep_notes_and_connect() {
    for action in [
        Action::SetOutlineOption(OutlineToggle::Numbering, false),
        Action::SetCheckEnabled(CheckKind::Question, false),
        Action::SetDefaultDocLanguage(Some("nl".into())),
        Action::SetSpellcheck(false),
        Action::SetAutosaveSecs(60),
        Action::SetAuthor("Ann".into()),
    ] {
        let mut a = connecting_app();
        apply(&mut a, action.clone());
        assert!(a.tabs[0].connect_mode.is_some(), "{action:?} ends Connect");
        assert_eq!(a.status_note.as_deref(), Some("A note"), "{action:?}");
        assert!(a.export.notice.is_some(), "{action:?} clears the notice");
    }
}

/// A document edit still clears both.
#[test]
fn edits_clear_notes_and_end_connect() {
    let mut a = connecting_app();
    apply(&mut a, Action::AddNote);
    assert!(a.tabs[0].connect_mode.is_none());
    assert!(a.status_note.is_none());
    assert!(a.export.notice.is_none());
}
