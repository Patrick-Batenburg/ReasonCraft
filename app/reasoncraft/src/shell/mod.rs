//! The window chrome around the map: title bar, tabs, ribbon, panels, status bar.

pub mod backstage;
pub mod icons;
pub mod ribbon;
pub mod statusbar;
pub mod tabs;
pub mod titlebar;

use eframe::egui::{
    self, Color32, CornerRadius, FontId, Frame, Margin, Response, Role, Sense, Stroke, StrokeKind,
    Vec2, WidgetInfo,
};
use reasoncraft_core::Item;

use crate::actions::Action;
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
        .show(ui, |ui| map_placeholder(ui, app, &tk));
}

/// Until the canvas lands (Task 9): the claim box, and a click target to
/// clear the selection.
fn map_placeholder(ui: &mut egui::Ui, app: &mut ReasonCraftApp, tk: &Tokens) {
    let area = ui.max_rect();
    let background = ui.interact(area, ui.scope_id().with("canvas_bg"), Sense::click());
    let tab = app.active_tab();
    let root = tab.doc.root;
    let text = tab
        .doc
        .boxes
        .get(&root)
        .map(|b| b.text.clone())
        .unwrap_or_default();
    let selected = tab.selection == Some(Item::Box(root));
    let (shown, colour) = if text.is_empty() {
        (t("canvas.empty_claim"), tk.text_muted)
    } else {
        (text, tk.box_text)
    };

    let galley = ui
        .painter()
        .layout(shown.clone(), FontId::proportional(15.0), colour, 260.0);
    let size = galley.size() + Vec2::new(32.0, 24.0);
    let rect = egui::Rect::from_center_size(
        area.center_top() + Vec2::new(0.0, 60.0 + size.y / 2.0),
        size,
    );
    let claim = ui.interact(rect, ui.scope_id().with("claim_box"), Sense::click());
    claim.widget_info(|| WidgetInfo::selected(Role::Button, true, selected, &shown));
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(6), tk.box_fill);
    let border = if selected {
        Stroke::new(2.0, tk.accent)
    } else {
        Stroke::new(1.25, tk.claim_border)
    };
    painter.rect_stroke(rect, CornerRadius::same(6), border, StrokeKind::Inside);
    painter.galley(rect.center() - galley.size() / 2.0, galley, colour);

    if claim.clicked() {
        app.push(Action::Select(Some(Item::Box(root))));
    } else if background.clicked() {
        app.push(Action::Select(None));
    }
}
