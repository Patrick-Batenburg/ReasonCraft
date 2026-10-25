//! The texture cache the canvas draws box images from.

use std::collections::{HashMap, HashSet};

use eframe::egui::{self, ColorImage, TextureFilter, TextureHandle, TextureOptions};
use image::imageops::FilterType;
use reasoncraft_core::{Assets, Document};

use super::prepare::decode;
use crate::doc_tab::DocTab;

/// Stored assets wider or taller than this are not drawn: this build only
/// stores images of at most [`super::MAX_SIDE`] px, so a larger one comes from a
/// crafted file and would stall the UI thread. Exports refuse them too.
pub const MAX_STORED_SIDE: u32 = 2048;
/// The most decoding a stored asset may allocate.
const MAX_STORED_ALLOC: u64 = MAX_STORED_SIDE as u64 * MAX_STORED_SIDE as u64 * 4 * 2;
/// At most this many images are decoded per frame; the rest follow in the
/// next frames, so opening a map with many images never stalls one frame.
const LOADS_PER_FRAME: usize = 2;
/// Textures are scaled down so their longest side is at most this: a box
/// never shows an image larger, and exports read the full-size asset.
pub const MAX_TEXTURE_SIDE: u32 = 1024;

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
