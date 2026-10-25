//! A map's SVG rasterised to PNG with resvg, and put on the clipboard as
//! an image.

use std::sync::Arc;

use image::ImageFormat;
use resvg::tiny_skia::{Color, Pixmap, Transform};
use resvg::usvg::{self, fontdb};

use super::ExportError;

/// The longest side of an exported image, in pixels.
pub const MAX_SIDE_PX: u32 = 16_384;
/// The most pixels an exported image may have.
pub const MAX_PIXELS: u64 = 256_000_000;

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
pub struct SystemClipboard;

impl ClipboardSink for SystemClipboard {
    fn set_image(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<(), ExportError> {
        let mut board =
            arboard::Clipboard::new().map_err(|e| ExportError::Clipboard(e.to_string()))?;
        board
            .set_image(arboard::ImageData {
                width: width as usize,
                height: height as usize,
                bytes: rgba.into(),
            })
            .map_err(|e| ExportError::Clipboard(e.to_string()))
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
pub fn copy_png_to_clipboard(png_rgba: (u32, u32, Vec<u8>)) -> Result<(), ExportError> {
    copy_image(&mut SystemClipboard, png_rgba)
}
