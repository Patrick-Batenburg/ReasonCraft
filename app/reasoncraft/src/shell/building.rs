//! The building panel: boxes, sources, comment presets and templates. A
//! click inserts at the selection (like the ribbon); a drag carries the
//! item to the map, where the canvas shows where it would land. A
//! template opens as a new map.

use eframe::egui::{
    self, Align, CollapsingHeader, Color32, CursorIcon, DragAndDrop, FontId, Id, LayerId, Order,
    Painter, Rect, RichText, Role, ScrollArea, Sense, Stroke, StrokeKind, Ui, WidgetInfo,
    text::{LayoutJob, TextFormat, TextWrapping},
    vec2,
};
use reasoncraft_core::SourceType;
use reasoncraft_core::examples::Template;

use super::hover_fill;
use super::icons::{self, Icon};
use super::source_icons::paint_source_icon;
use crate::actions::{Action, Insertable, is_enabled};
use crate::canvas::comments::{PRESETS, verdict_colour};
use crate::doc_tab::DocTab;
use crate::i18n::{FluentArgs, t, t_args};
use crate::theme::Tokens;

const BOXES: [Insertable; 5] = [
    Insertable::Reason,
    Insertable::Objection,
    Insertable::CoPremise,
    Insertable::Category,
    Insertable::Note,
];
/// Glyph size in a row and in a source tile.
const ROW_GLYPH: f32 = 16.0;
const TILE_GLYPH: f32 = 22.0;
const ROW_FONT: f32 = 13.0;
/// Tile font sizes, largest first: a long word gets a smaller size.
const TILE_FONTS: [f32; 3] = [11.0, 10.0, 9.0];
const TILE_H: f32 = 60.0;
const PAD: f32 = 6.0;
const GAP: f32 = 4.0;

/// The panel's name for `what` (and its tooltip and accessible name).
pub fn label(what: &Insertable) -> String {
    match what {
        Insertable::Reason => t("action.add_reason"),
        Insertable::Objection => t("action.add_objection"),
        Insertable::CoPremise => t("action.add_co_premise"),
        Insertable::Category => t("action.add_category"),
        Insertable::Note => t("action.add_note"),
        Insertable::Source(st) => t(&format!("source.{}", st.key())),
        Insertable::Comment(i) => PRESETS.get(*i).map_or_else(String::new, |p| t(p.0)),
        Insertable::Text { text, .. } => super::text_panel::excerpt(text),
    }
}

/// Draw the glyph of `what` in `rect`: the ribbon icon, the source glyph,
/// a speech bubble in the colour of the preset's verdict, or for text the
/// glyph of its source (a reason without one).
fn paint_glyph(painter: &Painter, rect: Rect, what: &Insertable, enabled: bool, tk: &Tokens) {
    let fg = if enabled { tk.box_text } else { tk.text_muted };
    let icon = match what {
        Insertable::Reason => Icon::Reason,
        Insertable::Objection => Icon::Objection,
        Insertable::CoPremise => Icon::CoPremise,
        Insertable::Category => Icon::Category,
        Insertable::Note => Icon::Note,
        Insertable::Source(st) => return paint_source_icon(painter, rect, st, fg),
        Insertable::Comment(i) => {
            let colour = verdict_colour(PRESETS.get(*i).map(|p| p.1), tk);
            let colour = if enabled {
                colour
            } else {
                colour.gamma_multiply(0.55)
            };
            return icons::paint(painter, rect, Icon::Comment, colour, tk.panel, tk);
        }
        Insertable::Text {
            source: Some(src), ..
        } => return paint_source_icon(painter, rect, &src.source_type, fg),
        Insertable::Text { source: None, .. } => Icon::Reason,
    };
    icons::paint(painter, rect, icon, fg, tk.panel, tk);
}

/// The building panel's contents. Clicks push [`Action::Insert`] at the
/// selection into `out`; drags set an egui drag-and-drop payload.
pub fn show(ui: &mut Ui, tab: &DocTab, tk: &Tokens, out: &mut Vec<Action>) {
    ui.label(
        RichText::new(t("panel.building"))
            .strong()
            .size(15.0)
            .color(tk.box_text),
    );
    ui.add_space(4.0);
    ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        section(ui, tk, "building.boxes", |ui| {
            for what in BOXES {
                row(ui, tab, tk, what, out);
            }
        });
        section(ui, tk, "building.sources", |ui| {
            let w = ((ui.available_width() - 2.0 * GAP) / 3.0).floor();
            for chunk in SourceType::ALL.chunks(3) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = GAP;
                    for st in chunk {
                        tile(ui, tab, tk, Insertable::Source(st.clone()), w, out);
                    }
                });
            }
        });
        section(ui, tk, "building.comments", |ui| {
            for i in 0..PRESETS.len() {
                row(ui, tab, tk, Insertable::Comment(i), out);
            }
        });
        section(ui, tk, "building.templates", |ui| {
            for which in Template::ALL {
                template_row(ui, tk, which, out);
            }
        });
    });
}

