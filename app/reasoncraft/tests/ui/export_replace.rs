//! UI tests for export safety: no silent replacing of an existing file
//! when the extension was added, errors that wait for an open message,
//! and the dialog's scope without a selection.

use crate::common;
use std::path::Path;

use egui_kittest::kittest::Queryable;
use reasoncraft::ReasonCraftApp;
use reasoncraft::actions::Action;
use reasoncraft::doc_tab::DocTab;
use reasoncraft::export::{ExportFormat, ExportOptions, ExportRequest, ExportScope, PageSetup};
use reasoncraft::file_ops::{FakeDialogs, Modal};
use reasoncraft::i18n::t;
use reasoncraft_core::{CardKind, Document, Parent};

use crate::common::H;

fn harness(fake: &FakeDialogs, dir: &Path) -> H {
    let mut doc = Document::new("Claim");
    let root = doc.root;
    doc.add_card(Parent::Box(root), CardKind::Reason, "Alpha")
        .unwrap();
    let mut app = ReasonCraftApp::new_for_test_with(Box::new(fake.clone()));
    app.tabs[0] = DocTab::from_document(doc);
    app.export.print_dir = dir.to_owned();
    common::harness(app)
}

fn wait(h: &mut H) {
    common::settle(h);
}

fn svg_request() -> Action {
    Action::RunExport(ExportRequest {
        format: ExportFormat::Svg,
        options: ExportOptions::default(),
        page: PageSetup::default(),
    })
}

/// The names of the files in `dir`.
fn files(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn export_asks_before_replacing_a_file_it_added_the_extension_to() {
    let dir = tempfile::tempdir().unwrap();
    let existing = dir.path().join("map.svg");
    std::fs::write(&existing, b"keep me").unwrap();
    let fake = FakeDialogs::default();
    let mut h = harness(&fake, dir.path());

    // The dialog checked "map", not "map.svg": ask first. Cancel keeps it.
    fake.set_export(dir.path().join("map"));
    h.state_mut().push(svg_request());
    h.run();
    assert!(matches!(
        h.state().modal,
        Some(Modal::ConfirmExportReplace { .. })
    ));
    assert!(!h.state().export.busy(), "nothing runs before the answer");
    h.get_by_label(&t("replace.cancel")).click();
    h.run();
    wait(&mut h);
    assert!(h.state().modal.is_none());
    assert_eq!(std::fs::read(&existing).unwrap(), b"keep me");

    // Replace writes it.
    fake.set_export(dir.path().join("map"));
    h.state_mut().push(svg_request());
    h.run();
    h.get_by_label(&t("replace.confirm")).click();
    h.run_steps(2);
    wait(&mut h);
    assert!(h.state().modal.is_none());
    let svg = std::fs::read_to_string(&existing).unwrap();
    assert!(svg.contains("<svg"), "{svg}");
    // Written through a temporary file that is gone again.
    assert_eq!(files(dir.path()), ["map.svg"]);
}

#[test]
fn a_name_the_dialog_checked_is_replaced_without_asking() {
    let dir = tempfile::tempdir().unwrap();
    let existing = dir.path().join("map.svg");
    std::fs::write(&existing, b"old").unwrap();
    let fake = FakeDialogs::default();
    let mut h = harness(&fake, dir.path());
    fake.set_export(existing.clone());
    h.state_mut().push(svg_request());
    h.run_steps(2);
    wait(&mut h);
    assert!(h.state().modal.is_none());
    assert_ne!(std::fs::read(&existing).unwrap(), b"old");
}

#[test]
fn export_error_waits_for_the_open_message() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDialogs::default();
    let mut h = harness(&fake, dir.path());
    fake.set_export(dir.path().join("missing").join("map.svg"));
    // The message shows before the export starts, however fast it fails.
    let first = Modal::Error {
        title: "First".into(),
        message: "Already showing".into(),
    };
    h.state_mut().modal = Some(first.clone());
    h.state_mut().push(svg_request());
    h.run_steps(2);
    wait(&mut h);
    assert_eq!(h.state().modal, Some(first), "not overwritten");
    h.get_by_label(&t("modal.ok")).click();
    h.run();
    match &h.state().modal {
        Some(Modal::Error { title, .. }) => assert_eq!(title, &t("export.error.title")),
        other => panic!("expected the export error next, got {other:?}"),
    }
}

#[test]
fn dialog_scope_is_whole_map_without_a_selection() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDialogs::default();
    let mut h = harness(&fake, dir.path());
    h.state_mut().settings.export.scope = ExportScope::Selection;
    h.state_mut().tabs[0].selection = None;
    h.state_mut().push(Action::OpenExport(ExportFormat::Png));
    h.run();
    let dialog = h.state().export_dialog.clone().unwrap();
    assert_eq!(dialog.options.scope, ExportScope::Map);
}
