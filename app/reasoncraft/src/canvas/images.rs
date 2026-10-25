//! Images in boxes: turning a picked file into the PNG that is stored, the
//! texture cache the canvas draws from, and painting an image (or its
//! placeholder) in the block the layout reserves above a box's text.

use std::collections::HashMap;
use std::io::Cursor;

use eframe::egui::{
    self, Align2, ColorImage, Rect, Shape, Stroke, TextureHandle, TextureOptions, pos2,
};
use image::imageops::FilterType;
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits};
use reasoncraft_core::layout::{BOX_TEXT_PAD, IMAGE_GAP, image_height};
use reasoncraft_core::{Assets, BoxNode, Document, ImageRef};

use super::paint::{Pass, screen_font};
use crate::doc_tab::DocTab;
use crate::i18n::t;

/// Larger picked files are refused before they are decoded.
pub const MAX_INPUT_BYTES: u64 = 40 * 1024 * 1024;
/// Wider or taller images are refused before their pixels are allocated.
pub const MAX_DIMENSION: u32 = 12_000;
/// The most a decoder may allocate.
const MAX_ALLOC: u64 = 512 * 1024 * 1024;
/// Stored images are scaled down so their longest side is at most this.
pub const MAX_SIDE: u32 = 1600;
/// World size of the placeholder's text.
const PLACEHOLDER_TEXT: f32 = 11.0;

/// Why a picked file cannot go in a box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageError {
    /// Over [`MAX_INPUT_BYTES`], or wider or taller than [`MAX_DIMENSION`].
    TooLarge,
    /// Not a PNG, JPEG or WebP image.
    Unsupported,
    /// Damaged: the format is known but the data does not decode.
    Corrupt,
}

impl ImageError {
    /// The i18n key of the message shown for this error.
    pub fn key(self) -> &'static str {
        match self {
            Self::TooLarge => "image.error.too_large",
            Self::Unsupported => "image.error.unsupported",
            Self::Corrupt => "image.error.corrupt",
        }
    }
}

fn classify(err: &image::ImageError) -> ImageError {
    match err {
        image::ImageError::Limits(_) => ImageError::TooLarge,
        image::ImageError::Unsupported(_) => ImageError::Unsupported,
        _ => ImageError::Corrupt,
    }
}

/// Decode `bytes` (any enabled format), upright per its EXIF orientation,
/// refusing images over the size limits before allocating their pixels.
fn decode(bytes: &[u8]) -> Result<DynamicImage, ImageError> {
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| ImageError::Corrupt)?;
    if reader.format().is_none() {
        return Err(ImageError::Unsupported);
    }
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_DIMENSION);
    limits.max_image_height = Some(MAX_DIMENSION);
    limits.max_alloc = Some(MAX_ALLOC);
    reader.limits(limits.clone());
    let mut decoder = reader.into_decoder().map_err(|e| classify(&e))?;
    // The output buffer counts too (`ImageReader::decode` does the same).
    limits
        .reserve(decoder.total_bytes())
        .map_err(|e| classify(&e))?;
    let orientation = decoder.orientation().ok();
    let mut img = DynamicImage::from_decoder(decoder).map_err(|e| classify(&e))?;
    if let Some(o) = orientation {
        img.apply_orientation(o);
    }
    if img.width() == 0 || img.height() == 0 {
        return Err(ImageError::Corrupt);
    }
    Ok(img)
}

/// `img` scaled down (Lanczos3) so its longest side is at most [`MAX_SIDE`].
fn fit(img: DynamicImage) -> DynamicImage {
    if img.width().max(img.height()) > MAX_SIDE {
        img.resize(MAX_SIDE, MAX_SIDE, FilterType::Lanczos3)
    } else {
        img
    }
}

/// Turn a picked image file into what a box stores: PNG bytes, scaled down
/// to at most [`MAX_SIDE`] px on the longest side, and the aspect (height
/// divided by width).
pub fn prepare_image(bytes: &[u8]) -> Result<(Vec<u8>, f32), ImageError> {
    if bytes.len() as u64 > MAX_INPUT_BYTES {
        return Err(ImageError::TooLarge);
    }
    let img = decode(bytes)?;
    let aspect = img.height() as f32 / img.width() as f32;
    let mut png = Vec::new();
    fit(img)
        .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .map_err(|_| ImageError::Corrupt)?;
    Ok((png, aspect))
}