/// A template: its icon and name; a click opens it as a new map.
fn template_row(ui: &mut Ui, tk: &Tokens, which: Template, out: &mut Vec<Action>) {
    let (key, icon) = super::template_key(which);
    let name = t(key);
    let width = ui.available_width();
    let text_w = width - ROW_GLYPH - 3.0 * PAD;
    let galley = ui.fonts_mut(|f| f.layout_job(text_job(&name, ROW_FONT, tk.box_text, text_w, 3)));
    let h = (galley.size().y + 2.0 * PAD).max(26.0);
    let (rect, resp) = ui.allocate_exact_size(vec2(width, h), Sense::click());
    let glyph = Rect::from_center_size(
        egui::pos2(rect.left() + PAD + ROW_GLYPH / 2.0, rect.center().y),
        egui::Vec2::splat(ROW_GLYPH),
    );
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        backdrop(painter, rect, &resp, tk);
        icons::paint(painter, glyph, icon, tk.box_text, tk.panel, tk);
        let text_pos = egui::pos2(glyph.right() + PAD, rect.center().y - galley.size().y / 2.0);
        painter.galley(text_pos, galley, tk.box_text);
    }
    let mut args = FluentArgs::new();
    args.set("item", name.as_str());
    let spoken = t_args("building.item_name", &args);
    resp.widget_info(|| WidgetInfo::labeled(Role::Button, true, &spoken));
    let resp = resp.on_hover_text(t(&format!("{key}.hint")));
    if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
        out.push(Action::OpenTemplate(which));
    }
}

/// A collapsible section, open at first.
fn section(ui: &mut Ui, tk: &Tokens, key: &str, body: impl FnOnce(&mut Ui)) {
    CollapsingHeader::new(RichText::new(t(key)).strong().color(tk.box_text))
        .id_salt(key)
        .default_open(true)
        .show(ui, body);
}

/// Whether a click on `what` would insert it at the selection.
fn enabled(tab: &DocTab, what: &Insertable) -> bool {
    let action = Action::Insert {
        what: what.clone(),
        to: None,
        on: None,
    };
    is_enabled(tab, &action)
}

/// Text laid out in `width`, at most `rows` rows (then "…").
fn text_job(text: &str, size: f32, colour: Color32, width: f32, rows: usize) -> LayoutJob {
    let mut job = LayoutJob::single_section(
        text.to_owned(),
        TextFormat::simple(FontId::proportional(size), colour),
    );
    job.wrap = TextWrapping {
        max_width: width,
        max_rows: rows,
        break_anywhere: false,
        overflow_character: Some('…'),
    };
    job
}

/// A full-width item: glyph, then its name (wrapping).
fn row(ui: &mut Ui, tab: &DocTab, tk: &Tokens, what: Insertable, out: &mut Vec<Action>) {
    let on = enabled(tab, &what);
    let name = label(&what);
    let fg = if on { tk.box_text } else { tk.text_muted };
    let width = ui.available_width();
    let text_w = width - ROW_GLYPH - 3.0 * PAD;
    let galley = ui.fonts_mut(|f| f.layout_job(text_job(&name, ROW_FONT, fg, text_w, 3)));
    let h = (galley.size().y + 2.0 * PAD).max(26.0);
    let (rect, resp) = ui.allocate_exact_size(vec2(width, h), Sense::click_and_drag());
    let glyph = Rect::from_center_size(
        egui::pos2(rect.left() + PAD + ROW_GLYPH / 2.0, rect.center().y),
        egui::Vec2::splat(ROW_GLYPH),
    );
    let text_pos = egui::pos2(glyph.right() + PAD, rect.center().y - galley.size().y / 2.0);
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        backdrop(painter, rect, &resp, tk);
        paint_glyph(painter, glyph, &what, on, tk);
        painter.galley(text_pos, galley, fg);
    }
    finish(resp, what, on, name, out);
}

