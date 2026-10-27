//! The row of open documents, with a "+" to start a new one.

use eframe::egui::{
    self, Align2, CornerRadius, FontId, Frame, Margin, Role, Sense, Vec2, WidgetInfo,
};

use super::icons::Icon;
use super::{hover_fill, icon_button};
use crate::actions::Action;
use crate::app::ReasonCraftApp;
use crate::i18n::{FluentArgs, t, t_args};

const HEIGHT: f32 = 30.0;
/// The most characters of a document's name a tab shows.
const TITLE_CHARS: usize = 24;
/// What follows the name of a document with unsaved changes.
const DIRTY_MARK: &str = " •";

/// `title` as a tab shows it: a name longer than [`TITLE_CHARS`] is cut
/// short with "…" (keeping the unsaved-changes mark).
pub fn short_title(title: &str) -> String {
    let (name, mark) = match title.strip_suffix(DIRTY_MARK) {
        Some(name) => (name, DIRTY_MARK),
        None => (title, ""),
    };
    match name.char_indices().nth(TITLE_CHARS) {
        Some((end, _)) => format!("{}…{mark}", name[..end].trim_end()),
        None => title.to_owned(),
    }
}

pub fn show(ui: &mut egui::Ui, app: &mut ReasonCraftApp) {
    let tk = app.tokens;
    egui::Panel::top("doc_tabs")
        .frame(
            Frame::new()
                .fill(tk.bg)
                .inner_margin(Margin::symmetric(8, 0)),
        )
        .exact_size(HEIGHT)
        .show_separator_line(false)
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                for i in 0..app.tabs.len() {
                    tab_chip(ui, app, i);
                }
                if icon_button(ui, Icon::Plus, &t("tabs.new"), 24.0, true, &tk, tk.bg).clicked() {
                    app.push(Action::New);
                }
            });
        });
}

fn tab_chip(ui: &mut egui::Ui, app: &mut ReasonCraftApp, i: usize) {
    let tk = app.tokens;
    let title = app.tabs[i].title();
    let shown = short_title(&title);
    let active = i == app.active;
    let font = FontId::proportional(13.0);
    let text_w = ui
        .painter()
        .layout_no_wrap(shown.clone(), font.clone(), tk.box_text)
        .size()
        .x;
    let width = (text_w + 52.0).clamp(120.0, 240.0);
    let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(width, HEIGHT - 4.0), Sense::click());
    resp.widget_info(|| WidgetInfo::selected(Role::Tab, true, active, &title));
    if shown != title {
        resp = resp.on_hover_text(&title);
    }

    let fill = if active {
        tk.ribbon
    } else if resp.hovered() {
        hover_fill(&tk, tk.bg)
    } else {
        tk.bg
    };
    let radius = CornerRadius {
        nw: 6,
        ne: 6,
        sw: 0,
        se: 0,
    };
    ui.painter().rect_filled(rect, radius, fill);
    let colour = if active { tk.box_text } else { tk.text_muted };
    ui.painter().text(
        rect.left_center() + Vec2::new(12.0, 0.0),
        Align2::LEFT_CENTER,
        &shown,
        font,
        colour,
    );

    // Close button on the right of the chip.
    let mut args = FluentArgs::new();
    args.set("name", title);
    let close_label = t_args("tabs.close", &args);
    let close_rect = egui::Rect::from_center_size(
        rect.right_center() - Vec2::new(16.0, 0.0),
        Vec2::splat(20.0),
    );
    let surface = fill;
    let close = ui.put(close_rect, |ui: &mut egui::Ui| {
        icon_button(ui, Icon::Close, &close_label, 20.0, true, &tk, surface)
    });

    if close.clicked() {
        app.push(Action::CloseTab(i));
    } else if resp.clicked() {
        app.push(Action::ActivateTab(i));
    }
}
