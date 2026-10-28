//! Faux bold in exports: drawn once, so its text is there once.

use reasoncraft_core::{Assets, BoxKind, Document, Parent, SourceType};

use super::pdf_tests::extract_text;
use super::tests::scene_of;
use super::*;
use crate::scene::Scene;
use crate::theme::Tokens;

/// A map with two statistic sources, each with a bold "Statistic" label.
fn two_statistics() -> Scene {
    let mut doc = Document::new("Claim");
    let kind = BoxKind::Source {
        source_type: SourceType::Statistic,
    };
    for text in ["CBS, 2024", "RIVM, 2023"] {
        doc.add_box(Parent::Box(doc.root), kind.clone(), text)
            .unwrap();
    }
    scene_of(&doc).0
}

fn svg(scene: &Scene, opts: &SvgOptions) -> String {
    scene_to_svg(scene, &Assets::default(), opts, &bundled_font_family())
}

fn label() -> String {
    crate::i18n::t("source.statistic")
}

/// A PDF's text (what a reader copies) holds each bold label once.
#[test]
fn pdf_bold_text_extracted_once() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("map.pdf");
    let work = ExportWork {
        options: ExportOptions::default(),
        page: PageSetup::default(),
        make: Make::Image {
            format: ImageFormat::Pdf,
            picture: Picture {
                scene: two_statistics(),
                assets: Assets::default(),
                tokens: Tokens::light(),
            },
            path: path.clone(),
        },
    };
    work.run().unwrap();
    let pdf = std::fs::read(&path).unwrap();
    assert_eq!(extract_text(&pdf).matches(label().as_str()).count(), 2);
}

/// Live SVG: one `<text>` per bold label, with a 0.35 px stroke painted
/// under its fill.
#[test]
fn svg_bold_is_one_stroked_text() {
    let opts = SvgOptions {
        text_as_paths: false,
        ..SvgOptions::new(&Tokens::light())
    };
    let svg = svg(&two_statistics(), &opts);
    let bold: Vec<&str> = svg
        .lines()
        .filter(|l| l.starts_with("<text") && l.contains(&format!(">{}<", label())))
        .collect();
    assert_eq!(bold.len(), 2, "{svg}");
    for line in bold {
        assert!(line.contains("stroke-width=\"0.35\""), "{line}");
        assert!(line.contains("paint-order=\"stroke\""), "{line}");
    }
    assert!(!TEXT_FACE.has_bold, "the bundled fonts have no bold face");
}

/// The PDF's SVG draws the bold stroke as outlines under fill-only text,
/// and looks the same as the stroked text.
#[test]
fn pdf_svg_bold_outline_looks_like_stroked_text() {
    let scene = two_statistics();
    let pdf_opts = SvgOptions::for_pdf(&Tokens::light());
    let pdf_svg = svg(&scene, &pdf_opts);
    let bold_lines: Vec<&str> = pdf_svg
        .lines()
        .filter(|l| l.contains(&format!(">{}<", label())))
        .collect();
    assert_eq!(bold_lines.len(), 2);
    assert!(bold_lines.iter().all(|l| !l.contains("stroke")));
    assert_eq!(pdf_svg.matches("stroke-width=\"0.35\"").count(), 2);
    let live = SvgOptions {
        bold_stroke_as_path: false,
        ..pdf_opts
    };
    let png = PngOptions {
        scale: 4.0,
        ..PngOptions::default()
    };
    let fonts = bundled_fontdb();
    let (_, _, a) = svg_to_rgba(&svg(&scene, &live), &png, &fonts).unwrap();
    let (_, _, b) = svg_to_rgba(&pdf_svg, &png, &fonts).unwrap();
    let differing = a
        .chunks(4)
        .zip(b.chunks(4))
        .filter(|(p, q)| p.iter().zip(*q).any(|(x, y)| x.abs_diff(*y) > 24))
        .count();
    // Leaving the stroke out makes ~2800 of these ~1M pixels differ.
    assert!(differing * 2000 < a.len() / 4, "{differing} pixels differ");
}
