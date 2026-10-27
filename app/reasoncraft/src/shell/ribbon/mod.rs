//! The ribbon: a tab strip and, for the active tab, labelled groups of commands.

use eframe::egui::{
    self, Align, Align2, Color32, FontId, Frame, Layout, Margin, Popup, PopupCloseBehavior, Rect,
    Response, Role, Sense, Stroke, StrokeKind, Ui, Vec2, WidgetInfo, pos2,
};

use super::icons::{self, Icon};
use super::{hover_fill, selected_fill, tab_button};
use crate::actions::Action;
use crate::app::{ReasonCraftApp, RibbonTab};
use crate::commands;
use crate::i18n::{FluentArgs, t, t_args};
use crate::theme::Tokens;

mod evaluate;
mod export;
mod fit;
mod help;
mod home;
mod insert;
mod overflow;
mod view;

const STRIP_H: f32 = 30.0;
const BUTTON_H: f32 = 66.0;
const SMALL_H: f32 = 22.0;
const LABEL_H: f32 = 18.0;
/// Width of a small button without its label (in a compact group).
const SMALL_COMPACT_W: f32 = 28.0;

const TABS: [(RibbonTab, &str); 7] = [
    (RibbonTab::File, "ribbon.tab.file"),
    (RibbonTab::Home, "ribbon.tab.home"),
    (RibbonTab::Insert, "ribbon.tab.insert"),
    (RibbonTab::Evaluate, "ribbon.tab.evaluate"),
    (RibbonTab::View, "ribbon.tab.view"),
    (RibbonTab::Export, "ribbon.tab.export"),
    (RibbonTab::Help, "ribbon.tab.help"),
];

pub fn show(ui: &mut Ui, app: &mut ReasonCraftApp) {
    let tk = app.tokens;
    let shown = egui::Panel::top("ribbon")
        .frame(
            Frame::new()
                .fill(tk.ribbon)
                .inner_margin(Margin::symmetric(8, 0)),
        )
        .show_separator_line(false)
        .show(ui, |ui| {
            tab_strip(ui, app);
            let tab = app.ribbon_tab;
            if tab != RibbonTab::File {
                overflow::row(ui, &tk, tab, BUTTON_H, |ui| {
                    ui.set_height(BUTTON_H + LABEL_H + 4.0);
                    match tab {
                        RibbonTab::Home => home::show(ui, app),
                        RibbonTab::Insert => insert::show(ui, app),
                        RibbonTab::Evaluate => evaluate::show(ui, app),
                        RibbonTab::View => view::show(ui, app),
                        RibbonTab::Export => export::show(ui, app),
                        RibbonTab::Help => help::show(ui, app),
                        RibbonTab::File => {}
                    }
                });
            }
            let r = ui.max_rect();
            let line = Stroke::new(1.0, tk.attachment_line.gamma_multiply(0.5));
            ui.painter()
                .hline(r.x_range().expand(8.0), r.bottom(), line);
        });
    let pane = crate::shell::panes::Pane::Ribbon;
    app.panes.shown(pane, shown.response.rect);
}

fn tab_strip(ui: &mut Ui, app: &mut ReasonCraftApp) {
    let tk = app.tokens;
    ui.horizontal(|ui| {
        ui.set_height(STRIP_H);
        ui.spacing_mut().item_spacing.x = 4.0;
        for (tab, key) in TABS {
            let active = app.ribbon_tab == tab;
            let resp = tab_button(ui, &tk, &t(key), active, tk.ribbon, 20.0, STRIP_H);
            if resp.clicked() {
                // Clicking File again closes the backstage.
                app.ribbon_tab = if active && tab == RibbonTab::File {
                    RibbonTab::Home
                } else {
                    tab
                };
                ui.ctx().request_repaint();
            }
        }
    });
}

/// A large button for the registry command `id` (its label and action);
/// a click queues the action.
fn command(
    ui: &mut Ui,
    tk: &Tokens,
    icon: Icon,
    id: &str,
    enabled: bool,
    selected: bool,
    out: &mut Vec<Action>,
) -> Response {
    let cmd = commands::get(id);
    let resp = large(ui, tk, icon, &t(cmd.label), enabled, selected);
    if resp.clicked() {
        out.push(cmd.action.clone());
    }
    resp
}

/// Like [`command`], as a small button.
fn small_command(
    ui: &mut Ui,
    tk: &Tokens,
    icon: Icon,
    id: &str,
    enabled: bool,
    out: &mut Vec<Action>,
) -> Response {
    let cmd = commands::get(id);
    let resp = small(ui, tk, icon, &t(cmd.label), enabled);
    if resp.clicked() {
        out.push(cmd.action.clone());
    }
    resp
}

/// A labelled group: its controls, the label underneath and a separator
/// after it. Planned to fit the window ([`fit`]), it may draw compact or
/// collapsed into one drop-down button showing `icon` and the label.
fn group(ui: &mut Ui, tk: &Tokens, icon: Icon, label: &str, add: impl FnOnce(&mut Ui)) {
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
        egui::Rangef::new(top + 6.0, top + BUTTON_H + LABEL_H - 2.0),
        line,
    );
    ui.add_space(8.0);
    fit::record(&ctx, label, level, ui.cursor().left() - start);
    fit::start_group(&ctx, false);
}

