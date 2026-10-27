use eframe::egui::{Align, Color32, Pos2, Rect, pos2};
use reasoncraft_core::Assets;

use super::raster::{MAX_RGBA_BYTES, pixel_size};
use super::tests::{preview_doc, scene_of};
use super::*;
use crate::scene::{Scene, Shape};
use crate::theme::Tokens;

fn svg_for(scene: &Scene, transparent: bool) -> String {
    let opts = SvgOptions {
        transparent,
        ..SvgOptions::new(&Tokens::dark())
    };
    scene_to_svg(scene, &Assets::default(), &opts, &bundled_font_family())
}

fn rect_scene(w: f32, h: f32) -> Scene {
    Scene {
        shapes: vec![Shape::Rect {
            rect: Rect::from_min_size(Pos2::ZERO, eframe::egui::vec2(w, h)),
            radius: 4.0,
            fill: Color32::RED,
            stroke: None,
        }],
        bounds: Rect::from_min_size(Pos2::ZERO, eframe::egui::vec2(w, h)),
    }
}

fn png_size(png: &[u8]) -> (u32, u32) {
    let (w, h, _) = png_to_rgba(png).unwrap();
    (w, h)
}

#[test]
fn png_has_expected_size_for_scale() {
    let mut assets = Assets::default();
    let doc = preview_doc(&mut assets);
    let (scene, _) = scene_of(&doc);
    let svg = scene_to_svg(
        &scene,
        &assets,
        &SvgOptions::new(&Tokens::dark()),
        &bundled_font_family(),
    );
    let b = scene.bounds;
    let (bw, bh) = (
        b.width() + 2.0 * DEFAULT_MARGIN,
        b.height() + 2.0 * DEFAULT_MARGIN,
    );
    for scale in [1.0_f32, 2.0, 3.0, 4.0] {
        let opts = PngOptions {
            scale,
            transparent: false,
        };
        let png = svg_to_png(&svg, &opts, &bundled_fontdb()).unwrap();
        let (w, h) = png_size(&png);
        assert_eq!(w, (bw * scale).ceil() as u32, "width at {scale}");
        assert_eq!(h, (bh * scale).ceil() as u32, "height at {scale}");
    }
}

#[test]
fn png_transparent_background_has_alpha() {
    let scene = rect_scene(40.0, 30.0);
    let fonts = bundled_fontdb();
    let opts = PngOptions {
        scale: 1.0,
        transparent: true,
    };
    let (w, h, px) = svg_to_rgba(&svg_for(&scene, true), &opts, &fonts).unwrap();
    assert_eq!(px.len(), (w * h * 4) as usize);
    assert_eq!(px[3], 0, "corner is see-through");
    let mid = (((h / 2) * w + w / 2) * 4) as usize;
    assert_eq!(
        &px[mid..mid + 4],
        &[255, 0, 0, 255],
        "the shape is opaque and straight"
    );
    // Not transparent: composed on white, fully opaque.
    let opts = PngOptions {
        transparent: false,
        ..opts
    };
    let (_, _, px) = svg_to_rgba(&svg_for(&scene, true), &opts, &fonts).unwrap();
    assert_eq!(&px[..4], &[255, 255, 255, 255]);
    // And with the SVG's own background the corner shows it.
    let (_, _, px) = svg_to_rgba(&svg_for(&scene, false), &opts, &fonts).unwrap();
    let c = Tokens::dark().canvas;
    assert_eq!(&px[..4], &[c.r(), c.g(), c.b(), 255]);
}

#[test]
fn png_export_capped() {
    let fonts = bundled_fontdb();
    let svg = svg_for(&rect_scene(20_000.0, 10.0), true);
    let opts = |scale| PngOptions {
        scale,
        transparent: true,
    };
    // Too long a side, even at scale 1.
    assert!(matches!(
        svg_to_png(&svg, &opts(4.0), &fonts),
        Err(ExportError::TooLarge { w, .. }) if w > MAX_SIDE_PX
    ));
    // Within the side cap but over the memory guard: 8 000 x 8 000 at 2x
    // is about 16 000 px a side, 256 MP (1 GB of RGBA).
    let svg = svg_for(&rect_scene(8_000.0, 8_000.0), true);
    let err = svg_to_png(&svg, &opts(2.0), &fonts).unwrap_err();
    assert!(matches!(err, ExportError::TooLarge { .. }), "{err}");
    assert_eq!(err.key(), "export-error-too_large");
}

#[test]
fn pixel_caps_are_exact() {
    assert!(pixel_size(16_384.0, 10.0, 1.0).is_ok());
    assert!(pixel_size(16_385.0, 10.0, 1.0).is_err());
    // The memory guard (512 MiB of RGBA, 128 Mi pixels) comes before the
    // 256 MP cap, so an image that large is refused for its memory.
    assert!(pixel_size(16_000.0, 16_000.0, 1.0).is_err());
    assert!(pixel_size(f32::NAN, 10.0, 1.0).is_err());
    assert!(pixel_size(10.0, 10.0, 0.0).is_err());
}

