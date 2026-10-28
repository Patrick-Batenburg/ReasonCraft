//! The map, fakes and helpers the export UI tests share.

use std::path::Path;
use std::sync::{Arc, Mutex};

use eframe::egui::accesskit::Role;
use egui_kittest::kittest::Queryable;
use reasoncraft::ReasonCraftApp;
use reasoncraft::actions::{Action, FakeOpener};
use reasoncraft::doc_tab::DocTab;
use reasoncraft::export::{ExportFormat, ExportOptions, ExportRequest, FakeClipboard, PageSetup};
use reasoncraft::file_ops::FakeDialogs;
use reasoncraft::i18n::t;
use reasoncraft_core::{CardKind, Document, NodeId, Parent};

use crate::common::{self, H};

/// What a test steers the app with.
pub struct Fakes {
    pub dialogs: FakeDialogs,
    pub opener: FakeOpener,
    pub clipboard: FakeClipboard,
}

/// "Claim" with reasons "Alpha" (with "Deep" under it) and "Beta"; returns
/// the harness and Alpha's box.
pub fn harness(dir: &Path) -> (H, Fakes, NodeId) {
    let mut doc = Document::new("Claim");
    let root = doc.root;
    let (alpha_card, alpha) = doc
        .add_card(Parent::Box(root), CardKind::Reason, "Alpha")
        .unwrap();
    let _ = alpha_card;
    doc.add_card(Parent::Box(alpha), CardKind::Reason, "Deep")
        .unwrap();
    doc.add_card(Parent::Box(root), CardKind::Objection, "Beta")
        .unwrap();
    let fakes = Fakes {
        dialogs: FakeDialogs::default(),
        opener: FakeOpener::default(),
        clipboard: FakeClipboard::default(),
    };
    let mut app = ReasonCraftApp::new_for_test_with(Box::new(fakes.dialogs.clone()));
    app.tabs[0] = DocTab::from_document(doc);
    app.link_opener = Box::new(fakes.opener.clone());
    app.export.clipboard = Arc::new(Mutex::new(fakes.clipboard.clone()));
    app.export.print_dir = dir.to_owned();
    app.left_panel = false;
    let h = common::harness(app);
    (h, fakes, alpha)
}

/// Steps frames until no export (or other background work) is running.
pub fn wait(h: &mut H) {
    common::settle(h);
}

pub fn request(format: ExportFormat, options: ExportOptions) -> Action {
    Action::RunExport(ExportRequest {
        format,
        options,
        page: PageSetup::default(),
    })
}

/// Clicks the button labelled with `key`'s text (the Text panel's tabs
/// and the ribbon's tabs share some names) and steps a few frames (the
/// busy spinner keeps repainting, so `run` would not settle).
pub fn click_button(h: &mut H, key: &str) {
    let label = t(key);
    h.get_by_role_and_label(Role::Button, &label).click();
    h.run_steps(4);
}

pub fn open_export_tab(h: &mut H) {
    h.get_by_role_and_label(Role::Tab, &t("ribbon.tab.export"))
        .click();
    h.run();
}
