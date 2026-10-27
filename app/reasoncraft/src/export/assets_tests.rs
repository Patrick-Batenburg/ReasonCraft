//! Box images in exports: JPEG and WebP become PNG; anything else is the
//! placeholder.

use std::borrow::Cow;
use std::io::Cursor;

use base64::Engine as _;
use image::{DynamicImage, ImageFormat, RgbImage};
use reasoncraft_core::{Assets, CardKind, Document, ImageRef, Parent};

use super::tests::scene_of;
use super::*;
use crate::theme::Tokens;

fn encoded(w: u32, h: u32, format: ImageFormat) -> Vec<u8> {
    let img = DynamicImage::ImageRgb8(RgbImage::from_pixel(w, h, image::Rgb([30, 60, 200])));
    let mut out = Vec::new();
    img.write_to(&mut Cursor::new(&mut out), format).unwrap();
    out
}

/// The language of the map [`svg_with_image`] draws.
const LANG: &str = "nl";

/// The live SVG of a map in [`LANG`] whose one card shows `bytes`.
fn svg_with_image(bytes: Vec<u8>) -> String {
    let mut doc = Document::new("Claim");
    doc.meta.language = LANG.to_owned();
    let mut assets = Assets::default();
    let key = assets.insert("img", bytes);
    let (_, card) = doc
        .add_card(Parent::Box(doc.root), CardKind::Reason, "Picture")
        .unwrap();
    let image = ImageRef {
        asset: key,
        aspect: 0.5,
    };
    doc.set_image(card, Some(image)).unwrap();
    let (scene, _) = scene_of(&doc);
    let opts = SvgOptions {
        text_as_paths: false,
        ..SvgOptions::new(&Tokens::light())
    };
    scene_to_svg(&scene, &assets, &opts, "x")
}

/// The images an SVG embeds, decoded from their data URLs.
fn embedded(svg: &str) -> Vec<Vec<u8>> {
    svg.split("data:image/png;base64,")
        .skip(1)
        .map(|s| {
            let data = &s[..s.find('"').unwrap()];
            base64::engine::general_purpose::STANDARD
                .decode(data)
                .unwrap()
        })
        .collect()
}

#[test]
fn jpeg_asset_exports_as_png() {
    let svg = svg_with_image(encoded(8, 4, ImageFormat::Jpeg));
    let images = embedded(&svg);
    assert_eq!(images.len(), 1, "{svg}");
    let png = image::load_from_memory_with_format(&images[0], ImageFormat::Png).unwrap();
    assert_eq!((png.width(), png.height()), (8, 4));
    assert!(!svg.contains(&crate::i18n::t_lang(LANG, "image.missing")));
}

#[test]
fn webp_asset_exports_as_png() {
    let images = embedded(&svg_with_image(encoded(6, 3, ImageFormat::WebP)));
    assert_eq!(images.len(), 1);
    let png = image::load_from_memory_with_format(&images[0], ImageFormat::Png).unwrap();
    assert_eq!((png.width(), png.height()), (6, 3));
}

/// The start of a GIF: a format exports do not convert.
const GIF: &[u8] = b"GIF89a\x01\x00\x01\x00\x00\x00\x00;";

#[test]
fn png_assets_pass_unchanged_and_others_are_refused() {
    let png = encoded(2, 2, ImageFormat::Png);
    assert!(matches!(export_png_bytes(&png), Some(Cow::Borrowed(b)) if b == png.as_slice()));
    assert!(export_png_bytes(GIF).is_none());
    assert!(export_png_bytes(b"\xff\xd8\xff\xe0 not really a jpeg").is_none());
    let svg = svg_with_image(GIF.to_vec());
    assert!(!svg.contains("<image"));
    assert!(svg.contains(&crate::i18n::t_lang(LANG, "image.missing")));
}

/// A JPEG larger than this build ever stores is refused, as on screen.
#[test]
fn oversized_jpeg_is_the_placeholder() {
    let side = crate::canvas::images::MAX_STORED_SIDE + 1;
    let svg = svg_with_image(encoded(side, 1, ImageFormat::Jpeg));
    assert!(!svg.contains("<image"));
}
