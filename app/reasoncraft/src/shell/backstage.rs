//! The File backstage: a full-window page with file commands and settings.

use eframe::egui::{
    self, Align, Align2, FontId, Frame, Id, Layout, Margin, Rect, RichText, Role, Sense, TextEdit,
    Vec2, WidgetInfo,
};

use super::hover_fill;
use super::icons::{self, Icon};
use crate::a11y::{in_group, labelled};
use crate::actions::Action;
use crate::app::{ReasonCraftApp, RibbonTab};
use crate::commands;
use crate::i18n::{UI_LANGUAGES, t};
use crate::settings::{AUTOSAVE_DEFAULT, AUTOSAVE_MAX, AUTOSAVE_MIN, UI_SCALE_MAX, UI_SCALE_MIN};
use crate::shell::text_panel::{check_help, check_key};
use crate::theme::{ThemeChoice, Tokens};
use reasoncraft_core::checks::CheckKind;

mod document;

/// A page beside the entries.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    /// Title, author and language of the document.
    Document,
    /// Personal preferences.
    Settings,
}

/// What a backstage entry does.
enum Entry {
    /// Back to the map, running the action (if any).
    Command(Option<Action>),
    /// Back to the map, on this ribbon tab.
    Ribbon(RibbonTab),
    /// Show (or hide) a page.
    Page(Page),
}

/// The page showing, if any (view state, kept in egui memory).
fn page_id() -> Id {
    Id::unique("backstage_page")
}

pub fn show(ui: &mut egui::Ui, app: &mut ReasonCraftApp) {
    let tk = app.tokens;
    let shown = egui::CentralPanel::default()
        .frame(Frame::new().fill(tk.panel).inner_margin(Margin::same(24)))
        .show(ui, |ui| {
            let mut page = ui.data(|d| d.get_temp::<Option<Page>>(page_id()).flatten());
            ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                ui.vertical(|ui| {
                    ui.set_width(220.0);
                    entries(ui, app, &mut page);
                });
                if let Some(page) = page {
                    ui.add_space(24.0);
                    ui.separator();
                    ui.add_space(24.0);
                    // Scrolls when it does not fit (at a large UI scale),
                    // each page on its own.
                    let scroll = egui::ScrollArea::vertical().id_salt(page == Page::Settings);
                    scroll.show(ui, |ui| {
                        ui.vertical(|ui| match page {
                            Page::Document => document::page(ui, app),
                            Page::Settings => settings_page(ui, app),
                        })
                    });
                }
            });
            ui.data_mut(|d| d.insert_temp(page_id(), page));
        });
    let pane = crate::shell::panes::Pane::Backstage;
    app.panes.shown(pane, shown.response.rect);
}

fn entries(ui: &mut egui::Ui, app: &mut ReasonCraftApp, page: &mut Option<Page>) {
    let tk = app.tokens;
    ui.spacing_mut().item_spacing.y = 4.0;
    // File commands take their label and action from the registry.
    let command = |icon, id| {
        let cmd = commands::get(id);
        (icon, cmd.label, Entry::Command(Some(cmd.action.clone())))
    };
    let items = [
        (Icon::Back, "backstage.back", Entry::Command(None)),
        command(Icon::New, "file.new"),
        command(Icon::Open, "file.open"),
        command(Icon::Open, "file.import_argdown"),
        command(Icon::Save, "file.save"),
        command(Icon::SaveAs, "file.save_as"),
        (
            Icon::ExportPng,
            "backstage.export",
            Entry::Ribbon(RibbonTab::Export),
        ),
        command(Icon::Print, "export.print"),
        (
            Icon::Note,
            "backstage.document",
            Entry::Page(Page::Document),
        ),
        (
            Icon::Settings,
            "backstage.settings",
            Entry::Page(Page::Settings),
        ),
    ];
    for (icon, key, entry) in items {
        let label = t(key);
        let on = matches!(entry, Entry::Page(p) if Some(p) == *page);
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
                Entry::Page(p) => *page = if *page == Some(p) { None } else { Some(p) },
                // Every file command returns to the map.
                Entry::Command(action) => {
                    app.ribbon_tab = RibbonTab::Home;
                    if let Some(action) = action {
                        app.push(action);
                    }
                }
                Entry::Ribbon(tab) => app.ribbon_tab = tab,
            }
            ui.ctx().request_repaint();
        }
    }
}

