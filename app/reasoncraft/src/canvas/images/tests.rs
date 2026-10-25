//! Tests of preparing, caching and placing box images.

use std::io::Cursor;

use eframe::egui::{self, Rect, pos2};
use image::{DynamicImage, ImageFormat};
use reasoncraft_core::layout::IMAGE_GAP;
use reasoncraft_core::{BoxKind, BoxNode, CardKind, ImageRef, NodeId, Parent};

use super::*;
use crate::doc_tab::DocTab;
use crate::i18n::t;

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
