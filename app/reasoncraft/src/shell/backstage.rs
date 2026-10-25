//! The File backstage: a full-window page with file commands and settings.

use eframe::egui::{
    self, Align, Align2, FontId, Frame, Id, Layout, Margin, Rect, RichText, Role, Sense, TextEdit,
    Vec2, WidgetInfo,
};

use super::hover_fill;
use super::icons::{self, Icon};
use crate::actions::Action;
use crate::app::{ReasonCraftApp, RibbonTab};
use crate::i18n::t;
use crate::shell::text_panel::check_key;
use crate::theme::{ThemeChoice, Tokens};
use reasoncraft_core::checks::CheckKind;

/// What a backstage entry does.
enum Entry {
    /// Back to the map, running the action (if any).
    Command(Option<Action>),
    /// Show the settings page.
    Settings,
}

/// Whether the settings page is showing (view state, kept in egui memory).
fn settings_id() -> Id {
    Id::unique("backstage_settings")
}

pub fn show(ui: &mut egui::Ui, app: &mut ReasonCraftApp) {
    let tk = app.tokens;
    egui::CentralPanel::default()
        .frame(Frame::new().fill(tk.panel).inner_margin(Margin::same(24)))
        .show(ui, |ui| {
            let mut settings = ui.data(|d| d.get_temp::<bool>(settings_id()).unwrap_or(false));
            ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                ui.vertical(|ui| {
                    ui.set_width(220.0);
                    entries(ui, app, &mut settings);
                });
                if settings {
                    ui.add_space(24.0);
                    ui.separator();
                    ui.add_space(24.0);
                    ui.vertical(|ui| settings_page(ui, app));
                }
            });
            ui.data_mut(|d| d.insert_temp(settings_id(), settings));
        });
}

fn entries(ui: &mut egui::Ui, app: &mut ReasonCraftApp, settings: &mut bool) {
    let tk = app.tokens;
    ui.spacing_mut().item_spacing.y = 4.0;
    let items = [
        (Icon::Back, "backstage.back", Entry::Command(None)),
        (Icon::New, "action.new", Entry::Command(Some(Action::New))),
        (
            Icon::Open,
            "action.open",
            Entry::Command(Some(Action::Open)),
        ),
        (
            Icon::Save,
            "action.save",
            Entry::Command(Some(Action::Save)),
        ),
        (
            Icon::SaveAs,
            "action.save_as",
            Entry::Command(Some(Action::SaveAs)),
        ),
        (Icon::Settings, "backstage.settings", Entry::Settings),
    ];
    for (icon, key, entry) in items {
        let label = t(key);
        let on = matches!(entry, Entry::Settings) && *settings;
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(220.0, 40.0), Sense::click());
        resp.widget_info(|| WidgetInfo::selected(Role::Button, true, on, &label));
        if resp.hovered() || on {
            ui.painter()
                .rect_filled(rect, 4.0, hover_fill(&tk, tk.panel));
        }
        let icon_rect =
            Rect::from_center_size(rect.left_center() + Vec2::new(22.0, 0.0), Vec2::splat(20.0));
        icons::paint(ui.painter(), icon_rect, icon, tk.box_text, tk.panel, &tk);
        ui.painter().text(
            rect.left_center() + Vec2::new(44.0, 0.0),
            Align2::LEFT_CENTER,
            &label,
            FontId::proportional(14.0),
            tk.box_text,
        );
        if resp.clicked() {
            match entry {
                Entry::Settings => *settings = !*settings,
                // Every file command returns to the map.
                Entry::Command(action) => {
                    app.ribbon_tab = RibbonTab::Home;
                    if let Some(action) = action {
                        app.push(action);
                    }
                }
            }
            ui.ctx().request_repaint();
        }
    }
}

/// The author name, the theme and which checks run.
fn settings_page(ui: &mut egui::Ui, app: &mut ReasonCraftApp) {
    let tk = app.tokens;
    ui.spacing_mut().item_spacing.y = 8.0;
    ui.label(
        RichText::new(t("settings.title"))
            .strong()
            .size(18.0)
            .color(tk.box_text),
    );
    ui.add_space(8.0);
    author_field(ui, app, &tk);
    ui.add_space(8.0);
    ui.label(RichText::new(t("settings.theme")).color(tk.box_text));
    ui.horizontal(|ui| {
        let themes = [
            ("theme.system", ThemeChoice::System),
            ("theme.light", ThemeChoice::Light),
            ("theme.dark", ThemeChoice::Dark),
        ];
        for (key, choice) in themes {
            if ui
                .selectable_label(app.settings.theme == choice, t(key))
                .clicked()
            {
                app.push(Action::SetTheme(choice));
            }
        }
    });
    ui.add_space(8.0);
    checks(ui, app, &tk);
}

/// One checkbox per course-rule check, its rule as the tooltip.
fn checks(ui: &mut egui::Ui, app: &mut ReasonCraftApp, tk: &Tokens) {
    ui.label(RichText::new(t("settings.checks")).color(tk.box_text));
    ui.label(RichText::new(t("settings.checks_hint")).color(tk.text_muted));
    ui.spacing_mut().item_spacing.y = 4.0;
    for kind in CheckKind::ALL {
        let key = check_key(kind);
        let mut on = app.settings.checks.is_on(kind);
        if ui
            .checkbox(&mut on, t(&format!("check.kind.{key}")))
            .on_hover_text(t(&format!("check.help.{key}")))
            .changed()
        {
            app.push(Action::SetCheckEnabled(kind, on));
        }
    }
}

/// The author name field. While it has focus it shows what is typed (so
/// a trailing space survives); every change is applied, trimmed.
fn author_field(ui: &mut egui::Ui, app: &mut ReasonCraftApp, tk: &Tokens) {
    let label = t("settings.author");
    ui.label(RichText::new(&label).color(tk.box_text));
    let id = Id::unique("settings_author");
    let focused = ui.memory(|m| m.has_focus(id));
    let mut text = if focused {
        ui.data(|d| d.get_temp::<String>(id))
            .unwrap_or_else(|| app.settings.author.clone())
    } else {
        app.settings.author.clone()
    };
    let response = ui.add(
        TextEdit::singleline(&mut text)
            .id(id)
            .hint_text(t("comment.anonymous"))
            .desired_width(280.0),
    );
    if response.changed() {
        app.push(Action::SetAuthor(text.clone()));
    }
    ui.data_mut(|d| d.insert_temp(id, text));
    ui.label(RichText::new(t("settings.author_hint")).color(tk.text_muted));
}
