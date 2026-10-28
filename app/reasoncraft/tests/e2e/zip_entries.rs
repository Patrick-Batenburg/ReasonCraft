//! Unknown zip entries from newer builds survive open, edit and save as.

use super::*;

/// Review focus 5 in the app: a `.rcraft` from a newer build with an extra
/// zip entry is opened, edited and saved as a new file through the app's
/// own commands, and the entry is still there.
#[test]
fn e2e_unknown_zip_entry_survives_open_edit_save_as() {
    let dir = tempfile::tempdir().unwrap();
    let newer = dir.path().join("newer.rcraft");
    let thumb = vec![0x89, b'P', b'N', b'G', 0, 1, 2, 3];
    let mut pkg = reasoncraft_core::Package::new(reasoncraft_core::Document::new("A claim"));
    pkg.other_entries
        .push(("thumbnail.png".to_owned(), thumb.clone().into()));
    file::save_package(&pkg, &newer).unwrap();

    let fake = FakeDialogs::default();
    let mut app = ReasonCraftApp::new_for_test_with(Box::new(fake.clone()));
    fake.set_open(newer.clone());
    run(&mut app, [Action::Open]);
    assert!(app.modal.is_none(), "open failed: {:?}", app.modal);
    let root = app.active_tab().doc.root;
    run(
        &mut app,
        [
            Action::Select(Some(Item::Box(root))),
            Action::AddReason,
            typed("A reason"),
        ],
    );
    let copy = dir.path().join("copy.rcraft");
    fake.set_save(copy.clone());
    run(&mut app, [Action::SaveAs]);
    assert!(app.modal.is_none(), "save failed: {:?}", app.modal);

    let saved = file::open_package(&copy).unwrap();
    assert_eq!(saved.doc.boxes.len(), 2, "the edit was saved");
    assert_eq!(
        saved.other_entries,
        vec![("thumbnail.png".to_owned(), thumb.into())]
    );
    // The tab still holds the entry for a later save.
    assert_eq!(app.active_tab().other_entries.len(), 1);
}
