//! The window chrome around the map: title bar, tabs, ribbon, panels, status bar.

pub mod about;
pub mod backstage;
pub mod building;
pub mod command_search;
pub mod export_dialog;
pub mod find_bar;
pub mod icons;
pub mod link_modal;
pub mod modal;
mod panel_width;
pub mod panes;
pub mod resize;
pub mod ribbon;
pub mod source_icons;
pub mod spelling_panel;
pub mod statusbar;
pub mod tabs;
pub mod text_panel;
pub mod titlebar;

use eframe::egui::{
    self, Color32, FontId, Frame, Margin, Response, Role, Sense, Stroke, Vec2, WidgetInfo,
};

use crate::app::{ReasonCraftApp, RibbonTab};
use crate::spell::SpellFields;
use crate::theme::Tokens;
use icons::Icon;
use reasoncraft_core::examples::Template;

/// The i18n key and icon of a template (its hint is `<key>.hint`).
pub fn template_key(which: Template) -> (&'static str, Icon) {
    match which {
        Template::BlankReasoning => ("template.blank_reasoning", Icon::Reason),
        Template::BlankGrouping => ("template.blank_grouping", Icon::Category),
        Template::EssayPlan => ("template.essay_plan", Icon::Essay),
        Template::PeerReview => ("template.peer_review", Icon::Comment),
    }
}

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

/// One tab of a tab strip (ribbon, Text panel): its label, accent-coloured
/// and underlined when `active`, on `surface`; `pad` wider than the label
/// and `height` tall.
pub fn tab_button(
    ui: &mut egui::Ui,
    tk: &Tokens,
    label: &str,
    active: bool,
    surface: Color32,
    pad: f32,
    height: f32,
) -> Response {
    let galley =
        ui.painter()
            .layout_no_wrap(label.to_owned(), FontId::proportional(13.0), tk.box_text);
    let size = Vec2::new(galley.size().x + pad, height);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    resp.widget_info(|| WidgetInfo::selected(Role::Tab, true, active, label));
    if resp.hovered() && !active {
        ui.painter()
            .rect_filled(rect.shrink(2.0), 4.0, hover_fill(tk, surface));
    }
    let colour = if active { tk.accent } else { tk.box_text };
    let text_pos = rect.center() - galley.size() / 2.0;
    let text_w = galley.size().x;
    ui.painter().galley(text_pos, galley, colour);
    if active {
        let x = egui::Rangef::new(text_pos.x, text_pos.x + text_w);
        ui.painter()
            .hline(x, rect.bottom() - 3.0, Stroke::new(2.5, tk.accent));
    }
    resp
}

/// The narrowest a side panel gets, in px; dragged narrower, it collapses
/// to its toggle. A small window squeezes it further (see
/// [`panel_width_range`]).
pub const PANEL_MIN_WIDTH: f32 = 220.0;

/// The width the Building and the Text panel start at, in px.
const BUILDING_WIDTH: f32 = 220.0;
const TEXT_WIDTH: f32 = 320.0;

/// The narrowest the map gets beside open side panels, in px.
pub const MAP_MIN_WIDTH: f32 = 240.0;

/// The narrowest a squeezed side panel gets, in px, however small the
/// window.
const PANEL_SQUEEZED_WIDTH: f32 = 120.0;

/// The widths a side panel may take when `open` side panels share a
/// window `available` px wide: each at most an equal share of what the
/// map leaves, so the map keeps [`MAP_MIN_WIDTH`]; the usual minimum
/// gives way to that share in a small window.
pub fn panel_width_range(available: f32, open: usize) -> egui::Rangef {
    let share = (available - MAP_MIN_WIDTH) / open.max(1) as f32;
    let max = share.max(PANEL_SQUEEZED_WIDTH);
    egui::Rangef::new(PANEL_MIN_WIDTH.min(max), max)
}

/// Building panel (left) and Text panel (right).
/// Hidden while the backstage is open.
pub fn show_side_panels(ui: &mut egui::Ui, app: &mut ReasonCraftApp) {
    if app.ribbon_tab == RibbonTab::File {
        return;
    }
    let tk = app.tokens;
    let frame = Frame::new().fill(tk.panel).inner_margin(Margin::same(12));
    let tab = &mut app.tabs[app.active];
    let mut out = Vec::new();
    let open = usize::from(app.left_panel) + usize::from(app.right_panel);
    let widths = panel_width_range(ui.available_width(), open);
    use panel_width::{Side, keep_chosen};
    keep_chosen(ui, "building_panel", Side::Left, widths, BUILDING_WIDTH);
    keep_chosen(ui, "text_panel", Side::Right, widths, TEXT_WIDTH);
    let building = egui::Panel::left("building_panel")
        .frame(frame)
        .default_size(BUILDING_WIDTH)
        .size_range(widths)
        .show_collapsible(ui, &mut app.left_panel, |ui| {
            building::show(ui, tab, &tk, &mut out);
        });
    if let Some(shown) = building {
        app.panes.shown(panes::Pane::Building, shown.response.rect);
    }
    let text = egui::Panel::right("text_panel")
        .frame(frame)
        .default_size(TEXT_WIDTH)
        .size_range(widths)
        .show_collapsible(ui, &mut app.right_panel, |ui| {
            spelling_panel::show(ui, tab, &mut app.spell, &tk, &mut out);
            let lang = tab.doc.meta.language.clone();
            let mut spell = SpellFields::new(&mut app.spell, &app.settings, &lang, &tk);
            let cx = text_panel::PanelContext {
                settings: &app.settings,
                os: app.os,
                help: &app.help,
            };
            text_panel::show(ui, tab, cx, &tk, &mut spell, &mut out);
        });
    if let Some(shown) = text {
        app.panes.shown(panes::Pane::Text, shown.response.rect);
    }
    // egui names both resize handles "Resize panel". The handle's id is
    // built the way egui builds it; the names test in
    // tests/accessibility guards it (a changed id leaves two "Resize
    // panel" nodes).
    for (salt, key) in [
        ("building_panel", "a11y.resize_building"),
        ("text_panel", "a11y.resize_text"),
    ] {
        let handle = ui.scope_id().with(salt).with("__resize");
        crate::a11y::name_node(ui.ctx(), handle, &crate::i18n::t(key));
    }
    app.pending.extend(out);
}

/// The backstage when the File tab is open, otherwise the map area with
/// the find bar above it.
pub fn show_central(ui: &mut egui::Ui, app: &mut ReasonCraftApp) {
    if app.ribbon_tab == RibbonTab::File {
        backstage::show(ui, app);
        return;
    }
    find_bar::show(ui, app);
    let clip = crate::canvas::input::clipboard_events(ui.ctx(), app);
    app.pending.extend(clip);
    let tk = app.tokens;
    let map = egui::CentralPanel::default()
        .frame(Frame::new().fill(tk.canvas))
        .show(ui, |ui| crate::canvas::show(ui, app));
    app.panes.shown(panes::Pane::Map, map.response.rect);
    building::paint_ghost(ui.ctx(), &tk);
}
