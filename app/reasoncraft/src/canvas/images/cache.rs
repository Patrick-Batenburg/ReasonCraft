//! The texture cache the canvas draws box images from.
//!
//! Only images of boxes on or near the screen get a texture, about the
//! size they are shown at ([`texture_side`]). Decoding runs on a worker
//! thread, one image at a time; the UI thread only uploads the result, a
//! few per frame. Textures that were not shown this frame are dropped,
//! least recently shown first, once together they pass the budget
//! ([`TEXTURE_BUDGET`]).

use std::collections::HashMap;

use eframe::egui::{self, ColorImage, Rect, TextureFilter, TextureHandle, TextureOptions};
use image::DynamicImage;
use image::imageops::FilterType;
use reasoncraft_core::{Assets, Document, Layout};

use super::prepare::decode;
use crate::canvas::Camera;
use crate::doc_tab::DocTab;
use crate::jobs::{Hold, Job, JobState, Queue};
use crate::scene::metrics::image_rect;

/// Stored assets wider or taller than this are not drawn: this build only
/// stores images of at most [`super::MAX_SIDE`] px, so a larger one comes from a
/// crafted file and would stall the worker. Exports refuse them too.
pub const MAX_STORED_SIDE: u32 = 2048;
/// The most decoding a stored asset may allocate.
const MAX_STORED_ALLOC: u64 = MAX_STORED_SIDE as u64 * MAX_STORED_SIDE as u64 * 4 * 2;
/// At most this many decoded images are uploaded per frame; the rest
/// follow in the next frames.
const UPLOADS_PER_FRAME: usize = 4;
/// Textures are scaled down so their longest side is at most this: a box
/// never shows an image larger, and exports read the full-size asset.
pub const MAX_TEXTURE_SIDE: u32 = 1024;
/// The smallest texture side asked for, so tiny boxes at a low zoom do not
/// each ask for a new texture as the zoom changes.
const MIN_TEXTURE_SIDE: u32 = 64;
/// The most texture memory (with mipmaps) the cache keeps for textures
/// that are not on screen: 256 MiB.
pub const TEXTURE_BUDGET: usize = 256 << 20;

/// A stored asset decoded, under the limits above; `None` when it does
/// not decode or is larger than this build ever stores. Exports decode
/// with it too, so they refuse what the canvas refuses.
pub fn decode_stored(bytes: &[u8]) -> Option<DynamicImage> {
    decode(bytes, MAX_STORED_SIDE, MAX_STORED_ALLOC, 0).ok()
}

/// At most `side` px on the longest side; `None` when it does not decode
/// or is larger than this build ever stores.
fn color_image(bytes: &[u8], side: u32) -> Option<ColorImage> {
    let mut img = decode_stored(bytes)?;
    if img.width().max(img.height()) > side {
        img = img.resize(side, side, FilterType::Triangle);
    }
    let rgba = img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    Some(ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()))
}

/// The texture side for an image shown `shown` px (its longest side, in
/// physical pixels): the next power of two, within the limits above.
pub fn texture_side(shown: f32) -> u32 {
    let px = shown.clamp(1.0, MAX_TEXTURE_SIDE as f32).ceil() as u32;
    px.next_power_of_two()
        .clamp(MIN_TEXTURE_SIDE, MAX_TEXTURE_SIDE)
}

/// The images of `doc` the canvas may show in `view` (and a screen around
/// it, so panning finds them ready), with the texture side each is shown
/// at, for [`ImageCache::load`].
pub fn wanted(
    doc: &Document,
    layout: &Layout,
    cam: &Camera,
    view: Rect,
    pixels_per_point: f32,
) -> Vec<(String, u32)> {
    let near = view.expand2(view.size());
    let shown = doc
        .boxes
        .values()
        .filter_map(|b| Some((b, b.image.as_ref()?)));
    shown
        .filter_map(|(b, img)| {
            let r = cam.world_rect_to_screen(image_rect(layout.body(b.id)?, img));
            near.intersects(r).then(|| {
                let side = texture_side(r.width().max(r.height()) * pixels_per_point);
                (img.asset.clone(), side)
            })
        })
        .collect()
}

