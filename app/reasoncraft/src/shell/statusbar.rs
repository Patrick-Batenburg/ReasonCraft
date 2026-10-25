//! Status bar: box count, theme toggle and zoom slider.

use eframe::egui::{self, Align, Frame, Layout, Margin, RichText, Slider};

use super::icon_button;
use super::icons::Icon;
use crate::actions::{Action, ZOOM_MAX, ZOOM_MIN};
use crate::app::ReasonCraftApp;
use crate::i18n::{FluentArgs, t, t_args};
use crate::theme::ThemeChoice;

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
                let tab = app.active_tab();
                let mut args = FluentArgs::new();
                args.set("count", tab.doc.boxes.len());
                ui.label(
                    RichText::new(t_args("status.boxes", &args))
                        .size(12.0)
                        .color(tk.text_muted),
                );

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
