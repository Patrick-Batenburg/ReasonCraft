//! The canvas paints the document's scene: box text, connectors, source
//! headers and markers reach the screen.

use eframe::egui::{self, epaint};
use egui_kittest::Harness;
use reasoncraft::ReasonCraftApp;
use reasoncraft::doc_tab::DocTab;
use reasoncraft::i18n::t;
use reasoncraft_core::{Acceptance, BoxKind, CardKind, Document, Parent, SourceType};

/// Every shape painted in the last frame, groups flattened.
fn shapes(h: &Harness<'_, ReasonCraftApp>) -> Vec<epaint::Shape> {
    fn flatten(s: &epaint::Shape, out: &mut Vec<epaint::Shape>) {
        match s {
            epaint::Shape::Vec(v) => v.iter().for_each(|s| flatten(s, out)),
            other => out.push(other.clone()),
        }
    }
    let mut out = Vec::new();
    for clipped in &h.output().shapes {
        flatten(&clipped.shape, &mut out);
    }
    out
}

#[test]
fn canvas_paints_the_scene() {
    let mut doc = Document::new("Cities should ban cars");
    let root = doc.root;
    let (_, premise) = doc
        .add_card(Parent::Box(root), CardKind::Reason, "Cleaner air")
        .unwrap();
    doc.add_box(
        Parent::Box(premise),
        BoxKind::Source {
            source_type: SourceType::Statistic,
        },
        "Air report",
    )
    .unwrap();
    doc.boxes.get_mut(&premise).unwrap().acceptance = Some(Acceptance::Accept);
    let mut app = ReasonCraftApp::new_for_test();
    app.tabs[0] = DocTab::from_document(doc);
    app.left_panel = false;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .with_max_steps(32)
        .build_ui_state(|ui, app: &mut ReasonCraftApp| app.show(ui), app);
    h.run();

    let painted = shapes(&h);
    let texts: Vec<String> = painted
        .iter()
        .filter_map(|s| match s {
            epaint::Shape::Text(t) => Some(t.galley.job.text.clone()),
            _ => None,
        })
        .collect();
    for want in [
        "Cities should ban cars",
        "Cleaner air",
        "Air report",
        &t("source.statistic"),
        "supports",
    ] {
        assert!(texts.iter().any(|t| t == want), "{want:?} in {texts:?}");
    }
    let curves = painted
        .iter()
        .filter(|s| matches!(s, epaint::Shape::CubicBezier(_)))
        .count();
    assert_eq!(curves, 2, "card and source connectors");
    let tk = h.state().tokens;
    let marker = painted.iter().any(|s| {
        matches!(s, epaint::Shape::Circle(c) if c.fill == tk.box_fill && c.stroke.color == tk.reason)
    });
    assert!(marker, "accept marker");
}

#[test]
#[ignore = "timing: run in release"]
fn scene_500_boxes_fast() {
    use reasoncraft::canvas::measure::{EguiMeasure, MeasureCache};
    use reasoncraft::scene::{SceneOptions, build_scene};
    use reasoncraft::theme::Tokens;
    use reasoncraft_core::{LayoutParams, fixtures::sample_iot_like, layout};

    let doc = sample_iot_like(500);
    let ctx = egui::Context::default();
    let mut cache = MeasureCache::default();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        let measure = EguiMeasure::new(ui.ctx(), &mut cache);
        let layout = layout(&doc, &measure, &LayoutParams::default());
        let opts = SceneOptions::export(Tokens::dark());
        // Warm the glyph cache, then time a whole-map build (no culling).
        build_scene(&doc, &layout, &opts, &measure);
        let started = std::time::Instant::now();
        let runs = 20;
        for _ in 0..runs {
            build_scene(&doc, &layout, &opts, &measure);
        }
        let each = started.elapsed() / runs;
        println!("scene of 500 boxes: {each:?}");
        assert!(each < std::time::Duration::from_millis(4), "{each:?}");
    });
    out.textures_delta.clear();
}
