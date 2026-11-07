//! Messages that end in the background wait for an open dialog (APP-3):
//! an export or image error never shows over the link editor, and
//! closing a message leaves the link editor alone.

use std::time::{Duration, Instant};

use reasoncraft::ReasonCraftApp;
use reasoncraft::actions::{Action, apply, poll_exports};
use reasoncraft::export::ExportFormat;
use reasoncraft::file_ops::FakeDialogs;

fn app() -> (ReasonCraftApp, FakeDialogs) {
    let fake = FakeDialogs::default();
    (
        ReasonCraftApp::new_for_test_with(Box::new(fake.clone())),
        fake,
    )
}

/// Polls the exports until none is running.
fn finish_exports(app: &mut ReasonCraftApp) {
    let start = Instant::now();
    while app.export.busy() {
        poll_exports(app);
        assert!(
            start.elapsed() < Duration::from_secs(60),
            "the export hangs"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    poll_exports(app);
}

#[test]
fn an_export_error_waits_for_the_link_editor() {
    let dir = tempfile::tempdir().unwrap();
    let (mut a, fake) = app();
    // A folder that does not exist: the export fails.
    fake.set_export(dir.path().join("missing").join("map.argdown"));
    apply(&mut a, Action::OpenExport(ExportFormat::Argdown));
    assert!(a.export.busy());
    apply(&mut a, Action::EditLink);
    a.link_edit.as_mut().unwrap().text = "https://example.org/typed-by-hand".into();

    finish_exports(&mut a);
    assert!(a.modal.is_none(), "the error showed over the link editor");
    assert_eq!(
        a.link_edit.as_ref().map(|e| e.text.as_str()),
        Some("https://example.org/typed-by-hand")
    );

    // Once the editor closes, the error shows.
    apply(&mut a, Action::CloseLinkEdit);
    poll_exports(&mut a);
    assert!(
        matches!(a.modal, Some(reasoncraft::file_ops::Modal::Error { .. })),
        "{:?}",
        a.modal
    );
}

#[test]
fn closing_a_message_keeps_the_link_editor() {
    let (mut a, _) = app();
    apply(&mut a, Action::EditLink);
    a.link_edit.as_mut().unwrap().text = "https://example.org".into();
    a.modal = Some(reasoncraft::file_ops::Modal::Error {
        title: "t".into(),
        message: "m".into(),
    });
    apply(&mut a, Action::DismissModal);
    assert!(a.modal.is_none());
    assert!(
        a.link_edit.is_some(),
        "OK on a message closed the link editor"
    );
    apply(&mut a, Action::CloseLinkEdit);
    assert!(a.link_edit.is_none());
}

#[test]
fn an_image_error_does_not_show_over_the_link_editor() {
    use reasoncraft::doc_tab::DocTab;
    use reasoncraft_core::{CardKind, Document, Item, Parent};

    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("holiday.png");
    std::fs::write(&file, b"\x89PNG\r\n\x1a\n this is not really a picture").unwrap();
    let (mut a, fake) = app();
    let mut doc = Document::new("Claim");
    let root = doc.root;
    let (_, premise) = (doc.add_card(Parent::Box(root), CardKind::Reason, "Premise")).unwrap();
    a.tabs[0] = DocTab::from_document(doc);
    a.tabs[0].selection = Some(Item::Box(premise));
    fake.set_image(file);
    apply(&mut a, Action::InsertImage);
    apply(&mut a, Action::EditLink);
    assert!(a.link_edit.is_some());

    reasoncraft::actions::wait_for_images(&mut a);
    assert!(a.modal.is_none(), "the error showed over the link editor");
    assert!(a.link_edit.is_some());
    let note = a.status_note.clone().unwrap_or_default();
    assert!(note.contains("holiday.png"), "{note:?}");
}
