//! The backstage entries down the left: file commands, the pages and the
//! way back to the map.

use eframe::egui::{self, Align2, FontId, Rect, Role, Sense, Vec2, WidgetInfo};

use super::Page;
use crate::actions::Action;
use crate::app::{ReasonCraftApp, RibbonTab};
use crate::commands;
use crate::i18n::t;
use crate::shell::hover_fill;
use crate::shell::icons::{self, Icon};

/// What a backstage entry does.
enum Entry {
    /// Back to the map, running the action (if any).
    Command(Option<Action>),
    /// Back to the map, on this ribbon tab.
    Ribbon(RibbonTab),
    /// Show a page.
    Page(Page),
}

/// The width of an entry: icon and label, or (`compact`) the icon alone.
pub(super) fn entry_width(compact: bool) -> f32 {
    if compact { 44.0 } else { 220.0 }
}

/// How many entries there are, and the space between two.
const COUNT: usize = 12;
const GAP: f32 = 4.0;

/// The height of an entry when the entries have `height` points: 40, or
/// less so that all of them fit (a short window), down to 24; below that
/// they scroll.
pub(super) fn entry_height(height: f32) -> f32 {
    ((height + GAP) / COUNT as f32 - GAP).clamp(24.0, 40.0)
}

/// The entries down the left, each `row_h` high (see [`entry_height`]);
/// `compact` shows their icons only, each label then in its tooltip (and
/// still its accessible name).
pub(super) fn entries(
    ui: &mut egui::Ui,
    app: &mut ReasonCraftApp,
    page: &mut Page,
    compact: bool,
    row_h: f32,
) {
    let tk = app.tokens;
    ui.spacing_mut().item_spacing.y = GAP;
    // File commands take their label and action from the registry.
    let command = |icon, id| {
        let cmd = commands::get(id);
        (icon, cmd.label, Entry::Command(Some(cmd.action.clone())))
    };
    let items: [_; COUNT] = [
        (Icon::Back, "backstage.back", Entry::Command(None)),
        (
            Icon::New,
            commands::get("file.new").label,
            Entry::Page(Page::New),
        ),
        command(Icon::Open, "file.open"),
        (Icon::Recent, "backstage.recent", Entry::Page(Page::Recent)),
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
        (Icon::About, "backstage.about", Entry::Page(Page::About)),
    ];
    for (icon, key, entry) in items {
        let label = t(key);
        let on = matches!(entry, Entry::Page(p) if p == *page);
        let size = Vec2::new(entry_width(compact), row_h);
        let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
        resp.widget_info(|| WidgetInfo::selected(Role::Button, true, on, &label));
        if resp.hovered() || on {
            ui.painter()
                .rect_filled(rect, 4.0, hover_fill(&tk, tk.panel));
        }
        let icon_rect =
            Rect::from_center_size(rect.left_center() + Vec2::new(22.0, 0.0), Vec2::splat(20.0));
        icons::paint(ui.painter(), icon_rect, icon, tk.box_text, tk.panel, &tk);
        let resp = if compact {
            // The label shows on hover, and on keyboard focus too.
            if resp.has_focus() {
                resp.show_tooltip_text(&label);
            }
            resp.on_hover_text(&label)
        } else {
            ui.painter().text(
                rect.left_center() + Vec2::new(44.0, 0.0),
                Align2::LEFT_CENTER,
                &label,
                FontId::proportional(14.0),
                tk.box_text,
            );
            resp
        };
        if resp.clicked() {
            match entry {
                Entry::Page(p) => *page = p,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_shrink_to_fit_a_short_window_then_scroll() {
        assert_eq!(entry_height(1000.0), 40.0);
        let fits = entry_height(340.0);
        assert!(fits < 40.0 && fits * COUNT as f32 + GAP * (COUNT - 1) as f32 <= 340.0 + 0.01);
        assert_eq!(entry_height(100.0), 24.0);
    }
}
