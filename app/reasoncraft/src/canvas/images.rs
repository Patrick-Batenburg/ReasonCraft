//! Images in boxes: turning a picked file into the PNG that is stored, the
//! texture cache the canvas draws from, and painting an image (or its
//! placeholder) in the block the layout reserves above a box's text.

use std::collections::{HashMap, HashSet};
use std::io::Cursor;

use eframe::egui::{
    self, Align2, ColorImage, Painter, Rect, Shape, Stroke, TextureFilter, TextureHandle,
    TextureOptions, pos2,
};
use image::imageops::FilterType;
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits};
use reasoncraft_core::layout::{BOX_TEXT_PAD, IMAGE_GAP, image_height};
use reasoncraft_core::{Assets, BoxNode, Document, ImageRef};

use super::paint::screen_font;
use crate::doc_tab::DocTab;
use crate::i18n::t;
use crate::theme::Tokens;

/// Larger picked files are refused before they are decoded.
pub const MAX_INPUT_BYTES: u64 = 40 * 1024 * 1024;
/// Wider or taller images are refused before their pixels are allocated.
pub const MAX_DIMENSION: u32 = 12_000;
/// The most a decoder may allocate.
const MAX_ALLOC: u64 = 512 * 1024 * 1024;
/// Stored assets wider or taller than this are not drawn: this build only
/// stores images of at most [`MAX_SIDE`] px, so a larger one comes from a
/// crafted file and would stall the UI thread.
const MAX_STORED_SIDE: u32 = 2048;
/// The most decoding a stored asset may allocate.
const MAX_STORED_ALLOC: u64 = MAX_STORED_SIDE as u64 * MAX_STORED_SIDE as u64 * 4 * 2;
/// At most this many images are decoded per frame; the rest follow in the
/// next frames, so opening a map with many images never stalls one frame.
const LOADS_PER_FRAME: usize = 2;
/// Stored images are scaled down so their longest side is at most this.
pub const MAX_SIDE: u32 = 1600;
/// Textures are scaled down so their longest side is at most this: a box
/// never shows an image larger, and exports read the full-size asset.
pub const MAX_TEXTURE_SIDE: u32 = 1024;
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

/// Decode `bytes` (any enabled format), upright per its EXIF orientation.
/// Images wider or taller than `max_side`, or needing more than
/// `max_alloc` bytes, are refused from their header, before their pixels
/// are allocated.
fn decode(bytes: &[u8], max_side: u32, max_alloc: u64) -> Result<DynamicImage, ImageError> {
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| ImageError::Corrupt)?;
    if reader.format().is_none() {
        return Err(ImageError::Unsupported);
    }
    let mut limits = Limits::default();
    limits.max_image_width = Some(max_side);
    limits.max_image_height = Some(max_side);
    limits.max_alloc = Some(max_alloc);
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
    let img = decode(bytes, MAX_DIMENSION, MAX_ALLOC)?;
    let aspect = img.height() as f32 / img.width() as f32;
    let mut png = Vec::new();
    fit(img)
        .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .map_err(|_| ImageError::Corrupt)?;
    Ok((png, aspect))
}

/// A stored asset as egui pixels, scaled down (Triangle) to at most
/// [`MAX_TEXTURE_SIDE`] px on the longest side; `None` when it does not
/// decode or is larger than this build ever stores.
fn color_image(bytes: &[u8]) -> Option<ColorImage> {
    let mut img = decode(bytes, MAX_STORED_SIDE, MAX_STORED_ALLOC).ok()?;
    if img.width().max(img.height()) > MAX_TEXTURE_SIDE {
        img = img.resize(MAX_TEXTURE_SIDE, MAX_TEXTURE_SIDE, FilterType::Triangle);
    }
    let rgba = img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    Some(ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()))
}

/// Texture options: linear, with mipmaps so zoomed-out images stay smooth.
const TEXTURE_OPTIONS: TextureOptions =
    TextureOptions::LINEAR.with_mipmap_mode(Some(TextureFilter::Linear));

/// What the canvas draws for a box's image.
#[derive(Clone, Copy)]
pub enum ImageState<'a> {
    /// The decoded image.
    Ready(&'a TextureHandle),
    /// The asset is there but waits for a later frame's decode: a plain
    /// block, so the "missing" placeholder never flashes.
    Pending,
    /// The asset is absent, or refused (does not decode, too large).
    Missing,
}

/// Textures of the images on screen, keyed by asset key and shared by all
/// tabs (equal keys are equal bytes). `None` marks an asset that does not
/// decode. The canvas shows a placeholder for it and for a missing asset.
#[derive(Default)]
pub struct ImageCache {
    textures: HashMap<String, Option<TextureHandle>>,
    /// Assets present but not decoded yet (deferred by the per-frame cap),
    /// as of the last [`ImageCache::load`].
    pending: HashSet<String>,
}

