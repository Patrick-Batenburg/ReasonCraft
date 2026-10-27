//! One choice on a backstage page: an icon, a name and a muted line
//! under it (what a template holds, or a recent file's folder).

use eframe::egui::{
    self, Color32, FontId, Rect, Response, Role, Sense, Ui, Vec2, WidgetInfo,
    text::{LayoutJob, TextFormat},
};

use super::super::hover_fill;
use super::super::icons::{self, Icon};
use crate::theme::Tokens;

/// The width of a choice.
pub(super) const WIDTH: f32 = 460.0;
const PAD: f32 = 8.0;
const ICON: f32 = 24.0;

/// `text` in `size` and `colour`, wrapped at `width`.
fn job(text: &str, size: f32, colour: Color32, width: f32) -> LayoutJob {
    let mut job = LayoutJob::single_section(
        text.to_owned(),
        TextFormat::simple(FontId::proportional(size), colour),
    );
    job.wrap.max_width = width;
    job
}

/// A clickable choice named `spoken` for screen readers.
pub(super) fn choice(
    ui: &mut Ui,
    tk: &Tokens,
    icon: Icon,
    name: &str,
    detail: &str,
    spoken: &str,
) -> Response {
    let text_w = WIDTH - ICON - 3.0 * PAD;
    let name_g = ui.fonts_mut(|f| f.layout_job(job(name, 14.0, tk.box_text, text_w)));
    let detail_g = ui.fonts_mut(|f| f.layout_job(job(detail, 12.0, tk.text_muted, text_w)));
    let text_h = name_g.size().y + 2.0 + detail_g.size().y;
    let size = Vec2::new(WIDTH, text_h.max(ICON) + 2.0 * PAD);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(Role::Button, true, spoken));
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let surface = if resp.hovered() || resp.has_focus() {
            let fill = hover_fill(tk, tk.panel);
            painter.rect_filled(rect, 4.0, fill);
            fill
        } else {
            tk.panel
        };
        let icon_rect = Rect::from_min_size(
            rect.min + Vec2::new(PAD, PAD + (text_h.max(ICON) - ICON) / 2.0),
            Vec2::splat(ICON),
        );
        icons::paint(painter, icon_rect, icon, tk.box_text, surface, tk);
        let x = icon_rect.right() + PAD;
        let top = rect.top() + PAD;
        let name_h = name_g.size().y;
        painter.galley(egui::pos2(x, top), name_g, tk.box_text);
        painter.galley(egui::pos2(x, top + name_h + 2.0), detail_g, tk.text_muted);
    }
    resp
}