/// The interface language, the author name, the theme and interface
/// size, the default language, spellcheck, autosave, which checks run,
/// and Reset to defaults.
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
    ui_language(ui, app, &tk);
    ui.add_space(8.0);
    author_field(ui, app, &tk);
    ui.add_space(8.0);
    let group = t("settings.theme");
    ui.label(RichText::new(&group).color(tk.box_text));
    ui.horizontal(|ui| {
        let themes = [
            ("theme.system", ThemeChoice::System),
            ("theme.light", ThemeChoice::Light),
            ("theme.dark", ThemeChoice::Dark),
        ];
        for (key, choice) in themes {
            let resp = ui.selectable_label(app.settings.theme == choice, t(key));
            if labelled(resp, in_group(&group, &t(key))).clicked() {
                app.push(Action::SetTheme(choice));
            }
        }
    });
    ui.add_space(8.0);
    ui_scale(ui, app, &tk);
    ui.add_space(8.0);
    let group = t("settings.default_language");
    ui.label(RichText::new(&group).color(tk.box_text));
    let current = app.settings.default_doc_language.clone();
    if let Some(lang) = document::language_picker(ui, &group, &current) {
        app.push(Action::SetDefaultDocLanguage(lang));
    }
    ui.label(RichText::new(t("settings.default_language_hint")).color(tk.text_muted));
    ui.add_space(8.0);
    let mut spellcheck = app.settings.spellcheck;
    if ui
        .checkbox(&mut spellcheck, t("settings.spellcheck"))
        .on_hover_text(t("settings.spellcheck_hint"))
        .changed()
    {
        app.push(Action::SetSpellcheck(spellcheck));
    }
    ui.add_space(8.0);
    autosave(ui, app, &tk);
    ui.add_space(8.0);
    checks(ui, app, &tk);
    ui.add_space(16.0);
    if ui.button(t("settings.reset")).clicked() {
        app.push(Action::ResetSettings);
    }
    ui.label(RichText::new(t("settings.reset.hint")).color(tk.text_muted));
}

/// The size of the whole interface, 80 % to 200 % in steps of 10 %. A
/// drag applies when it ends: the slider would move under the pointer if
/// the interface grew while dragging. Keys apply at once.
fn ui_scale(ui: &mut egui::Ui, app: &mut ReasonCraftApp, tk: &Tokens) {
    let label = t("settings.ui_scale");
    ui.label(RichText::new(&label).color(tk.box_text));
    let percent = |scale: f32| (scale * 100.0).round() as u32;
    let dragging_id = Id::unique("settings_ui_scale_drag");
    let mut value = ui
        .data(|d| d.get_temp(dragging_id))
        .unwrap_or_else(|| percent(app.settings.ui_scale_factor()));
    let slider = egui::Slider::new(&mut value, percent(UI_SCALE_MIN)..=percent(UI_SCALE_MAX))
        .step_by(10.0)
        .suffix(t("settings.ui_scale_unit"));
    let resp = labelled(ui.add(slider), label);
    if resp.dragged() {
        ui.data_mut(|d| d.insert_temp(dragging_id, value));
    } else {
        ui.data_mut(|d| d.remove_temp::<u32>(dragging_id));
        if resp.changed() || resp.drag_stopped() {
            app.push(Action::SetUiScale(value as f32 / 100.0));
        }
    }
}

