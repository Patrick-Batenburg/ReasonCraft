//! Images in boxes: turning a picked file into the PNG that is stored (on
//! a worker thread), the texture cache the canvas draws from, and painting
//! an image (or its placeholder) in the block the layout reserves above a
//! box's text.

mod cache;
mod job;
mod paint;
mod prepare;
#[cfg(test)]
mod tests;

pub use cache::{ImageCache, ImageState, MAX_TEXTURE_SIDE};
pub use job::{ImageJobs, ImageResult, Prepared};
pub use paint::{content_rect, image_rect, paint_image, preparing};
pub use prepare::{
    FailureKind, ImageError, ImageFailure, MAX_DIMENSION, MAX_INPUT_BYTES, MAX_SIDE, PreparedImage,
    prepare_file, prepare_image,
};