/// A source tile: glyph above its name (up to two rows, then "…").
fn tile(ui: &mut Ui, tab: &DocTab, tk: &Tokens, what: Insertable, w: f32, out: &mut Vec<Action>) {
    let on = enabled(tab, &what);
    let name = label(&what);
    let fg = if on { tk.box_text } else { tk.text_muted };
    let (rect, resp) = ui.allocate_exact_size(vec2(w, TILE_H), Sense::click_and_drag());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        backdrop(painter, rect, &resp, tk);
        let glyph = Rect::from_center_size(
            egui::pos2(rect.center().x, rect.top() + PAD + TILE_GLYPH / 2.0),
            egui::Vec2::splat(TILE_GLYPH),
        );
        paint_glyph(painter, glyph, &what, on, tk);
        let size = tile_font(ui, &name, w - 4.0);
        let mut job = text_job(&name, size, fg, w - 4.0, 2);
        job.halign = Align::Center;
        let galley = ui.fonts_mut(|f| f.layout_job(job));
        painter.galley(
            egui::pos2(rect.center().x, glyph.bottom() + 3.0),
            galley,
            fg,
        );
    }
    finish(resp, what, on, name, out);
}

/// The largest tile font size (down to a floor) at which no word of `name`
/// has to be split over two rows.
fn tile_font(ui: &Ui, name: &str, width: f32) -> f32 {
    let fits = |size: f32| {
        ui.fonts_mut(|f| {
            name.split_whitespace().all(|word| {
                let font = FontId::proportional(size);
                f.layout_no_wrap(word.to_owned(), font, Color32::PLACEHOLDER)
                    .size()
                    .x
                    <= width
            })
        })
    };
    TILE_FONTS
        .into_iter()
        .find(|&size| fits(size))
        .unwrap_or(TILE_FONTS[TILE_FONTS.len() - 1])
}

/// Hover background of an item.
fn backdrop(painter: &Painter, rect: Rect, resp: &egui::Response, tk: &Tokens) {
    if resp.hovered() || resp.dragged() {
        painter.rect_filled(rect, 4.0, hover_fill(tk, tk.panel));
    }
}

/// Accessibility, tooltip, click to insert at the selection, drag payload.
/// Its accessible name says where it is, as the ribbon has items of the
/// same name.
fn finish(resp: egui::Response, what: Insertable, on: bool, name: String, out: &mut Vec<Action>) {
    let mut args = FluentArgs::new();
    args.set("item", name.as_str());
    let spoken = t_args("building.item_name", &args);
    resp.widget_info(|| WidgetInfo::labeled(Role::Button, on, &spoken));
    let resp = resp.on_hover_text(&name);
    if on {
        let resp = resp.clone().on_hover_cursor(CursorIcon::PointingHand);
        if resp.clicked() {
            out.push(Action::Insert {
                what: what.clone(),
                to: None,
                on: None,
            });
        }
    }
    resp.dnd_set_drag_payload(what);
}

fn ghost_tint_id() -> Id {
    Id::unique("building_ghost_tint")
}

/// Colour this frame's ghost chip (the canvas: green or red over a target).
pub fn tint_ghost(ctx: &egui::Context, colour: Color32) {
    ctx.data_mut(|d| d.insert_temp(ghost_tint_id(), colour));
}

/// The chip that follows the pointer while an item is dragged out of the
/// panel, outlined in the tint the canvas chose (neutral elsewhere). Call
/// after the canvas.
pub fn paint_ghost(ctx: &egui::Context, tk: &Tokens) {
    let tint = ctx.data_mut(|d| d.remove_temp::<Color32>(ghost_tint_id()));
    let Some(what) = DragAndDrop::payload::<Insertable>(ctx) else {
        return;
    };
    let (pos, released) = ctx.input(|i| (i.pointer.latest_pos(), i.pointer.any_released()));
    let Some(pos) = pos.filter(|_| !released) else {
        return;
    };
    let painter = ctx.layer_painter(LayerId::new(Order::Tooltip, Id::unique("building_ghost")));
    let galley = painter.layout_job(text_job(&label(&what), ROW_FONT, tk.box_text, 200.0, 1));
    let size = vec2(ROW_GLYPH + galley.size().x + 3.0 * PAD, 28.0);
    let rect = Rect::from_min_size(pos + vec2(14.0, 12.0), size);
    let shadow = ctx.global_style().visuals.popup_shadow;
    painter.add(shadow.as_shape(rect, 6.0));
    let stroke = Stroke::new(1.5, tint.unwrap_or(tk.accent));
    painter.rect(rect, 6.0, tk.panel, stroke, StrokeKind::Outside);
    let glyph = Rect::from_center_size(
        egui::pos2(rect.left() + PAD + ROW_GLYPH / 2.0, rect.center().y),
        egui::Vec2::splat(ROW_GLYPH),
    );
    paint_glyph(&painter, glyph, &what, true, tk);
    let text_pos = egui::pos2(glyph.right() + PAD, rect.center().y - galley.size().y / 2.0);
    painter.galley(text_pos, galley, tk.box_text);
}
