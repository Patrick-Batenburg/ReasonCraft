//! The ribbon: a tab strip and, for the active tab, labelled groups of commands.

use eframe::egui::{
    self, Color32, FontId, Frame, Margin, Rect, Response, Role, Sense, Stroke, StrokeKind, Ui,
    Vec2, WidgetInfo, pos2,
};

use super::icons::{self, Icon};
use super::{hover_fill, selected_fill, tab_button};
use crate::actions::Action;
use crate::app::{ReasonCraftApp, RibbonTab};
use crate::commands;
use crate::i18n::t;
use crate::theme::Tokens;

mod evaluate;
mod export;
mod fit;
mod groups;
mod help;
mod home;
mod insert;
mod overflow;
mod tips;
mod view;

use groups::{group, keeps_open, menu_item, menu_popup, small_column};

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
    tips::prepare(ui.ctx(), app.os, app.active_tab());
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

/// The ribbon's tabs; in a window too narrow for all of them the row
/// scrolls sideways like the groups (chevrons, wheel, keyboard focus).
fn tab_strip(ui: &mut Ui, app: &mut ReasonCraftApp) {
    let tk = app.tokens;
    let shown_id = egui::Id::unique("ribbon_tab_shown");
    let shown: Option<RibbonTab> = ui.data(|d| d.get_temp(shown_id));
    ui.data_mut(|d| d.insert_temp(shown_id, app.ribbon_tab));
    overflow::tab_row(ui, &tk, STRIP_H, |ui| {
        ui.set_height(STRIP_H);
        ui.spacing_mut().item_spacing.x = 4.0;
        for (tab, key) in TABS {
            let active = app.ribbon_tab == tab;
            let resp = tab_button(ui, &tk, &t(key), active, tk.ribbon, 20.0, STRIP_H);
            overflow::follow_focus(ui, &resp);
            if active && shown != Some(tab) {
                // A tab chosen another way (a command) comes into view.
                resp.scroll_to_me(None);
            }
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

/// A large button for the registry command `id` (its label, action and
/// tooltip, see [`tips`]); a click queues the action.
fn command(
    ui: &mut Ui,
    tk: &Tokens,
    id: &str,
    enabled: bool,
    selected: bool,
    out: &mut Vec<Action>,
) -> Response {
    let cmd = commands::get(id);
    let resp = large(ui, tk, cmd.ribbon_icon(), &t(cmd.label), enabled, selected);
    let resp = tips::command(resp, cmd, enabled);
    if resp.clicked() {
        out.push(cmd.action.clone());
        close_drop_down(ui, &cmd.action);
    }
    resp
}

/// In a collapsed group's drop-down, a command that is done once (unlike
/// a toggle or a zoom step) closes the drop-down.
fn close_drop_down(ui: &Ui, action: &Action) {
    if fit::in_popup(ui.ctx()) && !keeps_open(action) {
        ui.close();
    }
}

/// Like [`command`], as a small button.
fn small_command(
    ui: &mut Ui,
    tk: &Tokens,
    id: &str,
    enabled: bool,
    out: &mut Vec<Action>,
) -> Response {
    let cmd = commands::get(id);
    // The tooltip also names the button when it is compact (icon only).
    let resp = small(ui, tk, cmd.ribbon_icon(), &t(cmd.label), enabled);
    let resp = tips::command(resp, cmd, enabled);
    if resp.clicked() {
        out.push(cmd.action.clone());
        close_drop_down(ui, &cmd.action);
    }
    resp
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
    resp
}
