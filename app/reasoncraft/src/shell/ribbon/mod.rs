//! The ribbon: a tab strip and, for the active tab, labelled groups of commands.

use eframe::egui::{
    self, Align, Align2, Color32, FontId, Frame, Layout, Margin, Rect, Response, Role, Sense,
    Stroke, StrokeKind, Ui, Vec2, WidgetInfo, pos2,
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
mod home;
mod view;

const STRIP_H: f32 = 30.0;
const BUTTON_H: f32 = 66.0;
const SMALL_H: f32 = 22.0;
const LABEL_H: f32 = 18.0;

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
    egui::Panel::top("ribbon")
        .frame(
            Frame::new()
                .fill(tk.ribbon)
                .inner_margin(Margin::symmetric(8, 0)),
        )
        .show_separator_line(false)
        .show(ui, |ui| {
            tab_strip(ui, app);
            if app.ribbon_tab != RibbonTab::File {
                ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                    ui.set_height(BUTTON_H + LABEL_H + 4.0);
                    match app.ribbon_tab {
                        RibbonTab::Home => home::show(ui, app),
                        RibbonTab::Evaluate => evaluate::show(ui, app),
                        RibbonTab::View => view::show(ui, app),
                        RibbonTab::Export => export::show(ui, app),
                        _ => {
                            ui.add_space(8.0);
                            ui.label(
                                egui::RichText::new(t("ribbon.coming_soon")).color(tk.text_muted),
                            );
                        }
                    }
                });
            }
            let r = ui.max_rect();
            let line = Stroke::new(1.0, tk.attachment_line.gamma_multiply(0.5));
            ui.painter()
                .hline(r.x_range().expand(8.0), r.bottom(), line);
        });
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

/// A labelled group: its controls, the label underneath and a separator after it.
fn group(ui: &mut Ui, tk: &Tokens, label: &str, add: impl FnOnce(&mut Ui)) {
    let top = ui.max_rect().top();
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
    ui.add_space(6.0);
    let x = ui.cursor().left();
    let line = Stroke::new(1.0, tk.attachment_line.gamma_multiply(0.6));
    ui.painter().vline(
        x,
        egui::Rangef::new(top + 6.0, top + BUTTON_H + LABEL_H - 2.0),
        line,
    );
    ui.add_space(8.0);
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
    let size = Vec2::new(galley.size().x + 34.0, SMALL_H);
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(size, sense);
    resp.widget_info(|| WidgetInfo::labeled(Role::Button, enabled, label));
    if ui.is_rect_visible(rect) {
        let surface = button_bg(ui, tk, rect, &resp, enabled, false);
        let icon_rect =
            Rect::from_center_size(rect.left_center() + Vec2::new(14.0, 0.0), Vec2::splat(16.0));
        icons::paint(ui.painter(), icon_rect, icon, colour, surface, tk);
        let text_pos = pos2(rect.left() + 28.0, rect.center().y - galley.size().y / 2.0);
        ui.painter().galley(text_pos, galley, colour);
    }
    resp
}
