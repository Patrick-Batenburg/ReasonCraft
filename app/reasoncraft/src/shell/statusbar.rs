//! Status bar: box count, issue count (or the "Connect to…" hint), the
//! busy chip of background jobs or the last export's notice, a status
//! note, theme toggle and zoom slider.

use eframe::egui::{
    self, Align, FontId, Frame, Layout, Margin, Response, RichText, Role, Sense, Slider,
    WidgetInfo, vec2,
};

use super::icons::Icon;
use super::{hover_fill, icon_button};
use crate::actions::{Action, Panel, ZOOM_MAX, ZOOM_MIN};
use crate::app::ReasonCraftApp;
use crate::export::Notice;
use crate::i18n::{FluentArgs, t, t_args};
use crate::settings::TextTab;
use crate::theme::{ThemeChoice, Tokens};

pub fn show(ui: &mut egui::Ui, app: &mut ReasonCraftApp) {
    let tk = app.tokens;
    let shown = egui::Panel::bottom("statusbar")
        .frame(
            Frame::new()
                .fill(tk.panel)
                .inner_margin(Margin::symmetric(12, 2)),
        )
        .exact_size(26.0)
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                let now = std::time::Instant::now();
                let tab = &mut app.tabs[app.active];
                let (found, due) = tab.settled_issues(&app.settings.checks, now);
                let issues = found.len();
                if let Some(due) = due {
                    ui.ctx().request_repaint_after(due);
                }
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

                job_status(ui, app, &tk);

                let zoom = app.active_tab().camera.zoom;
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
    let pane = crate::shell::panes::Pane::Status;
    app.panes.shown(pane, shown.response.rect);
}

/// How long a status note or export notice stays, in seconds.
pub const NOTE_SECS: f64 = 6.0;

/// Clears the status note and the export notice once they have shown for
/// [`NOTE_SECS`]; a new note starts the time again.
pub(crate) fn expire_notes(ctx: &egui::Context, app: &mut ReasonCraftApp) {
    if app.status_note.is_none() && app.export.notice.is_none() {
        app.notes_shown = None;
        return;
    }
    let now = ctx.input(|i| i.time);
    let current = (app.status_note.clone(), app.export.notice.clone());
    let since = match &app.notes_shown {
        Some((since, note, notice)) if (note, notice) == (&current.0, &current.1) => *since,
        _ => {
            app.notes_shown = Some((now, current.0, current.1));
            now
        }
    };
    let left = NOTE_SECS - (now - since);
    if left <= 0.0 {
        app.status_note = None;
        app.export.notice = None;
        app.notes_shown = None;
    } else {
        ctx.request_repaint_after(std::time::Duration::from_secs_f64(left));
    }
}

/// A spinner while background jobs run ("Exporting…", or "Preparing
/// image…" when only images are; the tooltip lists them all); otherwise
/// what the last export did ("Export done" with "Open folder", or
/// "Copied"). Then the status note, if any.
fn job_status(ui: &mut egui::Ui, app: &mut ReasonCraftApp, tk: &Tokens) {
    let text = |s: String| RichText::new(s).size(12.0).color(tk.text_muted);
    // Images count while a tab waits for them, also a finished one held
    // for an edit session: the same ones that show "Preparing image…".
    let images: usize = app.tabs.iter().map(|t| t.pending_images.len()).sum();
    let exports = app.export.running();
    if exports + images > 0 {
        ui.add_space(16.0);
        ui.add(egui::Spinner::new().size(12.0).color(tk.accent));
        let key = if exports > 0 {
            "status.exporting"
        } else {
            "status.preparing_image"
        };
        let mut lines = Vec::new();
        for (key, count) in [
            ("status.jobs.exports", exports),
            ("status.jobs.images", images),
        ] {
            if count > 0 {
                let mut args = FluentArgs::new();
                args.set("count", count);
                lines.push(t_args(key, &args));
            }
        }
        ui.label(text(t(key))).on_hover_text(lines.join("\n"));
    } else {
        match &app.export.notice {
            Some(Notice::Saved(_)) => {
                ui.add_space(16.0);
                ui.label(text(t("export.done")));
                let link = RichText::new(t("export.open_folder")).size(12.0);
                if ui.link(link).clicked() {
                    app.pending.push(Action::OpenExportFolder);
                }
            }
            Some(Notice::Copied) => {
                ui.add_space(16.0);
                ui.label(text(t("export.copied")));
            }
            None => {}
        }
    }
    if let Some(note) = &app.status_note {
        ui.add_space(16.0);
        ui.label(text(note.clone()));
    }
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
