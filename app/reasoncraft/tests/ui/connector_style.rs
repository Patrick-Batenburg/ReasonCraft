//! UI tests for the connector style: the View ribbon's "Connectors" menu
//! sets the personal setting (no history), and exports draw it.

use std::time::{Duration, Instant};

use eframe::egui;
use egui_kittest::{Harness, kittest::Queryable};
use reasoncraft::ReasonCraftApp;
use reasoncraft::actions::Action;
use reasoncraft::doc_tab::DocTab;
use reasoncraft::export::{ExportFormat, ExportOptions, ExportRequest, PageSetup};
use reasoncraft::file_ops::FakeDialogs;
use reasoncraft::i18n::t;
use reasoncraft_core::{CardKind, ConnectorStyle, Document, Parent};

type H = Harness<'static, ReasonCraftApp>;

fn harness(dialogs: FakeDialogs) -> H {
    let mut doc = Document::new("Claim");
    let root = doc.root;
    for text in ["Alpha", "Beta"] {
        doc.add_card(Parent::Box(root), CardKind::Reason, text)
            .unwrap();
    }
    let mut app = ReasonCraftApp::new_for_test_with(Box::new(dialogs));
    app.tabs[0] = DocTab::from_document(doc);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .with_max_steps(32)
        .build_ui_state(|ui, app: &mut ReasonCraftApp| app.show(ui), app);
    h.run();
    h
}

#[test]
fn ribbon_menu_sets_connector_style() {
    let mut h = harness(FakeDialogs::default());
    assert_eq!(h.state().settings.connector_style, ConnectorStyle::Curved);
    h.get_by_label(&t("ribbon.tab.view")).click();
    h.run();
    h.get_by_label(&t("action.connector_style")).click();
    h.run();
    h.get_by_label(&t("connector_style.elbow")).click();
    h.run();
    assert_eq!(h.state().settings.connector_style, ConnectorStyle::Elbow);
    assert!(h.state().tabs[0].history.undo.is_empty(), "no history");
}

/// The `d` attributes of the paths in `svg`.
fn path_data(svg: &str) -> Vec<&str> {
    svg.split("<path d=\"")
        .skip(1)
        .filter_map(|rest| rest.split('"').next())
        .collect()
}

/// Exports lay boxes out like the screen: a rated claim gets its indicator
/// row only while markers are shown, in the canvas and in the export.
#[test]
fn export_boxes_match_the_screen_layout() {
    use reasoncraft::canvas::measure::{EguiMeasure, MeasureCache};
    let dir = tempfile::tempdir().unwrap();
    let dialogs = FakeDialogs::default();
    let mut h = harness(dialogs.clone());
    let root = h.state().tabs[0].doc.root;
    h.state_mut().tabs[0]
        .doc
        .boxes
        .get_mut(&root)
        .unwrap()
        .acceptance = Some(reasoncraft_core::Acceptance::Accept);
    for markers in [true, false] {
        h.state_mut().settings.show_markers = markers;
        let path = dir.path().join(format!("{markers}.svg"));
        dialogs.set_export(path.clone());
        let options = ExportOptions {
            svg_outlines: false,
            ..ExportOptions::default()
        };
        reasoncraft::actions::apply(
            h.state_mut(),
            Action::RunExport(ExportRequest {
                format: ExportFormat::Svg,
                options,
                page: PageSetup::default(),
            }),
        );
        let start = Instant::now();
        while h.state().export.busy() {
            assert!(start.elapsed() < Duration::from_secs(60), "export hangs");
            h.step();
            std::thread::sleep(Duration::from_millis(5));
        }
        let svg = std::fs::read_to_string(path).unwrap();
        // The layout the canvas draws with these settings.
        let mut cache = MeasureCache::default();
        let params = h.state().settings.layout_params();
        let lay = reasoncraft_core::layout(
            &h.state().tabs[0].doc,
            &EguiMeasure::new(&h.ctx, &mut cache),
            &params,
        );
        let claim = lay.boxes[&root];
        assert_eq!(lay.indicator_row(root) > 0.0, markers);
        let rect = format!("width=\"{}\" height=\"{}\"", claim.width(), claim.height());
        assert!(svg.contains(&rect), "{markers}: no {rect} in the export");
    }
}

#[test]
fn svg_export_draws_the_connector_style() {
    let dir = tempfile::tempdir().unwrap();
    let dialogs = FakeDialogs::default();
    let mut h = harness(dialogs.clone());
    let export = |h: &mut H, style: ConnectorStyle| {
        h.state_mut().settings.connector_style = style;
        let path = dir.path().join(format!("{}.svg", style.key()));
        dialogs.set_export(path.clone());
        let options = ExportOptions {
            svg_outlines: false,
            ..ExportOptions::default()
        };
        reasoncraft::actions::apply(
            h.state_mut(),
            Action::RunExport(ExportRequest {
                format: ExportFormat::Svg,
                options,
                page: PageSetup::default(),
            }),
        );
        let start = Instant::now();
        while h.state().export.busy() {
            assert!(start.elapsed() < Duration::from_secs(60), "export hangs");
            h.step();
            std::thread::sleep(Duration::from_millis(5));
        }
        std::fs::read_to_string(path).unwrap()
    };
    let curved = export(&mut h, ConnectorStyle::Curved);
    assert!(
        path_data(&curved).iter().any(|d| d.contains('C')),
        "S-curves"
    );
    let tapered = export(&mut h, ConnectorStyle::Tapered);
    let paths = path_data(&tapered);
    assert!(!paths.iter().any(|d| d.contains('C')), "no stroked curves");
    let bands = paths
        .iter()
        .filter(|d| d.matches('L').count() >= 31)
        .count();
    assert_eq!(bands, 2, "one band per connector: {paths:?}");
}