/// A stored asset as egui pixels (`None` when it does not decode).
fn color_image(bytes: &[u8]) -> Option<ColorImage> {
    let rgba = fit(decode(bytes).ok()?).to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    Some(ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()))
}

/// Textures of the images on screen, keyed by asset key and shared by all
/// tabs (equal keys are equal bytes). `None` marks an asset that does not
/// decode. The canvas shows a placeholder for it and for a missing asset.
#[derive(Default)]
pub struct ImageCache {
    textures: HashMap<String, Option<TextureHandle>>,
}

impl ImageCache {
    /// Decode, once, every image `doc` shows that is not cached yet. A
    /// missing asset is not cached, so it loads once its bytes arrive (from
    /// another tab, say); one that does not decode is cached as `None`.
    pub fn load(&mut self, ctx: &egui::Context, doc: &Document, assets: &Assets) {
        for img in doc.boxes.values().filter_map(|b| b.image.as_ref()) {
            if self.textures.contains_key(&img.asset) {
                continue;
            }
            let Some(bytes) = assets.get(&img.asset) else {
                continue;
            };
            let texture = color_image(&bytes).map(|pixels| {
                let name = format!("asset:{}", img.asset);
                ctx.load_texture(name, pixels, TextureOptions::LINEAR)
            });
            self.textures.insert(img.asset.clone(), texture);
        }
    }

    /// Drop the textures no open tab holds the bytes of (its tab closed).
    pub fn evict_closed(&mut self, tabs: &[DocTab]) {
        self.textures
            .retain(|key, _| tabs.iter().any(|tab| tab.assets.get(key).is_some()));
    }

    pub fn texture(&self, key: &str) -> Option<&TextureHandle> {
        self.textures.get(key)?.as_ref()
    }

    pub fn len(&self) -> usize {
        self.textures.len()
    }

    pub fn is_empty(&self) -> bool {
        self.textures.is_empty()
    }
}

/// Where `image` is drawn in a box at `world` (world units).
pub fn image_rect(world: Rect, image: &ImageRef) -> Rect {
    let width = world.width() - 2.0 * BOX_TEXT_PAD;
    let height = image_height(world.width(), image) - BOX_TEXT_PAD - IMAGE_GAP;
    Rect::from_min_size(
        world.min + egui::vec2(BOX_TEXT_PAD, BOX_TEXT_PAD),
        egui::vec2(width, height),
    )
}

/// The part of box `world` below its image: where its text (and a source's
/// header) goes. The whole box when it has no image.
pub fn content_rect(world: Rect, node: &BoxNode) -> Rect {
    let mut rest = world;
    if let Some(img) = &node.image {
        rest.min.y += image_height(world.width(), img);
    }
    rest
}

