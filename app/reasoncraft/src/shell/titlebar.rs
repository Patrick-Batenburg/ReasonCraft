//! Custom title bar: quick access, document title, command search, window buttons.

use eframe::egui::{
    self, Align, Align2, FontId, Frame, Layout, Margin, Sense, Vec2, ViewportCommand,
};

use super::icons::Icon;
use super::{hover_fill, icon_button};
use crate::actions::{Action, is_enabled};
use crate::app::ReasonCraftApp;
use crate::i18n::t;

const HEIGHT: f32 = 36.0;
/// Room for the macOS traffic lights, which draw over our content.
const MAC_LEFT_INSET: f32 = 72.0;

pub fn show(ui: &mut egui::Ui, app: &mut ReasonCraftApp) {
    let tk = app.tokens;
    let frame = Frame::new()
        .fill(tk.bg)
        .inner_margin(Margin::symmetric(8, 0));
    egui::Panel::top("titlebar")
        .frame(frame)
        .exact_size(HEIGHT)
        .show_separator_line(false)
        .show(ui, |ui| {
            let bar = ui.max_rect();
            // Interact with the whole bar first, so the buttons on top win the
            // hit test. It moves the window by mouse only: Tab skips it, and a
            // screen reader reads it as the title bar.
            let drag = ui.interact(
                bar,
                ui.scope_id().with("titlebar_drag"),
                Sense::CLICK | Sense::DRAG,
            );
            let title = app.active_tab().title();
            drag.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TitleBar, true, &title));
            if drag.drag_started() {
                ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
            }
            let maximized = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
            if drag.double_clicked() {
                ui.ctx()
                    .send_viewport_cmd(ViewportCommand::Maximized(!maximized));
            }

            ui.painter().text(
                bar.center(),
                Align2::CENTER_CENTER,
                app.active_tab().title(),
                FontId::proportional(13.0),
                tk.box_text,
            );

            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                if cfg!(target_os = "macos") {
                    ui.add_space(MAC_LEFT_INSET);
                }
                let tab = app.active_tab();
                let (can_undo, can_redo) = (
                    is_enabled(tab, &Action::Undo),
                    is_enabled(tab, &Action::Redo),
                );
                let quick = [
                    (Icon::Save, "action.save", true, Action::Save),
                    (Icon::Undo, "action.undo", can_undo, Action::Undo),
                    (Icon::Redo, "action.redo", can_redo, Action::Redo),
                ];
                for (icon, key, enabled, action) in quick {
                    if icon_button(ui, icon, &t(key), 28.0, enabled, &tk, tk.bg).clicked() {
                        app.push(action);
                    }
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    if !cfg!(target_os = "macos") {
                        window_buttons(ui, app, maximized);
                        ui.add_space(12.0);
                    }
                    super::command_search::show(ui, app);
                });
            });
        });
}

fn window_buttons(ui: &mut egui::Ui, app: &mut ReasonCraftApp, maximized: bool) {
    let tk = app.tokens;
    let ctx = ui.ctx().clone();
    let size = Vec2::new(44.0, HEIGHT);
    let button = |ui: &mut egui::Ui, icon: Icon, label: String, danger: bool| {
        let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
        let hovered = resp.hovered();
        let (surface, fg) = match (hovered, danger) {
            (true, true) => (tk.danger, tk.on_danger),
            (true, false) => (hover_fill(&tk, tk.bg), tk.box_text),
            (false, _) => (tk.bg, tk.box_text),
        };
        if hovered {
            ui.painter().rect_filled(rect, 0.0, surface);
        }
        let icon_rect = egui::Rect::from_center_size(rect.center(), Vec2::splat(14.0));
        super::icons::paint(ui.painter(), icon_rect, icon, fg, surface, &tk);
        resp.on_hover_text(label).clicked()
    };
    // Right to left: close, maximize/restore, minimize.
    // Closing asks about unsaved changes first.
    if button(ui, Icon::Close, t("window.close"), true) {
        app.push(Action::CloseWindow);
    }
    let (icon, key) = if maximized {
        (Icon::Restore, "window.restore")
    } else {
        (Icon::Maximize, "window.maximize")
    };
    if button(ui, icon, t(key), false) {
        ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
    }
    if button(ui, Icon::Minimize, t("window.minimize"), false) {
        ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
    }
}
