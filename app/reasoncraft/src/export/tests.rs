use std::collections::HashSet;
use std::io::Cursor;

use eframe::egui;
use image::{DynamicImage, ImageFormat, RgbImage};
use reasoncraft_core::layout::BOX_TEXT_PAD;
use reasoncraft_core::{
    Assets, BoxKind, CardKind, Document, ImageRef, Item, LayoutParams, Parent, SourceType,
    fixtures::sample_iot_like, layout,
};
use resvg::usvg;

use super::*;
use crate::canvas::measure::{EguiMeasure, MeasureCache};
use crate::scene::{Scene, SceneOptions, Shape, TextRows, build_scene};
use crate::theme::Tokens;

/// The export scene of `doc`, laid out with egui's fonts, and the rows
/// each box's text wraps into.
pub(super) fn scene_of(doc: &Document) -> (Scene, Vec<Vec<String>>) {
    let ctx = egui::Context::default();
    let mut result = None;
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        let mut cache = MeasureCache::default();
        let m = EguiMeasure::new(ui.ctx(), &mut cache);
        let lay = layout(doc, &m, &LayoutParams::default());
        let scene = build_scene(
            doc,
            &lay,
            &SceneOptions::export(Tokens::dark(), crate::i18n::language_of(&doc.meta.language)),
            &m,
        );
        let parked = parked(doc);
        let rows = lay
            .boxes
            .iter()
            .filter(|(id, _)| !parked.contains(&Item::Box(**id)))
            .filter_map(|(id, r)| {
                let node = doc.boxes.get(id)?;
                let width = r.width() - 2.0 * BOX_TEXT_PAD;
                (!node.text.is_empty()).then(|| m.rows(&node.text, width, &node.kind))
            })
            .collect();
        result = Some((scene, rows));
    });
    out.textures_delta.clear();
    result.unwrap()
}

/// The parking area's items and everything under them.
fn parked(doc: &Document) -> HashSet<Item> {
    doc.parking
        .iter()
        .flat_map(|&p| std::iter::once(p).chain(doc.descendants(p)))
        .collect()
}

fn live() -> SvgOptions {
    SvgOptions {
        text_as_paths: false,
        ..SvgOptions::new(&Tokens::dark())
    }
}

fn svg_of(doc: &Document, assets: &Assets, opts: &SvgOptions) -> String {
    let (scene, _) = scene_of(doc);
    scene_to_svg(&scene, assets, opts, &bundled_font_family())
}