impl Pass<'_> {
    /// Box `node`'s image in its block at the top of `world`, or a dashed
    /// placeholder when its asset is missing or broken.
    pub(super) fn box_image(&self, world: Rect, node: &BoxNode) {
        let Some(img) = &node.image else { return };
        let r = self.screen(image_rect(world, img));
        if !self.visible(r) {
            return;
        }
        if let Some(texture) = self.images.texture(&img.asset) {
            let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
            self.painter
                .image(texture.id(), r, uv, egui::Color32::WHITE);
            return;
        }
        let colour = self.t.text_muted;
        let stroke = Stroke::new(1.0, colour);
        let (dash, gap) = (5.0, 4.0);
        let corners = [
            r.left_top(),
            r.right_top(),
            r.right_bottom(),
            r.left_bottom(),
        ];
        for i in 0..4 {
            let edge = [corners[i], corners[(i + 1) % 4]];
            self.painter
                .extend(Shape::dashed_line(&edge, stroke, dash, gap));
        }
        if let Some(font) = screen_font(PLACEHOLDER_TEXT, self.cam.zoom) {
            self.painter.text(
                r.center(),
                Align2::CENTER_CENTER,
                t("image.missing"),
                font,
                colour,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reasoncraft_core::{BoxKind, CardKind, NodeId, Parent};

    /// A `w`×`h` PNG with a gradient.
    fn png(w: u32, h: u32) -> Vec<u8> {
        let img = image::RgbImage::from_fn(w, h, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8])
        });
        let mut out = Vec::new();
        DynamicImage::ImageRgb8(img)
            .write_to(&mut Cursor::new(&mut out), ImageFormat::Png)
            .unwrap();
        out
    }

    fn dimensions(png: &[u8]) -> (u32, u32) {
        let img = image::load_from_memory_with_format(png, ImageFormat::Png).unwrap();
        (img.width(), img.height())
    }

    #[test]
    fn oversized_image_is_downscaled() {
        let (out, aspect) = prepare_image(&png(4000, 2000)).unwrap();
        assert_eq!(dimensions(&out), (1600, 800));
        assert!((aspect - 0.5).abs() < 1e-6, "{aspect}");
    }

    #[test]
    fn small_image_keeps_its_size_and_becomes_png() {
        let mut jpeg = Vec::new();
        DynamicImage::ImageRgb8(image::RgbImage::new(30, 90))
            .write_to(&mut Cursor::new(&mut jpeg), ImageFormat::Jpeg)
            .unwrap();
        let (out, aspect) = prepare_image(&jpeg).unwrap();
        assert_eq!(image::guess_format(&out).unwrap(), ImageFormat::Png);
        assert_eq!(dimensions(&out), (30, 90));
        assert!((aspect - 3.0).abs() < 1e-6);
    }

    #[test]
    fn broken_and_huge_inputs_are_refused() {
        assert_eq!(
            prepare_image(b"not an image at all"),
            Err(ImageError::Unsupported)
        );
        let whole = png(64, 64);
        assert_eq!(
            prepare_image(&whole[..whole.len() / 2]),
            Err(ImageError::Corrupt)
        );
        // Too wide: refused from the header, before any pixels are allocated.
        assert_eq!(prepare_image(&png(13_000, 1)), Err(ImageError::TooLarge));
        let big = vec![0u8; MAX_INPUT_BYTES as usize + 1];
        assert_eq!(prepare_image(&big), Err(ImageError::TooLarge));
    }

    #[test]
    fn error_texts_exist() {
        for e in [
            ImageError::TooLarge,
            ImageError::Unsupported,
            ImageError::Corrupt,
        ] {
            assert!(!t(e.key()).starts_with("image."), "missing ftl entry");
        }
        assert!(!t("image.missing").starts_with("image."));
    }

    #[test]
    fn cache_marks_missing_and_corrupt_assets_and_evicts_closed_tabs() {
        let ctx = egui::Context::default();
        let mut tab = DocTab::new();
        let root = tab.doc.root;
        let good = tab.assets.insert("png", png(8, 4));
        let bad = tab.assets.insert("png", b"garbage".to_vec());
        let doc = &mut tab.doc;
        let p = doc
            .add_card(Parent::Box(root), CardKind::Reason, "")
            .unwrap()
            .1;
        let q = doc
            .add_card(Parent::Box(root), CardKind::Reason, "")
            .unwrap()
            .1;
        let image = |asset: &str| {
            Some(ImageRef {
                asset: asset.into(),
                aspect: 0.5,
            })
        };
        doc.set_image(root, image(&good)).unwrap();
        doc.set_image(p, image(&bad)).unwrap();
        doc.set_image(q, image("missing.png")).unwrap();

        let mut cache = ImageCache::default();
        cache.load(&ctx, &tab.doc, &tab.assets);
        assert_eq!(cache.texture(&good).unwrap().size(), [8, 4]);
        assert!(cache.texture(&bad).is_none());
        assert!(cache.texture("missing.png").is_none());
        assert_eq!(cache.len(), 2, "a missing asset is not cached");

        cache.evict_closed(std::slice::from_ref(&tab));
        assert_eq!(cache.len(), 2, "its tab is still open");
        cache.evict_closed(&[DocTab::new()]);
        assert!(cache.is_empty());
    }

    #[test]
    fn image_sits_in_the_reserved_block_above_the_content() {
        let world = Rect::from_min_size(pos2(10.0, 20.0), egui::vec2(212.0, 300.0));
        let img = ImageRef {
            asset: "a.png".into(),
            aspect: 0.5,
        };
        let r = image_rect(world, &img);
        assert_eq!(r.min, pos2(16.0, 26.0));
        assert_eq!(r.width(), 200.0);
        assert_eq!(r.height(), 100.0);
        let mut node = BoxNode::new(NodeId::new(), BoxKind::Premise, "");
        node.image = Some(img);
        let rest = content_rect(world, &node);
        assert_eq!(rest.min.y, r.max.y + IMAGE_GAP);
        assert_eq!(rest.max, world.max);
    }
}
