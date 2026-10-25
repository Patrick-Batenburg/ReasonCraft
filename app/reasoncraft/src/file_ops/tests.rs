use super::*;
use reasoncraft_core::{CardKind, Parent};

#[test]
fn rcraft_extension_is_forced() {
    let ext = |p: &str| with_rcraft_extension(PathBuf::from(p));
    assert_eq!(ext("maps/a"), PathBuf::from("maps/a.rcraft"));
    assert_eq!(ext("maps/a.rcraft"), PathBuf::from("maps/a.rcraft"));
    assert_eq!(ext("maps/a.RCraft"), PathBuf::from("maps/a.RCraft"));
    assert_eq!(ext("maps/a.txt"), PathBuf::from("maps/a.txt.rcraft"));
    assert_eq!(ext("my.map"), PathBuf::from("my.map.rcraft"));
}

#[test]
fn error_keys_per_variant() {
    let io = || FileError::Io(std::io::Error::other("disk full"));
    let newer = || FileError::NewerVersion {
        found: 7,
        supported: 1,
    };
    let malformed = || FileError::Malformed("bad json".into());
    use FileOp::{Open, Save};
    assert_eq!(error_key(Open, &io()), "error.open.io");
    assert_eq!(
        error_key(Open, &FileError::NotAReasonCraftFile),
        "error.open.not_a_file"
    );
    assert_eq!(
        error_key(Open, &FileError::MissingDocument),
        "error.open.missing_document"
    );
    assert_eq!(error_key(Open, &malformed()), "error.open.malformed");
    assert_eq!(error_key(Open, &newer()), "error.open.newer_version");
    assert_eq!(error_key(Save, &io()), "error.save.io");
    assert_eq!(error_key(Save, &malformed()), "error.save.other");

    let text = error_text(Open, &newer());
    assert!(text.contains('7') && text.contains('1'), "{text}");
    assert!(!text.starts_with("error."), "{text}");
    for (op, err) in [
        (Open, io()),
        (Open, FileError::NotAReasonCraftFile),
        (Open, FileError::MissingDocument),
        (Open, malformed()),
        (Save, io()),
        (Save, malformed()),
    ] {
        let text = error_text(op, &err);
        assert!(!text.starts_with("error."), "missing ftl entry: {text}");
    }
}

fn dirty_tab() -> DocTab {
    let mut tab = DocTab::new();
    let root = tab.doc.root;
    tab.edit(|d| d.add_card(Parent::Box(root), CardKind::Reason, "x"))
        .unwrap();
    tab
}

#[test]
fn failed_save_keeps_path_and_dirty() {
    let dir = tempfile::tempdir().unwrap();
    let old = dir.path().join("old.rcraft");
    let mut tab = dirty_tab();
    tab.path = Some(old.clone());
    let fake = FakeDialogs::default();
    fake.set_save(dir.path().join("no/such/dir/new.rcraft"));
    assert!(save_tab(&mut tab, &fake, true).is_err());
    assert_eq!(tab.path, Some(old));
    assert!(tab.is_dirty());
}

#[test]
fn cancelled_save_dialog_changes_nothing() {
    let mut tab = dirty_tab();
    let fake = FakeDialogs::default();
    assert!(save_tab(&mut tab, &fake, false).is_ok());
    assert_eq!(tab.path, None);
    assert!(tab.is_dirty());
}

#[test]
fn save_uses_known_path_without_dialog() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("known.rcraft");
    let mut tab = dirty_tab();
    tab.path = Some(path.clone());
    // No dialog answer queued: a dialog would cancel the save.
    save_tab(&mut tab, &FakeDialogs::default(), false).unwrap();
    assert!(path.is_file());
    assert!(!tab.is_dirty());
}

#[test]
fn open_reuses_untouched_tab_else_adds() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.rcraft");
    let b = dir.path().join("b.rcraft");
    reasoncraft_core::file::save(&reasoncraft_core::Document::new("A"), &a).unwrap();
    reasoncraft_core::file::save(&reasoncraft_core::Document::new("B"), &b).unwrap();

    let mut app = ReasonCraftApp::new_for_test();
    open_into_app(&mut app, &a);
    assert_eq!(app.tabs.len(), 1, "the untouched new tab is reused");
    assert_eq!(app.tabs[0].path.as_deref(), Some(a.as_path()));
    open_into_app(&mut app, &b);
    assert_eq!(app.tabs.len(), 2);
    assert_eq!(app.active, 1);
    // Already open (by another spelling of the path): activate it.
    open_into_app(&mut app, &dir.path().join(".").join("a.rcraft"));
    assert_eq!(app.tabs.len(), 2);
    assert_eq!(app.active, 0);
    assert!(app.modal.is_none());

    open_into_app(&mut app, &dir.path().join("missing.rcraft"));
    assert_eq!(app.tabs.len(), 2);
    assert!(matches!(app.modal, Some(Modal::Error { .. })));
}

#[test]
fn command_line_skips_non_files_and_reports_bad_ones() {
    let dir = tempfile::tempdir().unwrap();
    let good = dir.path().join("good.rcraft");
    let bad = dir.path().join("bad.rcraft");
    reasoncraft_core::file::save(&reasoncraft_core::Document::new("G"), &good).unwrap();
    std::fs::write(&bad, "junk").unwrap();

    let mut app = ReasonCraftApp::new_for_test();
    open_paths(&mut app, [dir.path().join("nope.rcraft").into_os_string()]);
    assert!(app.modal.is_none(), "missing files are skipped");
    open_paths(&mut app, [bad.into_os_string(), good.into_os_string()]);
    assert_eq!(app.tabs.len(), 1);
    assert_eq!(app.tabs[0].doc.boxes[&app.tabs[0].doc.root].text, "G");
    match &app.modal {
        Some(Modal::Error { title, .. }) => assert!(title.contains("bad.rcraft"), "{title}"),
        other => panic!("expected an error modal, got {other:?}"),
    }
}
