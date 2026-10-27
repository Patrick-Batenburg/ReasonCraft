//! A ribbon group as planned to fit the window ([`super::fit`]): as
//! designed, compact, or collapsed into one drop-down button; and the
//! pieces every group is built from (columns of small buttons, menus).

use eframe::egui::{
    Align, Align2, FontId, Frame, Id, Key, Layout, Order, Popup, PopupCloseBehavior, RectAlign,
    Response, ScrollArea, Stroke, Ui, Widget, containers::menu, pos2,
};

use super::{BUTTON_H, LABEL_H, fit, large_button};
use crate::actions::Action;
use crate::i18n::{FluentArgs, t_args};
use crate::shell::icons::Icon;
use crate::theme::Tokens;

/// A labelled group: its controls, the label underneath and a separator
/// after it. Planned to fit the window, it may draw compact or collapsed
/// into one drop-down button showing `icon` and the label.
pub(super) fn group(ui: &mut Ui, tk: &Tokens, icon: Icon, label: &str, add: impl FnOnce(&mut Ui)) {
    let ctx = ui.ctx().clone();
    let level = fit::level(&ctx, label);
    let start = ui.cursor().left();
    let top = ui.max_rect().top();
    fit::start_group(&ctx, level == fit::COMPACT);
    if level == fit::COLLAPSED {
        collapsed(ui, tk, icon, label, add);
    } else {
        let label_w = ui
            .painter()
            .layout_no_wrap(label.to_owned(), FontId::proportional(11.0), tk.text_muted)
            .size()
            .x;
        let inner = ui
            .with_layout(Layout::left_to_right(Align::Min), |ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                ui.set_min_width(label_w + 12.0);
                add(ui);
            })
            .response
            .rect;
        let label_y = top + BUTTON_H + LABEL_H / 2.0;
        ui.painter().text(
            pos2(inner.center().x, label_y),
            Align2::CENTER_CENTER,
            label,
            FontId::proportional(11.0),
            tk.text_muted,
        );
    }
    ui.add_space(6.0);
    let x = ui.cursor().left();
    let line = Stroke::new(1.0, tk.attachment_line.gamma_multiply(0.6));
    ui.painter().vline(
        x,
        eframe::egui::Rangef::new(top + 6.0, top + BUTTON_H + LABEL_H - 2.0),
        line,
    );
    ui.add_space(8.0);
    fit::record(&ctx, label, level, ui.cursor().left() - start);
    fit::start_group(&ctx, false);
}

/// A collapsed group: one drop-down button that opens the whole group
/// below it.
fn collapsed(ui: &mut Ui, tk: &Tokens, icon: Icon, label: &str, add: impl FnOnce(&mut Ui)) {
    let mut args = FluentArgs::new();
    args.set("group", label);
    let name = t_args("ribbon.group_more", &args);
    let resp = large_button(ui, tk, icon, label, true, false, true);
    let resp = crate::a11y::labelled(resp, name);
    let popup_id = Popup::default_response_id(&resp);
    let was_open = Popup::is_id_open(ui.ctx(), popup_id);
    Popup::menu(&resp)
        .frame(Frame::popup(ui.style()).fill(tk.ribbon))
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        // Under the button from its left edge; a drop-down wider than
        // the room there is moved left only as far as the window needs.
        .align(RectAlign::BOTTOM_START)
        .align_alternatives(&[])
        .show(|ui| {
            fit::set_in_popup(ui.ctx(), true);
            ui.horizontal(|ui| {
                ui.set_height(BUTTON_H);
                ui.spacing_mut().item_spacing.x = 2.0;
                add(ui);
            });
            fit::set_in_popup(ui.ctx(), false);
        });
    if was_open {
        keep_focus(ui, &resp, popup_id);
    }
}

/// The focus never ends up nowhere when a collapsed group's drop-down
/// goes: Esc gives it back to the group's button, and tabbing out of the
/// drop-down onto the ribbon (or anywhere else outside it) closes it.
fn keep_focus(ui: &Ui, button: &Response, popup_id: Id) {
    let ctx = ui.ctx();
    if ctx.input(|i| i.key_pressed(Key::Escape)) {
        Popup::close_id(ctx, popup_id);
        button.request_focus();
        return;
    }
    let Some(focused) = ctx.memory(|m| m.focused()) else {
        return;
    };
    // Drop-downs and their menus are on foreground layers.
    let outside = ctx
        .read_response(focused)
        .is_some_and(|r| r.layer_id.order != Order::Foreground);
    if outside && focused != button.id {
        Popup::close_id(ctx, popup_id);
    }
}

/// The drop-down of a ribbon menu button (`resp`): a popup below it on
/// the ribbon, or a submenu beside it in a collapsed group's drop-down
/// (egui keeps one popup open at a time, so a second popup would close
/// the drop-down).
///
/// A menu taller than the window scrolls; an item the keyboard moves to
/// is scrolled into view.
pub(super) fn menu_popup(ui: &Ui, resp: &Response, add: impl FnOnce(&mut Ui)) {
    let scrolled = |ui: &mut Ui| {
        // The window's height less the menu frame's margins.
        let margins = Frame::menu(ui.style()).total_margin().sum().y;
        let max = ui.ctx().content_rect().height() - 2.0 * margins;
        ScrollArea::vertical()
            .max_height(max.max(40.0))
            .show(ui, add);
    };
    if menu::is_in_menu(ui) {
        menu::SubMenu::new().show(ui, resp, scrolled);
    } else {
        Popup::menu(resp).show(scrolled);
    }
}

/// An item of a ribbon menu, enabled or not; true when clicked, which
/// closes the menu (and a collapsed group's drop-down it is in).
pub(super) fn menu_item(ui: &mut Ui, enabled: bool, item: impl Widget) -> bool {
    let resp = ui.add_enabled(enabled, item);
    // The keyboard moving onto it scrolls a long menu.
    if resp.gained_focus() {
        resp.scroll_to_me(None);
    }
    let clicked = resp.clicked();
    if clicked {
        ui.close();
    }
    clicked
}

/// A column of small buttons, stacked without gaps so that three of them
/// are as tall as a large button and end above the group's label.
pub(super) fn small_column(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        add(ui);
    });
}

/// Whether a command clicked in a collapsed group's drop-down leaves it
/// open: a setting that may be changed again, or a step that is repeated.
pub(super) fn keeps_open(action: &Action) -> bool {
    #[cfg(debug_assertions)]
    if matches!(action, Action::ToggleSourceIcons) {
        return true;
    }
    matches!(
        action,
        Action::ToggleMarkers
            | Action::ToggleNumbers
            | Action::ToggleShading
            | Action::TogglePanel(_)
            | Action::SetTheme(_)
            | Action::ZoomIn
            | Action::ZoomOut
    )
}
