//! A map's SVG rasterised to PNG with resvg, and put on the clipboard as
//! an image.

use std::sync::Arc;

use image::ImageFormat;
use resvg::tiny_skia::{Color, Pixmap, Transform};
use resvg::usvg::{self, fontdb};

use reasoncraft_core::Assets;

use super::svg::{SvgOptions, scene_to_svg, svg_size};
use super::{ExportError, bundled_font_family};
use crate::scene::Scene;
use crate::theme::Tokens;

/// The longest side of an exported image, in pixels.
pub const MAX_SIDE_PX: u32 = 16_384;
/// The most pixels an exported image may have.
pub const MAX_PIXELS: u64 = 256_000_000;

/// The most pixels an image put on the clipboard may have. The clipboard
/// holds straight RGBA and the OS copies it again, so this stays well
/// under [`MAX_PIXELS`] (64 MP is about 256 MB per copy).
pub const MAX_CLIPBOARD_PIXELS: u64 = 64_000_000;

/// How an SVG is rasterised.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PngOptions {
    /// Pixels per SVG px (1, 2, 3 or 4).
    pub scale: f32,
    /// Keep what the SVG does not paint transparent (leave the background
    /// out of the SVG too). Otherwise the image is composed on white.
    pub transparent: bool,
}

impl Default for PngOptions {
    fn default() -> Self {
        Self {
            scale: 2.0,
            transparent: false,
        }
    }
}

/// The pixel size of an image of `width` x `height` SVG px at `scale`, or
/// `TooLarge` when it is over the caps. Nothing is allocated here.
pub(super) fn pixel_size(width: f32, height: f32, scale: f32) -> Result<(u32, u32), ExportError> {
    if !(scale.is_finite() && scale > 0.0 && width.is_finite() && height.is_finite()) {
        return Err(ExportError::Render("invalid size or scale".into()));
    }
    let w = (f64::from(width) * f64::from(scale)).ceil().max(1.0);
    let h = (f64::from(height) * f64::from(scale)).ceil().max(1.0);
    let side = f64::from(MAX_SIDE_PX);
    if w > side || h > side || w * h > MAX_PIXELS as f64 {
        return Err(ExportError::TooLarge {
            w: w.min(f64::from(u32::MAX)) as u32,
            h: h.min(f64::from(u32::MAX)) as u32,
        });
    }
    Ok((w as u32, h as u32))
}

/// `svg` as PNG bytes at `opts.scale`, with text drawn from `fonts`.
/// Fails with `TooLarge` before allocating when the image would pass
/// [`MAX_SIDE_PX`] or [`MAX_PIXELS`].
pub fn svg_to_png(
    svg: &str,
    opts: &PngOptions,
    fonts: &fontdb::Database,
) -> Result<Vec<u8>, ExportError> {
    let pixmap = render(svg, opts, fonts)?;
    pixmap
        .encode_png()
        .map_err(|e| ExportError::Render(e.to_string()))
}

