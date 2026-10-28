//! UI test: File → Import Argdown… opens the file in a new tab and shows
//! what the import changed.

use crate::common::harness;
use egui_kittest::kittest::Queryable;
use reasoncraft::ReasonCraftApp;
use reasoncraft::file_ops::{FakeDialogs, Modal};
use reasoncraft::i18n::t;

#[test]
fn backstage_imports_argdown_with_report() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tea.argdown");
    let text = "[Tea]: Tea is good for you. #health\n    <+ [Calm]: Tea calms the mind.\n";
    std::fs::write(&path, text).unwrap();
    let fake = FakeDialogs::default();
    let app = ReasonCraftApp::new_for_test_with(Box::new(fake.clone()));
    let mut h = harness(app);

    fake.set_import(path);
    h.get_by_label(&t("ribbon.tab.file")).click();
    h.run();
    h.get_by_label(&t("action.import_argdown")).click();
    h.run();
    let app = h.state();
    assert_eq!(app.tabs.len(), 2);
    let tab = app.active_tab();
    assert!(tab.is_dirty() && tab.path.is_none());
    assert_eq!(tab.doc.boxes[&tab.doc.root].text, "Tea is good for you.");
    assert!(matches!(&app.modal, Some(Modal::ImportReport(n)) if n.len() == 1));

    // The report names the line and what was left out; OK closes it.
    h.get_by_label(&t("argdown.report.title"));
    h.get_by_label(&t("modal.ok")).click();
    h.run();
    assert_eq!(h.state().modal, None);
}
