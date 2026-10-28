//! Sizes and places the scene and the canvas share: fonts, the parking
//! frame and where a box's image and text go. Here, not in the canvas, so
//! the scene (what is drawn) does not depend on the canvas (how it is
//! shown on screen).

use eframe::egui::{FontId, Rect, vec2};
use reasoncraft_core::layout::{BOX_TEXT_PAD, IMAGE_GAP, image_height};
use reasoncraft_core::{BoxKind, BoxNode, ImageRef, Layout};

/// The corner radius of the parking frame (world units).
pub const FRAME_RADIUS: f32 = 10.0;
const PARKING_PAD: f32 = 24.0;
const PARKING_HEADING: f32 = 26.0;
/// Text smaller than this many screen px is not drawn.
const MIN_TEXT_PX: f32 = 3.0;
/// Box text size at 100% zoom.
pub const BOX_FONT_SIZE: f32 = 13.0;
/// Citation text size in source boxes at 100% zoom.
pub const SOURCE_FONT_SIZE: f32 = 11.5;
/// The font box text of `kind` is measured and painted with, at 100% zoom.
pub fn font_for(kind: &BoxKind) -> FontId {
    match kind {
        BoxKind::Source { .. } => FontId::proportional(SOURCE_FONT_SIZE),
        _ => FontId::proportional(BOX_FONT_SIZE),
    }
}

/// The dashed frame around the parking area, heading included (world units).
pub fn parking_frame(layout: &Layout) -> Option<Rect> {
    layout.parking_bounds.map(|b| {
        Rect::from_min_max(
            b.min - vec2(PARKING_PAD, PARKING_PAD + PARKING_HEADING),
            b.max + vec2(PARKING_PAD, PARKING_PAD),
        )
    })
}

/// Screen font size for a world size, rounded down to half pixels so zooming
/// does not rasterise glyphs at every possible size. Rounding down keeps
/// rows inside their box.
pub fn screen_font(size: f32, zoom: f32) -> Option<FontId> {
    let px = (size * zoom * 2.0).floor() / 2.0;
    (px >= MIN_TEXT_PX).then(|| FontId::proportional(px))
}

/// Where `image` is drawn in a box at `world` (world units).
pub fn image_rect(world: Rect, image: &ImageRef) -> Rect {
    let width = world.width() - 2.0 * BOX_TEXT_PAD;
    let height = image_height(world.width(), image) - BOX_TEXT_PAD - IMAGE_GAP;
    Rect::from_min_size(
        world.min + vec2(BOX_TEXT_PAD, BOX_TEXT_PAD),
        vec2(width, height),
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
