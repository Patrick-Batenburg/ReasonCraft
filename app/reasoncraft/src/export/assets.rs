//! Box images as exports embed them: always PNG.

use std::borrow::Cow;
use std::io::Cursor;

use image::ImageFormat;

use super::svg::PNG_SIGNATURE;
use crate::canvas::images::decode_stored;

/// `asset` as PNG bytes: a PNG unchanged, a JPEG or WebP decoded (under
/// the canvas's limits, so exports refuse what the canvas refuses) and
/// re-encoded. `None` for anything else, which exports as the
/// missing-image placeholder. Decoding is slow: call it on the export
/// worker, never on the UI thread.
pub fn export_png_bytes(asset: &[u8]) -> Option<Cow<'_, [u8]>> {
    if asset.starts_with(PNG_SIGNATURE) {
        return Some(Cow::Borrowed(asset));
    }
    match image::guess_format(asset).ok()? {
        ImageFormat::Jpeg | ImageFormat::WebP => {}
        _ => return None,
    }
    let img = decode_stored(asset)?;
    let mut png = Vec::new();
    img.write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .ok()?;
    Some(Cow::Owned(png))
}
