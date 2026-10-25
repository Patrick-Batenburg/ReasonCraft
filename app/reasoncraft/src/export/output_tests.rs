//! Export helpers the app's export jobs use: scenes straight to PNG and
//! the clipboard, the one-page fit cap and tile labels at small margins.

use eframe::egui::{Color32, Pos2, Rect, vec2};
use reasoncraft_core::{Assets, CardKind, Document, Parent};

use super::pdf::placements;
use super::tests::scene_of;
use super::*;
use crate::scene::{Scene, Shape};
use crate::theme::Tokens;

/// A scene of one filled rect of `w` x `h` world px.
fn rect_scene(w: f32, h: f32) -> Scene {
    let rect = Rect::from_min_size(Pos2::ZERO, vec2(w, h));
    Scene {
        shapes: vec![Shape::Rect {
            rect,
            radius: 0.0,
            fill: Color32::from_rgb(200, 40, 40),
            stroke: None,
        }],
        bounds: rect,
    }
}

#[derive(Default)]
struct Sink(Option<(u32, u32, Vec<u8>)>);

impl ClipboardSink for Sink {
    fn set_image(&mut self, w: u32, h: u32, rgba: &[u8]) -> Result<(), ExportError> {
        self.0 = Some((w, h, rgba.to_vec()));
        Ok(())
    }
}

#[test]
fn scene_png_transparency_comes_from_png_options() {
    let scene = rect_scene(20.0, 10.0);
    let tokens = Tokens::light();
    let fonts = bundled_fontdb();
    let corner = |transparent| {
        let opts = PngOptions {
            scale: 1.0,
            transparent,
        };
        let png = scene_to_png(&scene, &Assets::default(), &tokens, &opts, &fonts).unwrap();
        let (w, h, px) = png_to_rgba(&png).unwrap();
        assert_eq!((w, h), (20 + 2 * 24, 10 + 2 * 24), "margin on every side");
        [px[0], px[1], px[2], px[3]]
    };
    let c = tokens.canvas;
    assert_eq!(corner(false), [c.r(), c.g(), c.b(), 255], "canvas colour");
    assert_eq!(corner(true)[3], 0, "no background at all");
}

#[test]
fn copy_scene_puts_pixels_on_the_sink() {
    let scene = rect_scene(20.0, 10.0);
    let mut sink = Sink::default();
    let opts = PngOptions::default();
    let tokens = Tokens::dark();
    let fonts = bundled_fontdb();
    copy_scene_to_clipboard(
        &scene,
        &Assets::default(),
        &tokens,
        &opts,
        &fonts,
        &mut sink,
    )
    .unwrap();
    let (w, h, px) = sink.0.unwrap();
    assert_eq!((w, h), (2 * 68, 2 * 58));
    assert_eq!(px.len(), (w * h * 4) as usize);
}

#[test]
fn clipboard_copy_is_capped_lower_than_files() {
    // 9000 x 9000 px is fine for a PNG file but too much for the clipboard.
    let scene = rect_scene(9000.0 - 48.0, 9000.0 - 48.0);
    let opts = PngOptions {
        scale: 1.0,
        transparent: false,
    };
    assert!(u64::from(9000u32 * 9000) > MAX_CLIPBOARD_PIXELS);
    assert!(u64::from(9000u32 * 9000) < MAX_PIXELS);
    let mut sink = Sink::default();
    let err = copy_scene_to_clipboard(
        &scene,
        &Assets::default(),
        &Tokens::light(),
        &opts,
        &bundled_fontdb(),
        &mut sink,
    )
    .unwrap_err();
    assert_eq!(err.key(), "export-error-clipboard_too_large");
    assert_eq!(err.detail(), "9000 × 9000");
    assert!(sink.0.is_none(), "nothing reached the clipboard");
}