impl ImageCache {
    /// Decode, once, the images `doc` shows that are not cached yet, at
    /// most [`LOADS_PER_FRAME`] of them; returns true when more are left
    /// for the next frame. A missing asset is not cached, so it loads once
    /// its bytes arrive (from another tab, say); one that does not decode
    /// is cached as `None`.
    pub fn load(&mut self, ctx: &egui::Context, doc: &Document, assets: &Assets) -> bool {
        let mut loaded = 0;
        self.pending.clear();
        for img in doc.boxes.values().filter_map(|b| b.image.as_ref()) {
            if self.textures.contains_key(&img.asset) {
                continue;
            }
            let Some(bytes) = assets.get(&img.asset) else {
                continue;
            };
            if loaded == LOADS_PER_FRAME {
                self.pending.insert(img.asset.clone());
                continue;
            }
            loaded += 1;
            let texture = color_image(&bytes).map(|pixels| {
                let name = format!("asset:{}", img.asset);
                ctx.load_texture(name, pixels, TEXTURE_OPTIONS)
            });
            self.textures.insert(img.asset.clone(), texture);
        }
        !self.pending.is_empty()
    }

    /// What to draw for asset `key`.
    pub fn state(&self, key: &str) -> ImageState<'_> {
        match self.textures.get(key) {
            Some(Some(texture)) => ImageState::Ready(texture),
            Some(None) => ImageState::Missing,
            None if self.pending.contains(key) => ImageState::Pending,
            None => ImageState::Missing,
        }
    }

    /// Drop the textures no open document shows (its tab closed, its box
    /// deleted or its insert undone).
    pub fn evict_unused(&mut self, tabs: &[DocTab]) {
        let used: HashSet<&str> = tabs
            .iter()
            .flat_map(|tab| tab.doc.boxes.values())
            .filter_map(|b| b.image.as_ref().map(|i| i.asset.as_str()))
            .collect();
        self.textures.retain(|key, _| used.contains(key.as_str()));
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

/// Asset `asset`'s image in screen rect `r`: a plain block while it waits
/// to be decoded, a dashed placeholder when it is missing or broken.
pub fn paint_image(
    painter: &Painter,
    r: Rect,
    asset: &str,
    images: &ImageCache,
    tk: &Tokens,
    zoom: f32,
) {
    match images.state(asset) {
        ImageState::Ready(texture) => {
            let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
            painter.image(texture.id(), r, uv, egui::Color32::WHITE);
            return;
        }
        ImageState::Pending => {
            painter.rect_filled(r, 0.0, tk.panel);
            return;
        }
        ImageState::Missing => {}
    }
    let colour = tk.text_muted;
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
        painter.extend(Shape::dashed_line(&edge, stroke, dash, gap));
    }
    if let Some(font) = screen_font(PLACEHOLDER_TEXT, zoom) {
        painter.text(
            r.center(),
            Align2::CENTER_CENTER,
            t("image.missing"),
            font,
            colour,
        );
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
    fn cache_marks_missing_and_corrupt_assets_and_evicts_unused() {
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
        assert!(
            !cache.load(&ctx, &tab.doc, &tab.assets),
            "two loads fit a frame"
        );
        assert_eq!(cache.texture(&good).unwrap().size(), [8, 4]);
        assert!(cache.texture(&bad).is_none());
        assert!(cache.texture("missing.png").is_none());
        assert_eq!(cache.len(), 2, "a missing asset is not cached");

        cache.evict_unused(std::slice::from_ref(&tab));
        assert_eq!(cache.len(), 2, "its tab is still open");
        tab.doc.set_image(p, None).unwrap();
        cache.evict_unused(std::slice::from_ref(&tab));
        assert_eq!(cache.len(), 1, "no box shows the broken asset any more");
        cache.evict_unused(&[DocTab::new()]);
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

    /// A PNG whose header claims `w`×`h` pixels, with almost no data:
    /// big dimensions without the cost of encoding them.
    fn png_header_only(w: u32, h: u32) -> Vec<u8> {
        fn crc32(data: &[u8]) -> u32 {
            let mut crc = !0u32;
            for &b in data {
                crc ^= u32::from(b);
                for _ in 0..8 {
                    crc = if crc & 1 == 1 {
                        (crc >> 1) ^ 0xEDB8_8320
                    } else {
                        crc >> 1
                    };
                }
            }
            !crc
        }
        fn chunk(out: &mut Vec<u8>, kind: &[u8], data: &[u8]) {
            out.extend((data.len() as u32).to_be_bytes());
            let mut body = kind.to_vec();
            body.extend(data);
            out.extend(&body);
            out.extend(crc32(&body).to_be_bytes());
        }
        let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
        let mut ihdr = Vec::new();
        ihdr.extend(w.to_be_bytes());
        ihdr.extend(h.to_be_bytes());
        ihdr.extend([8, 2, 0, 0, 0]);
        chunk(&mut out, b"IHDR", &ihdr);
        chunk(
            &mut out,
            b"IDAT",
            &[0x78, 0x9c, 0x03, 0x00, 0x00, 0x00, 0x00, 0x01],
        );
        chunk(&mut out, b"IEND", &[]);
        out
    }

    #[test]
    fn cache_refuses_assets_larger_than_this_build_stores() {
        let ctx = egui::Context::default();
        let mut tab = DocTab::new();
        let root = tab.doc.root;
        let tall = tab.assets.insert("png", png(1, 100_000));
        let huge = tab.assets.insert("png", png_header_only(12_000, 12_000));
        // The largest size this build stores.
        let ok = tab.assets.insert("png", png(1600, 1));
        let doc = &mut tab.doc;
        let mut boxes = vec![root];
        for _ in 0..2 {
            let (_, b) = doc
                .add_card(Parent::Box(root), CardKind::Reason, "")
                .unwrap();
            boxes.push(b);
        }
        for (b, asset) in boxes.into_iter().zip([&tall, &huge, &ok]) {
            let image = ImageRef {
                asset: asset.clone(),
                aspect: 1.0,
            };
            doc.set_image(b, Some(image)).unwrap();
        }

        let mut cache = ImageCache::default();
        let started = std::time::Instant::now();
        assert!(
            cache.load(&ctx, &tab.doc, &tab.assets),
            "a third load waits"
        );
        assert_eq!(cache.len(), 2);
        assert!(!cache.load(&ctx, &tab.doc, &tab.assets));
        assert!(started.elapsed() < std::time::Duration::from_secs(2));
        assert!(cache.texture(&tall).is_none(), "placeholder");
        assert!(cache.texture(&huge).is_none(), "placeholder");
        assert_eq!(cache.texture(&ok).unwrap().size(), [1024, 1]);
        assert_eq!(cache.len(), 3);
    }

    #[test]
    fn textures_are_scaled_down_to_the_texture_limit() {
        let ctx = egui::Context::default();
        let mut tab = DocTab::new();
        let root = tab.doc.root;
        let big = tab.assets.insert("png", png(1600, 1200));
        let image = ImageRef {
            asset: big.clone(),
            aspect: 0.75,
        };
        tab.doc.set_image(root, Some(image)).unwrap();
        let mut cache = ImageCache::default();
        assert!(!cache.load(&ctx, &tab.doc, &tab.assets));
        let [w, h] = cache.texture(&big).unwrap().size();
        assert!(w.max(h) <= MAX_TEXTURE_SIDE as usize, "{w}x{h}");
        assert_eq!([w, h], [1024, 768]);
        assert_eq!(
            &*tab.assets.get(&big).unwrap(),
            &png(1600, 1200)[..],
            "asset kept full size"
        );
    }

    #[test]
    fn image_state_is_pending_only_while_a_present_asset_waits() {
        let ctx = egui::Context::default();
        let mut tab = DocTab::new();
        let root = tab.doc.root;
        let keys: Vec<String> = (1..=3)
            .map(|n| tab.assets.insert("png", png(n, 1)))
            .collect();
        let bad = tab.assets.insert("png", b"garbage".to_vec());
        let doc = &mut tab.doc;
        let mut boxes = vec![root];
        for _ in 0..4 {
            boxes.push(
                doc.add_card(Parent::Box(root), CardKind::Reason, "")
                    .unwrap()
                    .1,
            );
        }
        let assets = keys
            .iter()
            .cloned()
            .chain([bad.clone(), "absent.png".into()]);
        for (b, asset) in boxes.into_iter().zip(assets) {
            doc.set_image(b, Some(ImageRef { asset, aspect: 1.0 }))
                .unwrap();
        }

        let mut cache = ImageCache::default();
        let state = |c: &ImageCache, k: &str| match c.state(k) {
            ImageState::Ready(_) => "ready",
            ImageState::Pending => "pending",
            ImageState::Missing => "missing",
        };
        assert_eq!(state(&cache, &keys[0]), "missing", "never loaded");
        assert!(cache.load(&ctx, &tab.doc, &tab.assets), "more left");
        let all: Vec<&str> = keys
            .iter()
            .chain([&bad])
            .map(|k| state(&cache, k))
            .collect();
        assert_eq!(
            all.iter().filter(|s| **s == "pending").count(),
            2,
            "{all:?}"
        );
        assert_eq!(state(&cache, "absent.png"), "missing");
        while cache.load(&ctx, &tab.doc, &tab.assets) {}
        for k in &keys {
            assert_eq!(state(&cache, k), "ready");
        }
        assert_eq!(state(&cache, &bad), "missing", "refused");
        assert_eq!(state(&cache, "absent.png"), "missing");
    }
}
