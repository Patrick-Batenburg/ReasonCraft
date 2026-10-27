//! A ribbon row that is wider than the window, even with its groups
//! shrunk ([`super::fit`]), scrolls sideways: a chevron at an end (shown
//! only while there is more that way), the mouse wheel, and keyboard focus
//! moving onto a hidden button all bring the rest into view.
//!
//! The chevrons lie over the row's ends, on a narrow strip of the ribbon's
//! own colour that the buttons scroll under, so the row never moves when
//! one appears or goes.
//!
//! The edge cues are adapted from WordCraft (github.com/storytold/wordcraft),
//! Apache-2.0.

use eframe::egui::{
    self, Align, Id, Layout, Rect, Role, ScrollArea, Sense, Shape, Stroke, Ui, Vec2, WidgetInfo,
    pos2, scroll_area::ScrollBarVisibility,
};

use super::fit;
use crate::app::RibbonTab;
use crate::i18n::t;
use crate::shell::hover_fill;
use crate::theme::Tokens;

/// Width of a chevron strip, in points.
const STRIP_W: f32 = 18.0;
/// How far a chevron click scrolls, in points.
const STEP: f32 = 160.0;

/// The ribbon row of `tab`, filled by `add`: its groups shrink to fit,
/// and it scrolls sideways when even that is not enough. `button_h` is
/// the height of the buttons, which the chevrons are centred on.
pub(super) fn row(
    ui: &mut Ui,
    tk: &Tokens,
    tab: RibbonTab,
    button_h: f32,
    add: impl FnOnce(&mut Ui),
) {
    let jump_id = Id::unique(("ribbon_scroll_to", format!("{tab:?}")));
    let jump: Option<f32> = ui.ctx().data_mut(|d| d.remove_temp(jump_id));
    // The wheel scrolls the row without Shift.
    ui.style_mut().always_scroll_the_only_direction = true;
    // No spacing is added when a focused button is scrolled into view.
    ui.spacing_mut().item_spacing.x = 0.0;
    let mut area = ScrollArea::horizontal()
        .id_salt(("ribbon_scroll", format!("{tab:?}")))
        .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
        .auto_shrink([false, true])
        .max_width(ui.available_width());
    if let Some(x) = jump {
        area = area.horizontal_scroll_offset(x);
    }
    fit::begin(ui.ctx(), &format!("{tab:?}"), ui.available_width());
    let out = area.show(ui, |ui| {
        ui.with_layout(Layout::left_to_right(Align::Min), add);
    });
    fit::end(ui.ctx());
    let view = out.inner_rect;
    let hidden = out.content_size.x - view.width();
    let x = out.state.clamped_offset().x;
    let ends = [
        (x > 1.0, view.left(), -1.0, "ribbon.scroll_left"),
        (
            hidden - x > 1.0,
            view.right() - STRIP_W,
            1.0,
            "ribbon.scroll_right",
        ),
    ];
    for (shown, left, dir, key) in ends {
        if !shown {
            continue;
        }
        let rect = Rect::from_min_size(pos2(left, view.top()), Vec2::new(STRIP_W, view.height()));
        let chevron_y = view.top() + button_h / 2.0;
        if chevron(ui, tk, rect, chevron_y, dir, &t(key)) {
            let to = (x + dir * STEP).clamp(0.0, hidden);
            ui.ctx().data_mut(|d| d.insert_temp(jump_id, to));
            ui.ctx().request_repaint();
        }
    }
}

/// A chevron over the row's end, on a strip of ribbon colour; true when
/// clicked. `dir` is -1 for the left end and 1 for the right.
fn chevron(ui: &mut Ui, tk: &Tokens, rect: Rect, y: f32, dir: f32, label: &str) -> bool {
    let id = ui.scope_id().with(("ribbon_arrow", label));
    let resp = ui.interact(rect, id, Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(Role::Button, true, label));
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, tk.ribbon);
    let lit = resp.hovered() || resp.has_focus();
    if lit {
        let fill = hover_fill(tk, tk.ribbon);
        painter.rect_filled(rect.shrink2(Vec2::new(1.0, 4.0)), 3.0, fill);
    }
    let ink = if lit { tk.box_text } else { tk.text_muted };
    let (cx, w, h) = (rect.center().x, 2.5 * dir, 5.0);
    let points = vec![pos2(cx - w, y - h), pos2(cx + w, y), pos2(cx - w, y + h)];
    painter.add(Shape::line(points, Stroke::new(1.5, ink)));
    let clicked = resp.clicked();
    resp.on_hover_text(label);
    clicked
}

/// Brings a ribbon button into view when the keyboard focus lands on it,
/// clear of the chevron strips (at the row's ends, where no strip is
/// shown, the scroll stops at the end instead). Not for the buttons in a
/// collapsed group's drop-down, which is never scrolled.
pub(super) fn follow_focus(ui: &Ui, resp: &egui::Response) {
    if resp.gained_focus() && !fit::in_popup(ui.ctx()) {
        ui.scroll_to_rect(resp.rect.expand2(Vec2::new(STRIP_W, 0.0)), None);
    }
}