#[derive(Default)]
struct FakeClipboard(Option<(u32, u32, Vec<u8>)>);

impl ClipboardSink for FakeClipboard {
    fn set_image(&mut self, w: u32, h: u32, rgba: &[u8]) -> Result<(), ExportError> {
        self.0 = Some((w, h, rgba.to_vec()));
        Ok(())
    }
}

#[test]
fn copy_image_hands_pixels_to_the_sink() {
    let scene = rect_scene(8.0, 6.0);
    let opts = PngOptions {
        scale: 2.0,
        transparent: false,
    };
    let rgba = svg_to_rgba(&svg_for(&scene, false), &opts, &bundled_fontdb()).unwrap();
    let mut sink = FakeClipboard::default();
    copy_image(&mut sink, rgba.clone()).unwrap();
    assert_eq!(sink.0, Some(rgba));
    // A buffer that does not fit its size is refused, not passed on.
    let mut sink = FakeClipboard::default();
    let bad = copy_image(&mut sink, (4, 4, vec![0; 10]));
    assert!(matches!(bad, Err(ExportError::Clipboard(_))));
    assert!(sink.0.is_none());
}

#[test]
fn svg_with_control_characters_and_nan_is_valid() {
    let nan = f32::NAN;
    let scene = Scene {
        shapes: vec![
            Shape::Text {
                origin: pos2(nan, 5.0),
                rows: vec!["a\u{1}b\u{FFFE}c\u{FFFF}d\u{b}e".into()],
                size: 14.0,
                bold: false,
                color: Color32::WHITE,
                align: Align::Min,
                line_height: nan,
            },
            Shape::Rect {
                rect: Rect::from_min_max(pos2(0.0, nan), pos2(f32::INFINITY, 20.0)),
                radius: 2.0,
                fill: Color32::BLUE,
                stroke: None,
            },
        ],
        bounds: Rect::from_min_size(Pos2::ZERO, eframe::egui::vec2(50.0, 30.0)),
    };
    for as_paths in [false, true] {
        let opts = SvgOptions {
            text_as_paths: as_paths,
            ..SvgOptions::new(&Tokens::dark())
        };
        let svg = scene_to_svg(&scene, &Assets::default(), &opts, &bundled_font_family());
        assert!(!svg.contains("NaN") && !svg.contains("inf"), "{svg}");
        assert!(!svg.contains(['\u{1}', '\u{b}', '\u{FFFE}', '\u{FFFF}']));
        if !as_paths {
            assert!(svg.contains(">abcde<"), "{svg}");
        }
        svg_to_png(&svg, &PngOptions::default(), &bundled_fontdb()).unwrap();
    }
}

/// With `RC_PNG_PREVIEW=<dir>` set, writes the preview map at scale 2 as
/// `task-4.png` there.
#[test]
fn png_preview_sample() {
    let Some(dir) = std::env::var_os("RC_PNG_PREVIEW") else {
        return;
    };
    let mut assets = Assets::default();
    let doc = preview_doc(&mut assets);
    let (scene, _) = scene_of(&doc);
    let svg = scene_to_svg(
        &scene,
        &assets,
        &SvgOptions::new(&Tokens::dark()),
        &bundled_font_family(),
    );
    let png = svg_to_png(&svg, &PngOptions::default(), &bundled_fontdb()).unwrap();
    std::fs::write(std::path::Path::new(&dir).join("task-4.png"), png).unwrap();
}

#[test]
fn png_export_refuses_over_memory_guard() {
    // 16 384 x 8 192 px is exactly 512 MiB of RGBA: allowed. One row more
    // is over the guard, though far under the 256 MP cap.
    assert_eq!(MAX_RGBA_BYTES, 512 * 1024 * 1024);
    assert!(pixel_size(16_384.0, 8_192.0, 1.0).is_ok());
    let err = pixel_size(16_384.0, 8_193.0, 1.0).unwrap_err();
    assert!(matches!(
        err,
        ExportError::TooLarge {
            w: 16_384,
            h: 8_193
        }
    ));
    assert_eq!(err.key(), "export-error-too_large", "the cap's message");
    // 12 000 x 12 000 px (144 MP, 576 MB) is refused before rendering.
    let svg = svg_for(&rect_scene(12_000.0, 12_000.0), true);
    let opts = PngOptions {
        scale: 1.0,
        transparent: true,
    };
    let err = svg_to_png(&svg, &opts, &bundled_fontdb()).unwrap_err();
    assert!(matches!(err, ExportError::TooLarge { .. }), "{err}");
}