/// The contents of every `<text>` element, in order.
fn text_elements(svg: &str) -> Vec<String> {
    svg.split("<text")
        .skip(1)
        .map(|s| {
            let body = &s[s.find('>').unwrap() + 1..];
            body[..body.find("</text>").unwrap()].to_owned()
        })
        .collect()
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn parse(svg: &str) -> usvg::Tree {
    let opt = usvg::Options {
        fontdb: shared_fontdb(),
        ..Default::default()
    };
    usvg::Tree::from_str(svg, &opt).expect("valid SVG")
}

fn png(w: u32, h: u32) -> Vec<u8> {
    let img = RgbImage::from_fn(w, h, |x, y| image::Rgb([x as u8 * 30, y as u8 * 30, 200]));
    let mut out = Vec::new();
    DynamicImage::ImageRgb8(img)
        .write_to(&mut Cursor::new(&mut out), ImageFormat::Png)
        .unwrap();
    out
}

#[test]
fn svg_parses_with_usvg() {
    let doc = sample_iot_like(30);
    let svg = svg_of(&doc, &Assets::default(), &live());
    let tree = parse(&svg);
    assert!(tree.size().width() > 100.0);
    assert_eq!(text_nodes(&tree), text_elements(&svg).len());
}

#[test]
fn svg_text_rows_match_layout() {
    let mut doc = sample_iot_like(20);
    let root = Parent::Box(doc.root);
    // Dutch diacritics, a long word, an emoji and a blank line.
    let long = "Fietsenstallingscapaciteitsuitbreidingsplannen worden één voor één \
                geëvalueerd 🚲 door de gemeente\n\nnaïef?";
    doc.add_card(root, CardKind::Reason, long).unwrap();
    let (scene, boxes) = scene_of(&doc);
    let svg = scene_to_svg(&scene, &Assets::default(), &live(), &bundled_font_family());
    let texts = text_elements(&svg);
    assert!(boxes.iter().any(|rows| rows.len() >= 3), "some text wraps");
    for rows in &boxes {
        let want: Vec<String> = rows.iter().map(|r| escape(r)).collect();
        let found = texts.windows(want.len()).any(|w| w == want.as_slice());
        assert!(found, "rows {want:?} not found as consecutive <text>s");
    }
    // One <text> per row of every text in the scene.
    let rows: usize = scene
        .shapes
        .iter()
        .map(|s| match s {
            Shape::Text { rows, .. } => rows.len(),
            _ => 0,
        })
        .sum();
    assert_eq!(texts.len(), rows);
}

#[test]
fn svg_escapes_text() {
    let doc = Document::new("<b>&\"x\"");
    let svg = svg_of(&doc, &Assets::default(), &live());
    assert!(svg.contains("&lt;b&gt;&amp;&quot;x&quot;"), "{svg}");
    assert!(!svg.contains("<b>"));
    assert!(svg.contains(r#"xml:space="preserve""#));
    parse(&svg);
}

#[test]
fn svg_embeds_images() {
    let mut doc = Document::new("Claim");
    let mut assets = Assets::default();
    let bytes = png(4, 2);
    let key = assets.insert("png", bytes.clone());
    let root = doc.root;
    let p = doc
        .add_card(Parent::Box(root), CardKind::Reason, "Has a picture")
        .unwrap()
        .1;
    let q = doc
        .add_card(Parent::Box(root), CardKind::Reason, "Lost its picture")
        .unwrap()
        .1;
    let image = |asset: &str| {
        Some(ImageRef {
            asset: asset.into(),
            aspect: 0.5,
        })
    };
    doc.set_image(p, image(&key)).unwrap();
    doc.set_image(q, image("gone.png")).unwrap();
    let svg = svg_of(&doc, &assets, &live());
    use base64::Engine as _;
    let data = base64::engine::general_purpose::STANDARD.encode(&bytes);
    assert!(svg.contains(&format!("data:image/png;base64,{data}")));
    assert_eq!(svg.matches("<image").count(), 1);
    assert!(
        svg.contains("stroke-dasharray"),
        "missing image placeholder"
    );
    assert!(text_elements(&svg).contains(&escape(&crate::i18n::t("image.missing"))));
    let tree = parse(&svg);
    assert_eq!(
        count(tree.root(), &|n| matches!(n, usvg::Node::Image(_))),
        1
    );
}

#[test]
fn svg_refuses_images_the_screen_refuses() {
    let mut doc = Document::new("Claim");
    let mut bytes = png(4, 2);
    // A forged header: wider than any image this build stores.
    let side = crate::canvas::images::MAX_STORED_SIDE + 1;
    bytes[16..20].copy_from_slice(&side.to_be_bytes());
    let mut assets = Assets::default();
    let key = assets.insert("png", bytes);
    let root = doc.root;
    let image = Some(ImageRef {
        asset: key,
        aspect: 0.5,
    });
    doc.set_image(root, image).unwrap();
    let svg = svg_of(&doc, &assets, &live());
    assert!(!svg.contains("<image"), "{svg}");
    assert!(text_elements(&svg).contains(&escape(&crate::i18n::t("image.missing"))));
}

/// The nodes under `g` (subtrees included) that match `pred`. (usvg's
/// own `has_text_nodes` always answers yes.)
fn count(g: &usvg::Group, pred: &dyn Fn(&usvg::Node) -> bool) -> usize {
    let mut n = 0;
    for node in g.children() {
        n += usize::from(pred(node));
        if let usvg::Node::Group(g) = node {
            n += count(g, pred);
        }
        node.subroots(|sub| n += count(sub, pred));
    }
    n
}

fn text_nodes(tree: &usvg::Tree) -> usize {
    count(tree.root(), &|n| matches!(n, usvg::Node::Text(_)))
}

#[test]
fn svg_outlines_have_no_text_elements() {
    let mut doc = sample_iot_like(20);
    let root = Parent::Box(doc.root);
    doc.add_box(
        root,
        BoxKind::Source {
            source_type: SourceType::Data,
        },
        "A source",
    )
    .ok();
    let opts = SvgOptions::new(&Tokens::light());
    assert!(opts.text_as_paths, "outlines by default");
    let svg = svg_of(&doc, &Assets::default(), &opts);
    assert!(!svg.contains("<text"), "text left in outlines");
    let tree = parse(&svg);
    assert_eq!(text_nodes(&tree), 0);
    assert!(svg.contains("<path"));
}

#[test]
fn svg_view_box_has_margin_and_background() {
    let doc = Document::new("Claim");
    let (scene, _) = scene_of(&doc);
    let opts = live();
    let svg = scene_to_svg(&scene, &Assets::default(), &opts, "x");
    let b = scene.bounds.expand(DEFAULT_MARGIN);
    let view = format!(
        r#"viewBox="{} {} {} {}""#,
        num(b.min.x),
        num(b.min.y),
        num(b.width()),
        num(b.height())
    );
    assert!(svg.contains(&view), "{view} in {svg}");
    let [r, g, bl, _] = opts.background.to_srgba_unmultiplied();
    assert!(svg.contains(&format!("#{r:02x}{g:02x}{bl:02x}")));
    let clear = SvgOptions {
        transparent: true,
        ..opts
    };
    let svg = scene_to_svg(&scene, &Assets::default(), &clear, "x");
    assert!(!svg.contains(&format!("fill=\"#{r:02x}{g:02x}{bl:02x}\"")));
}

/// Formats like the writer does.
fn num(v: f32) -> String {
    svg::num(v)
}

#[test]
fn bundled_fonts_have_the_text_family() {
    let db = bundled_fontdb();
    let family = &TEXT_FACE.family;
    assert!(
        db.faces()
            .any(|f| f.families.iter().any(|(n, _)| n == family)),
        "{family}"
    );
    assert!(bundled_font_family().starts_with(&format!("'{family}'")));
}

#[test]
fn text_metrics_match_egui() {
    let ctx = egui::Context::default();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        for size in [9.0, 10.5, 11.0, 11.5, 12.0, 13.0, 14.0, 20.0, 26.5] {
            let font = egui::FontId::proportional(size);
            let g = ui.ctx().fonts_mut(|f| {
                f.layout_no_wrap("Mg".to_owned(), font.clone(), egui::Color32::WHITE)
            });
            let baseline = g.rows[0].glyphs[0].pos.y;
            assert_eq!(TEXT_FACE.baseline(size), baseline, "baseline at {size}");
            assert_eq!(TEXT_FACE.row_height(size), g.size().y, "row at {size}");
        }
    });
    out.textures_delta.clear();
}

/// A small map with every kind of box, an image and Dutch text.
pub(super) fn preview_doc(assets: &mut Assets) -> Document {
    let mut doc = Document::new("Het centrum wordt autovrij gemaakt");
    let root = Parent::Box(doc.root);
    let (_, p) = doc
        .add_card(
            root,
            CardKind::Reason,
            "Winkeliers zien meer klanten één jaar na de invoering",
        )
        .unwrap();
    let key = assets.insert("png", png(9, 6));
    doc.set_image(
        p,
        Some(ImageRef {
            asset: key,
            aspect: 6.0 / 9.0,
        }),
    )
    .unwrap();
    let (_, o) = doc
        .add_card(
            root,
            CardKind::Objection,
            "Bevoorrading wordt lastiger voor naïeve planners",
        )
        .unwrap();
    doc.add_box(
        Parent::Box(p),
        BoxKind::Source {
            source_type: SourceType::Data,
        },
        "CBS, 2024",
    )
    .unwrap();
    doc.add_card(
        Parent::Box(o),
        CardKind::Objection,
        "Venstertijden lossen dat op",
    )
    .unwrap();
    doc.add_box(root, BoxKind::Note, "Nog na te gaan").unwrap();
    doc
}

/// Rasterises the preview map's SVG with resvg: outlined and live text
/// render alike. With `RC_SVG_PREVIEW=<dir>` set, writes the map as
/// `task-3-sample.rcraft` and the render as `task-3-svg.png` there, to put
/// next to a screenshot of the app.
#[test]
fn svg_renders_with_resvg() {
    let mut assets = Assets::default();
    let doc = preview_doc(&mut assets);
    let (scene, _) = scene_of(&doc);
    let family = bundled_font_family();
    let render = |opts: &SvgOptions| {
        let tree = parse(&scene_to_svg(&scene, &assets, opts, &family));
        let size = tree.size().to_int_size();
        let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width(), size.height()).unwrap();
        resvg::render(&tree, Default::default(), &mut pixmap.as_mut());
        pixmap
    };
    let outlined = render(&SvgOptions::new(&Tokens::dark()));
    let live = render(&live());
    assert_eq!(outlined.width(), live.width());
    let differing = outlined
        .pixels()
        .iter()
        .zip(live.pixels())
        .filter(|(a, b)| a != b)
        .count();
    assert!(
        differing * 1000 < outlined.pixels().len(),
        "{differing} pixels differ"
    );
    let bg = outlined.pixel(1, 1).unwrap();
    assert!(outlined.pixels().iter().filter(|p| **p != bg).count() > 1000);
    // The embedded image is drawn (it is the only blue thing).
    let blue = |p: &&resvg::tiny_skia::PremultipliedColorU8| p.blue() > 150 && p.green() < 120;
    assert!(
        outlined.pixels().iter().filter(blue).count() > 1000,
        "image drawn"
    );
    if let Some(dir) = std::env::var_os("RC_SVG_PREVIEW") {
        let dir = std::path::PathBuf::from(dir);
        let mut pkg = reasoncraft_core::Package::new(doc);
        pkg.assets = assets;
        reasoncraft_core::file::save_package(&pkg, &dir.join("task-3-sample.rcraft")).unwrap();
        outlined.save_png(dir.join("task-3-svg.png")).unwrap();
    }
}
