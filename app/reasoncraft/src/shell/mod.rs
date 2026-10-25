//! The window chrome around the map: title bar, tabs, ribbon, panels, status bar.

pub mod backstage;
pub mod icons;
pub mod modal;
pub mod ribbon;
pub mod statusbar;
pub mod tabs;
pub mod titlebar;

use eframe::egui::{self, Color32, Frame, Margin, Response, Role, Sense, Vec2, WidgetInfo};

use crate::app::{ReasonCraftApp, RibbonTab};
use crate::i18n::t;
use crate::theme::Tokens;
use icons::Icon;

/// Opaque background of a hovered control on `surface`.
pub fn hover_fill(t: &Tokens, surface: Color32) -> Color32 {
    surface.lerp_to_gamma(t.box_text, 0.1)
}

/// Opaque background of a control that is "on" (a selected toggle), on `surface`.
pub fn selected_fill(t: &Tokens, surface: Color32) -> Color32 {
    surface.lerp_to_gamma(t.accent, 0.18)
}

/// A square icon-only button. `label` is its accessible name and tooltip.
pub fn icon_button(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    size: f32,
    enabled: bool,
    t: &Tokens,
    surface: Color32,
) -> Response {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), sense);
    resp.widget_info(|| WidgetInfo::labeled(Role::Button, enabled, label));
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let mut surface = surface;
        if enabled && resp.hovered() {
            surface = hover_fill(t, surface);
            painter.rect_filled(rect, 4.0, surface);
        }
        let fg = if enabled { t.box_text } else { t.text_muted };
        let inner = rect.shrink(size * 0.22);
        icons::paint(painter, inner, icon, fg, surface, t);
    }
    resp.on_hover_text(label)
}

/// Building panel (left) and Text panel (right): placeholders in step 1.
/// Hidden while the backstage is open.
pub fn show_side_panels(ui: &mut egui::Ui, app: &mut ReasonCraftApp) {
    if app.ribbon_tab == RibbonTab::File {
        return;
    }
    let tk = app.tokens;
    let frame = Frame::new().fill(tk.panel).inner_margin(Margin::same(12));
    let body = |ui: &mut egui::Ui, heading: &str| {
        ui.label(
            egui::RichText::new(heading)
                .strong()
                .size(15.0)
                .color(tk.box_text),
        );
        ui.add_space(6.0);
        ui.label(egui::RichText::new(t("panel.coming_later")).color(tk.text_muted));
        ui.take_available_space();
    };
    egui::Panel::left("building_panel")
        .frame(frame)
        .default_size(220.0)
        .show_collapsible(ui, &mut app.left_panel, |ui| body(ui, &t("panel.building")));
    egui::Panel::right("text_panel")
        .frame(frame)
        .default_size(260.0)
        .show_collapsible(ui, &mut app.right_panel, |ui| body(ui, &t("panel.text")));
}

/// The backstage when the File tab is open, otherwise the map area.
pub fn show_central(ui: &mut egui::Ui, app: &mut ReasonCraftApp) {
    if app.ribbon_tab == RibbonTab::File {
        backstage::show(ui, app);
        return;
    }
    let tk = app.tokens;
    egui::CentralPanel::default()
        .frame(Frame::new().fill(tk.canvas))
        .show(ui, |ui| crate::canvas::show(ui, app));
}
