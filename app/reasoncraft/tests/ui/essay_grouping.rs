//! A grouping map has no essay (help: Outline and essay, Grouping maps):
//! the Essay tab says so instead of offering an introduction and a
//! conclusion, and Export > Essay is greyed out.

use crate::common::{app_with_doc, build};
use egui_kittest::kittest::Queryable;
use reasoncraft::actions::{Action, can_run};
use reasoncraft::export::ExportFormat;
use reasoncraft::i18n::t;
use reasoncraft::settings::TextTab;
use reasoncraft_core::{BoxKind, Document, Parent};

const ESSAYS: [ExportFormat; 3] = [
    ExportFormat::EssayMarkdown,
    ExportFormat::EssayText,
    ExportFormat::EssayDocx,
];

fn grouping_doc() -> Document {
    let mut d = Document::new("Ways to travel");
    let root = Parent::Box(d.root);
    d.add_box(root, BoxKind::Category, "By train").unwrap();
    d
}

#[test]
fn grouping_map_has_no_essay() {
    let mut doc = grouping_doc();
    // Text kept from when the map had reasons stays reachable.
    doc.essay
        .unplaced
        .push(("An old reason".into(), "Its paragraph".into()));
    let mut app = app_with_doc(doc);
    app.settings.text_tab = TextTab::Essay;
    for format in ESSAYS {
        assert!(!can_run(&app, &Action::OpenExport(format)), "{format:?}");
    }
    assert!(can_run(
        &app,
        &Action::OpenExport(ExportFormat::OutlineMarkdown)
    ));
    let mut h = build(app);
    h.run();
    h.get_by_label(&t("essay.grouping"));
    assert!(h.query_by_label(&t("essay.intro.field")).is_none());
    assert!(h.query_by_label(&t("essay.conclusion.field")).is_none());
    assert!(h.query_by_label(&t("essay.help.title")).is_none());
    h.get_by_label(&t("essay.unplaced"));
}

#[test]
fn a_map_with_only_a_claim_keeps_its_essay() {
    let mut app = app_with_doc(Document::new("Cities should ban cars"));
    app.settings.text_tab = TextTab::Essay;
    for format in ESSAYS {
        assert!(can_run(&app, &Action::OpenExport(format)), "{format:?}");
    }
    let mut h = build(app);
    h.run();
    assert!(h.query_by_label(&t("essay.grouping")).is_none());
    h.get_by_label(&t("essay.intro.field"));
}
