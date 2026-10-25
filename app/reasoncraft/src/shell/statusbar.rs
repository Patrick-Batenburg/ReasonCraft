//! Status bar: box count, issue count (or the "Connect to…" hint), theme
//! toggle and zoom slider.

use eframe::egui::{
    self, Align, FontId, Frame, Layout, Margin, Response, RichText, Role, Sense, Slider,
    WidgetInfo, vec2,
};

use super::icons::Icon;
use super::{hover_fill, icon_button};
use crate::actions::{Action, Panel, ZOOM_MAX, ZOOM_MIN};
use crate::app::ReasonCraftApp;
use crate::i18n::{FluentArgs, t, t_args};
use crate::settings::TextTab;
use crate::theme::{ThemeChoice, Tokens};

pub fn show(ui: &mut egui::Ui, app: &mut ReasonCraftApp) {
    let tk = app.tokens;
    egui::Panel::bottom("statusbar")
        .frame(
            Frame::new()
                .fill(tk.panel)
                .inner_margin(Margin::symmetric(12, 2)),
        )
        .exact_size(26.0)
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                let issues = app.tabs[app.active].issues(&app.settings.checks).len();
                let tab = app.active_tab();
                let mut args = FluentArgs::new();
                args.set("count", tab.doc.boxes.len());
                ui.label(
                    RichText::new(t_args("status.boxes", &args))
                        .size(12.0)
                        .color(tk.text_muted),
                );
                ui.add_space(12.0);
                if issue_chip(ui, issues, &tk).clicked() {
                    app.pending.push(Action::SetTextTab(TextTab::Check));
                    app.pending.push(Action::ShowPanel(Panel::Right));
                }
                let tab = app.active_tab();
                if tab.connect_mode.is_some() {
                    ui.add_space(16.0);
                    ui.label(
                        RichText::new(t("connect.hint"))
                            .size(12.0)
                            .strong()
                            .color(tk.accent),
                    );
                }

                let zoom = tab.camera.zoom;
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let mut args = FluentArgs::new();
                    args.set("percent", (zoom * 100.0).round());
                    ui.label(
                        RichText::new(t_args("status.zoom_value", &args))
                            .size(12.0)
                            .color(tk.text_muted),
                    );

                    let mut pct = zoom * 100.0;
                    let slider = ui
                        .add(
                            Slider::new(&mut pct, ZOOM_MIN * 100.0..=ZOOM_MAX * 100.0)
                                .logarithmic(true)
                                .show_value(false),
                        )
                        .on_hover_text(t("status.zoom"));
                    if slider.changed() {
                        app.push(Action::SetZoom(pct / 100.0));
                    }

                    ui.add_space(8.0);
                    let (icon, next) = if tk.dark {
                        (Icon::ThemeLight, ThemeChoice::Light)
                    } else {
                        (Icon::ThemeDark, ThemeChoice::Dark)
                    };
                    let label = t("status.toggle_theme");
                    if icon_button(ui, icon, &label, 22.0, true, &tk, tk.panel).clicked() {
                        app.push(Action::SetTheme(next));
                    }
                });
            });
        });
}

/// "3 issues", with a dot when there are any; a click shows them in the
/// Check tab.
fn issue_chip(ui: &mut egui::Ui, count: usize, tk: &Tokens) -> Response {
    const DOT: f32 = 6.0;
    let mut args = FluentArgs::new();
    args.set("count", count);
    let label = t_args("status.issues", &args);
    let colour = if count > 0 {
        tk.box_text
    } else {
        tk.text_muted
    };
    let galley = ui
        .painter()
        .layout_no_wrap(label.clone(), FontId::proportional(12.0), colour);
    let dot_w = if count > 0 { DOT + 5.0 } else { 0.0 };
    let size = vec2(galley.size().x + dot_w + 12.0, 20.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(Role::Button, true, &label));
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if resp.hovered() {
            painter.rect_filled(rect, 4.0, hover_fill(tk, tk.panel));
        }
        let mut x = rect.left() + 6.0;
        if count > 0 {
            let centre = egui::pos2(x + DOT / 2.0, rect.center().y);
            painter.circle_filled(centre, DOT / 2.0, tk.rebuttal);
            x += dot_w;
        }
        let y = rect.center().y - galley.size().y / 2.0;
        painter.galley(egui::pos2(x, y), galley, colour);
    }
    resp.on_hover_text(t("status.tip.issues"))
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}