/// Texture options: linear, with mipmaps so zoomed-out images stay smooth.
const TEXTURE_OPTIONS: TextureOptions =
    TextureOptions::LINEAR.with_mipmap_mode(Some(TextureFilter::Linear));

/// What the canvas draws for a box's image.
#[derive(Clone, Copy)]
pub enum ImageState<'a> {
    /// The decoded image.
    Ready(&'a TextureHandle),
    /// The asset is there but is being decoded: a plain block, so the
    /// "missing" placeholder never flashes.
    Pending,
    /// The asset is absent, or refused (does not decode, too large).
    Missing,
}

/// A cached texture (`None` for an asset that does not decode).
struct Entry {
    texture: Option<TextureHandle>,
    /// The side it was asked for (the image may be smaller).
    side: u32,
    /// Its memory, with mipmaps.
    bytes: usize,
    /// The frame it was last wanted in.
    shown: u64,
}

/// A decode on the worker.
struct Decoding {
    key: String,
    side: u32,
    job: Job<Option<ColorImage>>,
}

/// Textures of the images on screen, keyed by asset key and shared by all
/// tabs (equal keys are equal bytes). The canvas shows a placeholder for
/// an asset that does not decode, and for a missing one.
pub struct ImageCache {
    textures: HashMap<String, Entry>,
    /// The sides asked of the worker and not back yet, by asset.
    asked: HashMap<String, u32>,
    decoding: Vec<Decoding>,
    queue: Queue,
    /// Counts [`Self::load`] calls.
    frame: u64,
    /// The memory of all textures.
    bytes: usize,
    /// The most memory kept for textures not shown this frame.
    pub budget: usize,
    /// Hold each decode back until released (tests only).
    pub hold: Option<Hold>,
}

impl Default for ImageCache {
    fn default() -> Self {
        Self {
            textures: HashMap::new(),
            asked: HashMap::new(),
            decoding: Vec::new(),
            queue: Queue::default(),
            frame: 0,
            bytes: 0,
            budget: TEXTURE_BUDGET,
            hold: None,
        }
    }
}

impl ImageCache {
    /// One frame's work: upload what the worker decoded (at most
    /// [`UPLOADS_PER_FRAME`]), ask it for the `wanted` images (asset key and
    /// texture side) not cached at that side yet, and drop textures not
    /// wanted now while the cache is over its budget. A missing asset is
    /// not asked for, so it loads once its bytes arrive (from another tab,
    /// say). Returns true when this frame uploaded as many images as it
    /// may and more are outstanding: the next frame should come at once.
    /// Otherwise decodes still running are looked for by the frame loop's
    /// polling (see [`Self::busy`]).
    pub fn load(&mut self, ctx: &egui::Context, wanted: &[(String, u32)], assets: &Assets) -> bool {
        self.frame += 1;
        let more = self.upload(ctx);
        for (key, side) in wanted {
            let side = (*side).min(MAX_TEXTURE_SIDE);
            if let Some(e) = self.textures.get_mut(key) {
                e.shown = self.frame;
                if e.side >= side {
                    continue;
                }
            }
            if self.asked.get(key).is_some_and(|&s| s >= side) {
                continue;
            }
            let Some(bytes) = assets.get(key) else {
                continue;
            };
            let job = (self.queue).push(self.hold.clone(), move || color_image(&bytes, side));
            self.asked.insert(key.clone(), side);
            let key = key.clone();
            self.decoding.push(Decoding { key, side, job });
        }
        self.trim();
        more
    }

    /// Whether images are being decoded, or decoded and not uploaded yet.
    pub fn busy(&self) -> bool {
        !self.decoding.is_empty()
    }

