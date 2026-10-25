//! The File backstage: a full-window page with file commands.

use eframe::egui::{self, Align2, FontId, Frame, Margin, Rect, Role, Sense, Vec2, WidgetInfo};

use super::hover_fill;
use super::icons::{self, Icon};
use crate::actions::Action;
use crate::app::{ReasonCraftApp, RibbonTab};
use crate::i18n::t;

pub fn show(ui: &mut egui::Ui, app: &mut ReasonCraftApp) {
    let tk = app.tokens;
    egui::CentralPanel::default()
        .frame(Frame::new().fill(tk.panel).inner_margin(Margin::same(24)))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            let items = [
                (Icon::Back, "backstage.back", None),
                (Icon::New, "action.new", Some(Action::New)),
                (Icon::Open, "action.open", Some(Action::Open)),
                (Icon::Save, "action.save", Some(Action::Save)),
                (Icon::SaveAs, "action.save_as", Some(Action::SaveAs)),
            ];
            for (icon, key, action) in items {
                let label = t(key);
                let (rect, resp) = ui.allocate_exact_size(Vec2::new(220.0, 40.0), Sense::click());
                resp.widget_info(|| WidgetInfo::labeled(Role::Button, true, &label));
                if resp.hovered() {
                    ui.painter()
                        .rect_filled(rect, 4.0, hover_fill(&tk, tk.panel));
                }
                let icon_rect = Rect::from_center_size(
                    rect.left_center() + Vec2::new(22.0, 0.0),
                    Vec2::splat(20.0),
                );
                icons::paint(ui.painter(), icon_rect, icon, tk.box_text, tk.panel, &tk);
                ui.painter().text(
                    rect.left_center() + Vec2::new(44.0, 0.0),
                    Align2::LEFT_CENTER,
                    &label,
                    FontId::proportional(14.0),
                    tk.box_text,
                );
                if resp.clicked() {
                    // Every backstage command returns to the map.
                    app.ribbon_tab = RibbonTab::Home;
                    if let Some(action) = action {
                        app.push(action);
                    }
                    ui.ctx().request_repaint();
                }
            }
        });
}