/// A collapsed group: one drop-down button that opens the whole group.
fn collapsed(ui: &mut Ui, tk: &Tokens, icon: Icon, label: &str, add: impl FnOnce(&mut Ui)) {
    let mut args = FluentArgs::new();
    args.set("group", label);
    let name = t_args("ribbon.group_more", &args);
    let resp = large_button(ui, tk, icon, label, true, false, true);
    let resp = crate::a11y::labelled(resp, name);
    let frame = Frame::popup(ui.style()).fill(tk.ribbon);
    Popup::menu(&resp)
        .frame(frame)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            fit::set_in_popup(ui.ctx(), true);
            ui.horizontal(|ui| {
                ui.set_height(BUTTON_H);
                ui.spacing_mut().item_spacing.x = 2.0;
                add(ui);
            });
            fit::set_in_popup(ui.ctx(), false);
        });
}

/// Paint the hover or "on" background of a ribbon button; returns the colour under the icon.
fn button_bg(
    ui: &Ui,
    tk: &Tokens,
    rect: Rect,
    resp: &Response,
    enabled: bool,
    selected: bool,
) -> Color32 {
    if selected {
        let fill = selected_fill(tk, tk.ribbon);
        ui.painter().rect_filled(rect, 4.0, fill);
        let border = Stroke::new(1.0, tk.accent.gamma_multiply(0.7));
        ui.painter()
            .rect_stroke(rect, 4.0, border, StrokeKind::Inside);
        fill
    } else if enabled && resp.hovered() {
        let fill = hover_fill(tk, tk.ribbon);
        ui.painter().rect_filled(rect, 4.0, fill);
        fill
    } else {
        tk.ribbon
    }
}

/// A large button: icon on top, label underneath.
fn large(
    ui: &mut Ui,
    tk: &Tokens,
    icon: Icon,
    label: &str,
    enabled: bool,
    selected: bool,
) -> Response {
    large_button(ui, tk, icon, label, enabled, selected, false)
}

/// A large button that opens a dropdown: a "▾" follows its label.
fn large_menu(ui: &mut Ui, tk: &Tokens, icon: Icon, label: &str, enabled: bool) -> Response {
    large_button(ui, tk, icon, label, enabled, false, true)
}

/// Width of a dropdown arrow and its gap to the label.
const ARROW_W: f32 = 7.0;
const ARROW_GAP: f32 = 4.0;

fn large_button(
    ui: &mut Ui,
    tk: &Tokens,
    icon: Icon,
    label: &str,
    enabled: bool,
    selected: bool,
    dropdown: bool,
) -> Response {
    let colour = if enabled { tk.box_text } else { tk.text_muted };
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), FontId::proportional(12.0), colour);
    let arrow = if dropdown { ARROW_GAP + ARROW_W } else { 0.0 };
    let text_w = galley.size().x + arrow;
    let size = Vec2::new((text_w + 14.0).max(50.0), BUTTON_H);
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(size, sense);
    resp.widget_info(|| WidgetInfo::selected(Role::Button, enabled, selected, label));
    overflow::follow_focus(ui, &resp);
    if ui.is_rect_visible(rect) {
        let surface = button_bg(ui, tk, rect, &resp, enabled, selected);
        let icon_rect =
            Rect::from_center_size(pos2(rect.center().x, rect.top() + 22.0), Vec2::splat(28.0));
        icons::paint(ui.painter(), icon_rect, icon, colour, surface, tk);
        let text_pos = pos2(rect.center().x - text_w / 2.0, rect.top() + 44.0);
        if dropdown {
            let x = text_pos.x + galley.size().x + ARROW_GAP;
            let y = text_pos.y + galley.size().y / 2.0;
            let tip = [
                pos2(x, y - ARROW_W / 4.0),
                pos2(x + ARROW_W, y - ARROW_W / 4.0),
                pos2(x + ARROW_W / 2.0, y + ARROW_W / 4.0),
            ];
            ui.painter().add(egui::Shape::convex_polygon(
                tip.to_vec(),
                colour,
                Stroke::NONE,
            ));
        }
        ui.painter().galley(text_pos, galley, colour);
    }
    resp
}

/// A small button: icon and label side by side.
fn small(ui: &mut Ui, tk: &Tokens, icon: Icon, label: &str, enabled: bool) -> Response {
    let colour = if enabled { tk.box_text } else { tk.text_muted };
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), FontId::proportional(12.0), colour);
    let full_w = galley.size().x + 34.0;
    let compact = fit::compact(ui.ctx());
    if !compact {
        fit::add_saving(ui.ctx(), ui.scope_id(), full_w - SMALL_COMPACT_W);
    }
    let width = if compact { SMALL_COMPACT_W } else { full_w };
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, SMALL_H), sense);
    resp.widget_info(|| WidgetInfo::labeled(Role::Button, enabled, label));
    overflow::follow_focus(ui, &resp);
    if ui.is_rect_visible(rect) {
        let surface = button_bg(ui, tk, rect, &resp, enabled, false);
        let icon_rect =
            Rect::from_center_size(rect.left_center() + Vec2::new(14.0, 0.0), Vec2::splat(16.0));
        icons::paint(ui.painter(), icon_rect, icon, colour, surface, tk);
        if !compact {
            let text_pos = pos2(rect.left() + 28.0, rect.center().y - galley.size().y / 2.0);
            ui.painter().galley(text_pos, galley, colour);
        }
    }
    // Compact, the label is only in the tooltip.
    if compact {
        resp.on_hover_text(label)
    } else {
        resp
    }
}