    /// Upload the decoded images that are back, in the order asked.
    /// Returns true when it uploaded its share and more are outstanding.
    fn upload(&mut self, ctx: &egui::Context) -> bool {
        let mut uploaded = 0;
        let mut more = false;
        let mut still = Vec::new();
        for d in std::mem::take(&mut self.decoding) {
            if uploaded == UPLOADS_PER_FRAME {
                more = true;
                still.push(d);
                continue;
            }
            let image = match d.job.poll() {
                JobState::Running => None,
                JobState::Done(image) => Some(image),
                // The worker failed on it: show the placeholder.
                JobState::Lost => Some(None),
            };
            let Some(image) = image else {
                still.push(d);
                continue;
            };
            uploaded += 1;
            if self.asked.get(&d.key) == Some(&d.side) {
                self.asked.remove(&d.key);
            }
            self.store(ctx, d.key, d.side, image);
        }
        self.decoding = still;
        more
    }

    /// Keep `image`, decoded for `side`, unless a texture at least as
    /// large is cached already.
    fn store(&mut self, ctx: &egui::Context, key: String, side: u32, image: Option<ColorImage>) {
        if self.textures.get(&key).is_some_and(|e| e.side >= side) {
            return;
        }
        let bytes = image.as_ref().map_or(0, |i| i.pixels.len() * 4 * 4 / 3);
        let texture =
            image.map(|pixels| ctx.load_texture(format!("asset:{key}"), pixels, TEXTURE_OPTIONS));
        // A refused asset is never asked for again.
        let side = if texture.is_some() { side } else { u32::MAX };
        let entry = Entry {
            texture,
            side,
            bytes,
            shown: self.frame,
        };
        if let Some(old) = self.textures.insert(key, entry) {
            self.bytes -= old.bytes;
        }
        self.bytes += bytes;
    }

    /// Over budget, drop the textures not wanted this frame, least
    /// recently wanted first.
    fn trim(&mut self) {
        if self.bytes <= self.budget {
            return;
        }
        let mut idle: Vec<(u64, String)> = (self.textures.iter())
            .filter(|(_, e)| e.shown < self.frame && e.bytes > 0)
            .map(|(k, e)| (e.shown, k.clone()))
            .collect();
        idle.sort_unstable();
        for (_, key) in idle {
            if self.bytes <= self.budget {
                break;
            }
            if let Some(e) = self.textures.remove(&key) {
                self.bytes -= e.bytes;
            }
        }
    }

    /// What to draw for asset `key`.
    pub fn state(&self, key: &str) -> ImageState<'_> {
        match self.textures.get(key) {
            Some(Entry {
                texture: Some(texture),
                ..
            }) => ImageState::Ready(texture),
            Some(_) => ImageState::Missing,
            None if self.asked.contains_key(key) => ImageState::Pending,
            None => ImageState::Missing,
        }
    }

    /// Drop the textures no open document shows (its tab closed, its box
    /// deleted or its insert undone).
    pub fn evict_unused(&mut self, tabs: &[DocTab]) {
        let used: std::collections::HashSet<&str> = tabs
            .iter()
            .flat_map(|tab| tab.doc.boxes.values())
            .filter_map(|b| b.image.as_ref().map(|i| i.asset.as_str()))
            .collect();
        let bytes = &mut self.bytes;
        self.textures.retain(|key, e| {
            let keep = used.contains(key.as_str());
            if !keep {
                *bytes -= e.bytes;
            }
            keep
        });
    }

    pub fn texture(&self, key: &str) -> Option<&TextureHandle> {
        self.textures.get(key)?.texture.as_ref()
    }

    /// The memory the textures take, with mipmaps.
    pub fn bytes(&self) -> usize {
        self.bytes
    }

    pub fn len(&self) -> usize {
        self.textures.len()
    }

    pub fn is_empty(&self) -> bool {
        self.textures.is_empty()
    }
}
