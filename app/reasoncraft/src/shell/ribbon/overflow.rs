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
//! Copyright (c) 2026 ArtCraft Team and the WordCraft contributors,
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
    let name = format!("{tab:?}");
    let keys = ["ribbon.scroll_left", "ribbon.scroll_right"];
    ui.scope(|ui| scrolling_row(ui, tk, &name, true, keys, button_h, add));
}

/// The row of ribbon tabs, filled by `add`: it scrolls sideways like a
/// row of groups when the window is too narrow for every tab (its tabs
/// call [`follow_focus`]). `height` is the row's.
pub(super) fn tab_row(ui: &mut Ui, tk: &Tokens, height: f32, add: impl FnOnce(&mut Ui)) {
    let keys = ["ribbon.tabs_left", "ribbon.tabs_right"];
    ui.scope(|ui| scrolling_row(ui, tk, "tabs", false, keys, height, add));
}

/// A row named `name` that scrolls sideways; `fit` shrinks its groups
/// first ([`super::fit`]); `keys` name its left and right chevrons.
fn scrolling_row(
    ui: &mut Ui,
    tk: &Tokens,
    name: &str,
    fit: bool,
    keys: [&str; 2],
    button_h: f32,
    add: impl FnOnce(&mut Ui),
) {
    let ctx = ui.ctx().clone();
    let jump_id = Id::unique(("ribbon_scroll_to", name));
    let jump: Option<f32> = ctx.data_mut(|d| d.remove_temp(jump_id));
    // The wheel scrolls the row without Shift.
    ui.style_mut().always_scroll_the_only_direction = true;
    // No spacing is added when a focused button is scrolled into view.
    ui.spacing_mut().item_spacing.x = 0.0;
    let mut area = ScrollArea::horizontal()
        .id_salt(("ribbon_scroll", name))
        .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
        .auto_shrink([false, true])
        .max_width(ui.available_width());
    if let Some(x) = jump {
        area = area.horizontal_scroll_offset(x);
    }
    if fit {
        fit::begin(&ctx, name, ui.available_width());
    }
    ctx.data_mut(|d| d.remove_temp::<Vec<Id>>(buttons_id()));
    let out = area.show(ui, |ui| {
        ui.with_layout(Layout::left_to_right(Align::Min), add);
    });
    if fit {
        fit::end(&ctx);
    }
    let buttons: Vec<Id> = ctx.data_mut(|d| d.remove_temp(buttons_id()).unwrap_or_default());
    let view = out.inner_rect;
    let hidden = out.content_size.x - view.width();
    let x = out.state.clamped_offset().x;
    let ends = [
        (x > 1.0, view.left(), -1.0, keys[0], buttons.first()),
        (
            hidden - x > 1.0,
            view.right() - STRIP_W,
            1.0,
            keys[1],
            buttons.last(),
        ),
    ];
    let focused = ctx.memory(|m| m.focused());
    for (shown, left, dir, key, end_button) in ends {
        let label = t(key);
        let id = ui.scope_id().with(("ribbon_arrow", &label));
        if !shown {
            // The chevron went while it had the focus (it reached its
            // end): the focus goes to the button at that end.
            if focused == Some(id)
                && let Some(&b) = end_button
            {
                ctx.memory_mut(|m| m.request_focus(b));
            }
            continue;
        }
        let rect = Rect::from_min_size(pos2(left, view.top()), Vec2::new(STRIP_W, view.height()));
        let chevron_y = view.top() + button_h / 2.0;
        if chevron(ui, tk, id, rect, chevron_y, dir, &label) {
            let to = (x + dir * STEP).clamp(0.0, hidden);
            ctx.data_mut(|d| d.insert_temp(jump_id, to));
            ctx.request_repaint();
        }
    }
}

/// Where the row's focusable buttons, in order, are collected each frame.
fn buttons_id() -> Id {
    Id::unique("ribbon_row_buttons")
}

/// A chevron over the row's end, on a strip of ribbon colour; true when
/// clicked. `dir` is -1 for the left end and 1 for the right.
fn chevron(ui: &mut Ui, tk: &Tokens, id: Id, rect: Rect, y: f32, dir: f32, label: &str) -> bool {
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
/// shown, the scroll stops at the end instead), and notes it as one of
/// the row's buttons. Not for the buttons in a collapsed group's
/// drop-down, which is never scrolled.
pub(super) fn follow_focus(ui: &Ui, resp: &egui::Response) {
    if fit::in_popup(ui.ctx()) {
        return;
    }
    if resp.sense.is_focusable() {
        ui.ctx().data_mut(|d| {
            d.get_temp_mut_or_default::<Vec<Id>>(buttons_id())
                .push(resp.id)
        });
    }
    if resp.gained_focus() {
        ui.scroll_to_rect(resp.rect.expand2(Vec2::new(STRIP_W, 0.0)), None);
    }
}