/// The interface language: System (the OS language) or one of the UI
/// languages, each named in its own language. Same commands as View →
/// Language.
fn ui_language(ui: &mut egui::Ui, app: &mut ReasonCraftApp, tk: &Tokens) {
    let group = t("settings.ui_language");
    ui.label(RichText::new(&group).color(tk.box_text));
    let current = app.settings.ui_language_choice();
    ui.horizontal(|ui| {
        let langs = UI_LANGUAGES.map(|l| (l, Some(l)));
        for (key, lang) in std::iter::once(("system", None)).chain(langs) {
            let cmd = commands::get(&format!("view.language.{key}"));
            let on = current == lang;
            let resp = ui.selectable_label(on, t(cmd.label));
            if labelled(resp, in_group(&group, &t(cmd.label))).clicked() && !on {
                app.push(cmd.action.clone());
            }
        }
    });
    ui.label(RichText::new(t("settings.ui_language_hint")).color(tk.text_muted));
}

/// Autosave on or off, and its interval in seconds. Turning it back on
/// brings back the interval it had (while the app runs).
fn autosave(ui: &mut egui::Ui, app: &mut ReasonCraftApp, tk: &Tokens) {
    let secs = app.settings.autosave_interval();
    let mut on = secs.is_some();
    let last_id = Id::unique("settings_autosave_last");
    ui.horizontal(|ui| {
        if ui.checkbox(&mut on, t("settings.autosave")).changed() {
            let secs = if on {
                ui.data(|d| d.get_temp(last_id)).unwrap_or(AUTOSAVE_DEFAULT)
            } else {
                ui.data_mut(|d| d.insert_temp(last_id, secs.unwrap_or(AUTOSAVE_DEFAULT)));
                0
            };
            app.push(Action::SetAutosaveSecs(secs));
        }
        if let Some(mut secs) = secs {
            let field = egui::DragValue::new(&mut secs)
                .range(AUTOSAVE_MIN..=AUTOSAVE_MAX)
                .suffix(t("settings.autosave_unit"));
            let resp = ui.add(field);
            resp.widget_info(|| {
                WidgetInfo::labeled(Role::SpinButton, true, t("settings.autosave_interval"))
            });
            if resp.changed() {
                app.push(Action::SetAutosaveSecs(secs));
            }
        }
    });
    ui.label(RichText::new(t("settings.autosave_hint")).color(tk.text_muted));
}

/// One checkbox per course-rule check, its rule (for the active
/// document's language) as the tooltip.
fn checks(ui: &mut egui::Ui, app: &mut ReasonCraftApp, tk: &Tokens) {
    let doc_lang = app.active_tab().doc.meta.language.clone();
    ui.label(RichText::new(t("settings.checks")).color(tk.box_text));
    ui.label(RichText::new(t("settings.checks_hint")).color(tk.text_muted));
    ui.spacing_mut().item_spacing.y = 4.0;
    for kind in CheckKind::ALL {
        let key = check_key(kind);
        let mut on = app.settings.checks.is_on(kind);
        if ui
            .checkbox(&mut on, t(&format!("check.kind.{key}")))
            .on_hover_text(check_help(kind, &doc_lang))
            .changed()
        {
            app.push(Action::SetCheckEnabled(kind, on));
        }
    }
}

/// The author name field. While it has focus it shows what is typed (so
/// a trailing space survives); every change is applied, trimmed.
fn author_field(ui: &mut egui::Ui, app: &mut ReasonCraftApp, tk: &Tokens) {
    let label = ui.label(RichText::new(t("settings.author")).color(tk.box_text));
    let id = Id::unique("settings_author");
    let focused = ui.memory(|m| m.has_focus(id));
    let mut text = if focused {
        ui.data(|d| d.get_temp::<String>(id))
            .unwrap_or_else(|| app.settings.author.clone())
    } else {
        app.settings.author.clone()
    };
    let response = ui
        .add(
            TextEdit::singleline(&mut text)
                .id(id)
                .hint_text(t("comment.anonymous"))
                .desired_width(280.0),
        )
        .labelled_by(label.id);
    if response.changed() {
        app.push(Action::SetAuthor(text.clone()));
    }
    ui.data_mut(|d| d.insert_temp(id, text));
    ui.label(RichText::new(t("settings.author_hint")).color(tk.text_muted));
}
