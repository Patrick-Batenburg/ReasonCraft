//! Turning a picked file into the PNG a box stores. Runs on a worker
//! thread (see [`super::ImageJobs`]); also decodes for the texture cache.

use std::fs::File;
use std::io::{Cursor, Read};
use std::path::Path;

use image::imageops::FilterType;
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits};

use crate::i18n::{FluentArgs, t, t_args};

/// Larger picked files are refused before they are decoded.
pub const MAX_INPUT_BYTES: u64 = 40 * 1024 * 1024;
/// Wider or taller images are refused before their pixels are allocated.
pub const MAX_DIMENSION: u32 = 12_000;
/// The most preparing an image may allocate: the decoded pixels and the
/// scaling's buffers ([`FIT_SCRATCH`]).
const MAX_ALLOC: u64 = 512 * 1024 * 1024;
/// Stored images are scaled down so their longest side is at most this.
pub const MAX_SIDE: u32 = 1600;
/// Larger images are first shrunk (by averaging, in 8 bits) to this
/// longest side, so the Lanczos pass works on a small image.
const SHRINK_SIDE: u32 = 2 * MAX_SIDE;
/// The most [`fit`] allocates beside the decoded image: the shrunk copy
/// (RGBA, 8 bits per channel) and Lanczos's intermediate (one pass at
/// full width, RGBA in `f32`) and result.
const FIT_SCRATCH: u64 = {
    let (s, m) = (SHRINK_SIDE as u64, MAX_SIDE as u64);
    s * s * 4 + s * m * 16 + m * m * 4
};

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
/// `max_alloc` bytes besides `scratch` (what the caller allocates for its
/// work on them), are refused from their header, before their pixels are
/// allocated.
pub(super) fn decode(
    bytes: &[u8],
    max_side: u32,
    max_alloc: u64,
    scratch: u64,
) -> Result<DynamicImage, ImageError> {
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
        .reserve(decoder.total_bytes().saturating_add(scratch))
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

/// `img` scaled down (Lanczos3) so its longest side is at most
/// [`MAX_SIDE`]; a very large one is first shrunk to [`SHRINK_SIDE`], so
/// the scaling never needs more than [`FIT_SCRATCH`] bytes.
fn fit(mut img: DynamicImage) -> DynamicImage {
    let longest = img.width().max(img.height());
    if longest > SHRINK_SIDE {
        img = img.thumbnail(SHRINK_SIDE, SHRINK_SIDE);
    }
    if longest > MAX_SIDE {
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
    let img = decode(bytes, MAX_DIMENSION, MAX_ALLOC, FIT_SCRATCH)?;
    let aspect = img.height() as f32 / img.width() as f32;
    let mut png = Vec::new();
    fit(img)
        .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .map_err(|_| ImageError::Corrupt)?;
    Ok((png, aspect))
}

/// A picked image, ready for a box: PNG bytes and the aspect (height
/// divided by width).
#[derive(Clone, PartialEq)]
pub struct PreparedImage {
    pub png: Vec<u8>,
    pub aspect: f32,
}

impl std::fmt::Debug for PreparedImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedImage")
            .field("png", &format_args!("{} bytes", self.png.len()))
            .field("aspect", &self.aspect)
            .finish()
    }
}

/// Why a picked file could not be used.
#[derive(Clone, Debug, PartialEq)]
pub enum FailureKind {
    /// The file could not be read (the OS's message).
    Read(String),
    Image(ImageError),
    /// The worker stopped without a result.
    Stopped,
}

/// A picked file that could not be used, and why.
#[derive(Clone, Debug, PartialEq)]
pub struct ImageFailure {
    /// The file's name, for the message.
    pub file: String,
    pub kind: FailureKind,
}

impl ImageFailure {
    /// "Could not use <file>".
    pub fn title(&self) -> String {
        let mut args = FluentArgs::new();
        args.set("file", self.file.clone());
        t_args("image.error.title", &args)
    }

    /// What went wrong, for the user.
    pub fn message(&self) -> String {
        match &self.kind {
            FailureKind::Image(e) => t(e.key()),
            FailureKind::Read(detail) => {
                let mut args = FluentArgs::new();
                args.set("detail", detail.clone());
                t_args("image.error.read", &args)
            }
            FailureKind::Stopped => t("image.error.stopped"),
        }
    }
}

/// The name `path` is shown by in messages.
pub fn file_label(path: &Path) -> String {
    let file = path.file_name().unwrap_or(path.as_os_str());
    file.to_string_lossy().into_owned()
}

/// Read and prepare the image at `path`. Reading stops one byte past the
/// size limit, so a huge file is refused without being read whole.
pub fn prepare_file(path: &Path) -> Result<PreparedImage, ImageFailure> {
    let fail = |kind| ImageFailure {
        file: file_label(path),
        kind,
    };
    let mut bytes = Vec::new();
    File::open(path)
        .and_then(|f| f.take(MAX_INPUT_BYTES + 1).read_to_end(&mut bytes))
        .map_err(|e| fail(FailureKind::Read(e.to_string())))?;
    let (png, aspect) = prepare_image(&bytes).map_err(|e| fail(FailureKind::Image(e)))?;
    Ok(PreparedImage { png, aspect })
}
