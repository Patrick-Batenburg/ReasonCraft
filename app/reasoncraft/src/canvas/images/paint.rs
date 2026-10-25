//! Painting a box's image (or its placeholder) in the block the layout
//! reserves above the box's text, and the "Preparing image…" band.

use eframe::egui::{
    self, Align2, Painter, Rect, Role, Sense, Shape, Stroke, StrokeKind, WidgetInfo, pos2,
};
use reasoncraft_core::layout::{BOX_TEXT_PAD, IMAGE_GAP, image_height};
use reasoncraft_core::{BoxNode, ImageRef, Layout};

use super::{ImageCache, ImageState};
use crate::canvas::paint::screen_font;
use crate::doc_tab::DocTab;
use crate::i18n::t;
use crate::scene::missing_image;
use crate::theme::Tokens;

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
    let stroke = Stroke::new(missing_image::STROKE, colour);
    let (dash, gap) = (missing_image::DASH, missing_image::GAP);
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
    if let Some(font) = screen_font(missing_image::TEXT_SIZE, zoom) {
        painter.text(
            r.center(),
            Align2::CENTER_CENTER,
            missing_image::label(),
            font,
            colour,
        );
    }
}

/// Height of the "Preparing image…" band, in world units.
const PREPARING_HEIGHT: f32 = 24.0;
/// Its text size, in world units.
const PREPARING_TEXT: f32 = 11.0;

/// "Preparing image…" across the top of each box of `tab` whose image is
/// being prepared, also named for screen readers. Screen-only: exports
/// never show it.
pub fn preparing(ui: &egui::Ui, tab: &DocTab, layout: &Layout, tk: &Tokens) {
    if tab.pending_images.is_empty() {
        return;
    }
    let label = t("image.preparing");
    let painter = ui.painter_at(ui.max_rect());
    let zoom = tab.camera.zoom;
    for &node in tab.pending_images.keys() {
        let Some(&world) = layout.boxes.get(&node) else {
            continue;
        };
        let band = Rect::from_min_size(
            world.min + egui::vec2(BOX_TEXT_PAD, BOX_TEXT_PAD),
            egui::vec2(world.width() - 2.0 * BOX_TEXT_PAD, PREPARING_HEIGHT),
        );
        let r = tab.camera.world_rect_to_screen(band);
        if !painter.clip_rect().intersects(r) {
            continue;
        }
        painter.rect(
            r,
            3.0 * zoom,
            tk.panel,
            Stroke::new(1.0, tk.text_muted),
            StrokeKind::Inside,
        );
        if let Some(font) = screen_font(PREPARING_TEXT, zoom) {
            painter.text(
                r.center(),
                Align2::CENTER_CENTER,
                &label,
                font,
                tk.text_muted,
            );
        }
        let response = ui.interact(
            r,
            ui.scope_id().with(("preparing_image", node)),
            Sense::hover(),
        );
        response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &label));
    }
}