#[test]
fn one_page_never_enlarges_a_small_map() {
    let mut doc = Document::new("Claim");
    let root = doc.root;
    doc.add_card(Parent::Box(root), CardKind::Reason, "Because")
        .unwrap();
    let (scene, _) = scene_of(&doc);
    let svg = scene_to_svg(
        &scene,
        &Assets::default(),
        &SvgOptions::new(&Tokens::light()),
        &bundled_font_family(),
    );
    let tree = resvg::usvg::Tree::from_str(&svg, &Default::default()).unwrap();
    let (w, h) = (tree.size().width(), tree.size().height());
    let setup = PageSetup::default();
    let (pages, [_, _, aw, ah]) = placements(w, h, &setup).unwrap();
    let p = pages[0];
    // 1 world px = 0.25 mm, as a tile at scale 1.
    let natural = 0.25 * 72.0 / 25.4;
    assert!((p.scale - natural).abs() < 1e-4, "{} != {natural}", p.scale);
    assert!(
        w * p.scale < aw && h * p.scale < ah,
        "smaller than the page"
    );
    // Still centred.
    let (pw, ph) = setup.page_size_pt();
    assert!((p.left + w * p.scale / 2.0 - pw / 2.0).abs() < 0.01);
    assert!((p.top - h * p.scale / 2.0 - ph / 2.0).abs() < 0.01);
    // A big map is still shrunk to fit.
    let (pages, _) = placements(20_000.0, 9_000.0, &setup).unwrap();
    assert!((20_000.0 * pages[0].scale - aw).abs() < 0.01);
}

#[test]
fn tile_labels_are_left_out_when_the_margin_is_too_small() {
    let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1000\" height=\"600\" \
               viewBox=\"0 0 1000 600\"><rect width=\"1000\" height=\"600\"/></svg>";
    let setup = PageSetup {
        fit: Fit::Tile { scale: 2.0 },
        margin_mm: 1.0,
        ..PageSetup::default()
    };
    let pdf = svg_to_pdf(svg, &setup, &bundled_fontdb()).unwrap();
    let has = |needle: &[u8]| pdf.windows(needle.len()).any(|w| w == needle);
    assert!(!has(b") Tj"), "no labels in a 1 mm margin");
    assert!(!has(b"Helvetica"));
}

#[test]
fn print_file_is_written_without_opening() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_print_file(b"%PDF-1.7 x", dir.path()).unwrap();
    assert_eq!(path.parent().unwrap(), dir.path());
    assert_eq!(std::fs::read(path).unwrap(), b"%PDF-1.7 x");
}

#[test]
fn copy_survives_a_poisoned_clipboard_lock() {
    let fake = FakeClipboard::default();
    let shared: SharedClipboard = std::sync::Arc::new(std::sync::Mutex::new(fake.clone()));
    let poison = shared.clone();
    let _ = std::thread::spawn(move || {
        let _guard = poison.lock().unwrap();
        panic!("poison the lock");
    })
    .join();
    assert!(shared.is_poisoned());
    let work = ExportWork {
        format: ExportFormat::Png,
        options: ExportOptions::default(),
        page: PageSetup::default(),
        snapshot: Snapshot::Scene {
            scene: rect_scene(10.0, 10.0),
            assets: Assets::default(),
            tokens: Tokens::light(),
        },
        target: Target::Clipboard(shared),
    };
    assert_eq!(work.run().unwrap(), Outcome::Copied);
    assert_eq!(fake.images.lock().unwrap().len(), 1);
}

#[test]
fn a_job_that_dies_is_reported_as_stopped() {
    let job: Job<u8> = Job::spawn(None, || panic!("worker dies"));
    let start = std::time::Instant::now();
    loop {
        match job.poll() {
            JobState::Running => {
                assert!(start.elapsed().as_secs() < 30);
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            JobState::Lost => break,
            JobState::Done(_) => panic!("no result expected"),
        }
    }
    assert_eq!(ExportError::Stopped.key(), "export-error-stopped");
    assert_eq!(
        ExportError::Unsupported(ExportFormat::EssayDocx).key(),
        "export-error-unsupported"
    );
}
