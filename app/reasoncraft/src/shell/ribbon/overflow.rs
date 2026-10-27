//! A ribbon row that is wider than the window scrolls sideways: arrow
//! buttons at its ends (shown only while there is more that way), the
//! mouse wheel, and keyboard focus moving onto a hidden button all bring
//! the rest into view, so every command stays reachable by mouse.

use eframe::egui::{
    self, Align, Id, Layout, Rect, Role, ScrollArea, Sense, Shape, Stroke, Ui, Vec2, WidgetInfo,
    pos2, scroll_area::ScrollBarVisibility,
};

use crate::app::RibbonTab;
use crate::i18n::t;
use crate::shell::hover_fill;
use crate::theme::Tokens;

/// Width of an arrow button, in points.
const ARROW_W: f32 = 22.0;

/// The ribbon row of `tab`, filled by `add`, scrolling sideways when it
/// does not fit.
pub(super) fn row(ui: &mut Ui, tk: &Tokens, tab: RibbonTab, add: impl FnOnce(&mut Ui)) {
    let jump_id = Id::unique(("ribbon_scroll_to", format!("{tab:?}")));
    let jump: Option<f32> = ui.ctx().data_mut(|d| d.remove_temp(jump_id));
    // The wheel scrolls the row without Shift.
    ui.style_mut().always_scroll_the_only_direction = true;
    let mut area = ScrollArea::horizontal()
        .id_salt(("ribbon_scroll", format!("{tab:?}")))
        .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
        .auto_shrink([false, true]);
    if let Some(x) = jump {
        area = area.horizontal_scroll_offset(x);
    }
    // While the row overflows (as found last frame), a gutter at each end
    // holds its arrow, so no button lies under one.
    let over_id = Id::unique(("ribbon_overflows", format!("{tab:?}")));
    let overflows = ui.ctx().data(|d| d.get_temp::<bool>(over_id)) == Some(true);
    let full = ui.available_width();
    let gutter = if overflows { ARROW_W } else { 0.0 };
    let out = ui
        .horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.add_space(gutter);
            let out = area.max_width(full - 2.0 * gutter).show(ui, |ui| {
                ui.with_layout(Layout::left_to_right(Align::Min), add);
            });
            ui.add_space(gutter);
            out
        })
        .inner;
    let view = out.inner_rect;
    let now_overflows = out.content_size.x > full + 0.5;
    if now_overflows != overflows {
        ui.ctx().data_mut(|d| d.insert_temp(over_id, now_overflows));
        ui.ctx().request_repaint();
    }
    let hidden = out.content_size.x - view.width();
    if !overflows || hidden <= 0.5 {
        return;
    }
    let x = out.state.clamped_offset().x;
    let page = view.width().max(ARROW_W) * 0.75;
    let ends = [
        (x > 0.5, view.left() - ARROW_W, -1.0, "ribbon.scroll_left"),
        (x < hidden - 0.5, view.right(), 1.0, "ribbon.scroll_right"),
    ];
    for (shown, left, dir, key) in ends {
        if !shown {
            continue;
        }
        let height = view.height().max(out.content_size.y);
        let rect = Rect::from_min_size(pos2(left, view.top()), Vec2::new(ARROW_W, height));
        if arrow(ui, tk, rect, dir, &t(key)) {
            let to = (x + dir * page).clamp(0.0, hidden);
            ui.ctx().data_mut(|d| d.insert_temp(jump_id, to));
            ui.ctx().request_repaint();
        }
    }
}

/// An arrow button over the row's end; true when clicked.
fn arrow(ui: &mut Ui, tk: &Tokens, rect: Rect, dir: f32, label: &str) -> bool {
    let id = ui.scope_id().with(("ribbon_arrow", label));
    let resp = ui.interact(rect, id, Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(Role::Button, true, label));
    let fill = if resp.hovered() {
        hover_fill(tk, tk.ribbon)
    } else {
        tk.ribbon
    };
    let painter = ui.painter();
    painter.rect_filled(rect, 4.0, fill);
    let edge = if dir < 0.0 { rect.right() } else { rect.left() };
    let line = Stroke::new(1.0, tk.attachment_line.gamma_multiply(0.6));
    painter.vline(edge, rect.y_range().shrink(6.0), line);
    let c = rect.center();
    let (w, h) = (4.0 * dir, 8.0);
    let points = vec![
        pos2(c.x - w, c.y - h),
        pos2(c.x + w, c.y),
        pos2(c.x - w, c.y + h),
    ];
    painter.add(Shape::line(points, Stroke::new(2.0, tk.box_text)));
    let clicked = resp.clicked();
    resp.on_hover_text(label);
    clicked
}

/// Brings a ribbon button into view when the keyboard focus lands on it.
pub(super) fn follow_focus(resp: &egui::Response) {
    if resp.gained_focus() {
        resp.scroll_to_me(None);
    }
}