/// `svg` as straight (non-premultiplied) RGBA pixels with width and height.
pub fn svg_to_rgba(
    svg: &str,
    opts: &PngOptions,
    fonts: &fontdb::Database,
) -> Result<(u32, u32, Vec<u8>), ExportError> {
    let pixmap = render(svg, opts, fonts)?;
    let (w, h) = (pixmap.width(), pixmap.height());
    let mut data = pixmap.take();
    for px in data.as_chunks_mut::<4>().0 {
        let a = u32::from(px[3]);
        if a != 0 && a != 255 {
            for c in &mut px[..3] {
                *c = ((u32::from(*c) * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
    Ok((w, h, data))
}

fn render(svg: &str, opts: &PngOptions, fonts: &fontdb::Database) -> Result<Pixmap, ExportError> {
    let usvg_opts = usvg::Options {
        fontdb: Arc::new(fonts.clone()),
        ..Default::default()
    };
    let tree =
        usvg::Tree::from_str(svg, &usvg_opts).map_err(|e| ExportError::Render(e.to_string()))?;
    let size = tree.size();
    let (w, h) = pixel_size(size.width(), size.height(), opts.scale)?;
    let mut pixmap = Pixmap::new(w, h).ok_or_else(|| ExportError::Render("empty image".into()))?;
    if !opts.transparent {
        pixmap.fill(Color::WHITE);
    }
    resvg::render(
        &tree,
        Transform::from_scale(opts.scale, opts.scale),
        &mut pixmap.as_mut(),
    );
    Ok(pixmap)
}

/// Decodes PNG bytes to straight RGBA pixels.
pub fn png_to_rgba(png: &[u8]) -> Result<(u32, u32, Vec<u8>), ExportError> {
    let img = image::load_from_memory_with_format(png, ImageFormat::Png)
        .map_err(|e| ExportError::Render(e.to_string()))?
        .into_rgba8();
    Ok((img.width(), img.height(), img.into_raw()))
}

/// Somewhere an image can be put for other programs to paste.
pub trait ClipboardSink {
    fn set_image(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<(), ExportError>;
}

/// The system clipboard. Only the running app uses this; tests use a fake.
///
/// It keeps one `arboard::Clipboard` for as long as it lives (the app
/// keeps it for its whole run). On Linux (X11, and Wayland through its
/// data-control protocol) the copied image is served by that object and
/// is gone as soon as it is dropped, unless a clipboard manager took a
/// copy; a clipboard made and dropped per copy would report success while
/// pasting gives nothing. `SetExtLinux::wait` is not used: it blocks until
/// another program takes over the clipboard. arboard's `Clipboard` is
/// `Send + Sync` on every platform, so the worker threads can use it.
#[derive(Default)]
pub struct SystemClipboard {
    board: Option<arboard::Clipboard>,
}

impl ClipboardSink for SystemClipboard {
    fn set_image(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<(), ExportError> {
        let board = match &mut self.board {
            Some(board) => board,
            empty => empty.insert(
                arboard::Clipboard::new().map_err(|e| ExportError::Clipboard(e.to_string()))?,
            ),
        };
        board
            .set_image(arboard::ImageData {
                width: width as usize,
                height: height as usize,
                bytes: rgba.into(),
            })
            .map_err(|e| ExportError::Clipboard(e.to_string()))
    }
}

/// An image as width, height and straight RGBA pixels.
pub type RgbaImage = (u32, u32, Vec<u8>);

/// A clipboard for tests: it keeps every image put on it. Clones share
/// what they hold, so a test keeps a clone to look at the app's copy.
#[derive(Clone, Default)]
pub struct FakeClipboard {
    pub images: Arc<std::sync::Mutex<Vec<RgbaImage>>>,
}

impl ClipboardSink for FakeClipboard {
    fn set_image(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<(), ExportError> {
        if let Ok(mut images) = self.images.lock() {
            images.push((width, height, rgba.to_vec()));
        }
        Ok(())
    }
}

/// Puts an RGBA image (width, height, pixels) on `sink`, checking that
/// the pixel buffer fits the size first.
pub fn copy_image(
    sink: &mut dyn ClipboardSink,
    (w, h, rgba): (u32, u32, Vec<u8>),
) -> Result<(), ExportError> {
    if rgba.len() as u64 != u64::from(w) * u64::from(h) * 4 {
        return Err(ExportError::Clipboard(
            "image data does not fit its size".into(),
        ));
    }
    sink.set_image(w, h, &rgba)
}

/// Puts an RGBA image on the system clipboard.
///
/// The clipboard object is dropped afterwards, so on Linux the image may
/// not outlive this call; the app keeps a [`SystemClipboard`] instead.
pub fn copy_png_to_clipboard(png_rgba: (u32, u32, Vec<u8>)) -> Result<(), ExportError> {
    copy_image(&mut SystemClipboard::default(), png_rgba)
}

/// `scene` in the colours of `tokens` as the live-text SVG a raster is
/// drawn from. Its background follows `opts.transparent`, so the PNG and
/// the SVG under it never disagree about transparency.
fn raster_svg(scene: &Scene, assets: &Assets, tokens: &Tokens, opts: &PngOptions) -> String {
    let svg_opts = SvgOptions {
        text_as_paths: false,
        transparent: opts.transparent,
        ..SvgOptions::new(tokens)
    };
    scene_to_svg(scene, assets, &svg_opts, &bundled_font_family())
}

/// `scene` (drawn at zoom 1) as PNG bytes in the colours of `tokens`.
pub fn scene_to_png(
    scene: &Scene,
    assets: &Assets,
    tokens: &Tokens,
    opts: &PngOptions,
    fonts: &fontdb::Database,
) -> Result<Vec<u8>, ExportError> {
    svg_to_png(&raster_svg(scene, assets, tokens, opts), opts, fonts)
}

/// Puts `scene` on `sink` as an image. Fails with `ClipboardTooLarge`
/// before drawing anything when it would pass [`MAX_CLIPBOARD_PIXELS`].
pub fn copy_scene_to_clipboard(
    scene: &Scene,
    assets: &Assets,
    tokens: &Tokens,
    opts: &PngOptions,
    fonts: &fontdb::Database,
    sink: &mut dyn ClipboardSink,
) -> Result<(), ExportError> {
    let (w, h) = svg_size(scene, &SvgOptions::new(tokens));
    let (pw, ph) = pixel_size(w, h, opts.scale)?;
    if u64::from(pw) * u64::from(ph) > MAX_CLIPBOARD_PIXELS {
        return Err(ExportError::ClipboardTooLarge { w: pw, h: ph });
    }
    let rgba = svg_to_rgba(&raster_svg(scene, assets, tokens, opts), opts, fonts)?;
    copy_image(sink, rgba)
}
