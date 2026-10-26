//! The pieces of the Essay tab: a map statement (role-coloured bar, as in
//! the outline) that selects its item when clicked, and the paragraph field
//! under it.

use eframe::egui::{
    self, Color32, FontId, Id, Rect, Role, Sense, TextEdit, Ui, WidgetInfo, pos2, text::LayoutJob,
    vec2,
};
use reasoncraft_core::essay::EssaySlot;
use reasoncraft_core::{Item, SourceType};

use crate::actions::Action;
use crate::canvas::rating::{Mark, paint_mark};
use crate::doc_tab::DocTab;
use crate::i18n::t;
use crate::shell::source_icons::paint_source_icon;
use crate::shell::{hover_fill, selected_fill};
use crate::spell::{FieldStyle, SpellFields};
use crate::theme::Tokens;

/// Indent per depth level.
pub(super) const INDENT: f32 = 14.0;
/// Room the text always keeps, however deep the element.
const MIN_TEXT_W: f32 = 120.0;
const BAR_W: f32 = 3.0;
const ICON: f32 = 12.0;
const MARK: f32 = 10.0;
const GAP: f32 = 6.0;
const PAD_Y: f32 = 3.0;
const FONT: f32 = 13.0;

/// The indent for `depth` in a column `width` wide.
pub(super) fn indent(depth: usize, width: f32) -> f32 {
    (depth as f32 * INDENT).min((width - MIN_TEXT_W).max(0.0))
}

/// How a statement is drawn.
pub(super) struct Statement<'a> {
    pub item: Item,
    pub text: &'a str,
    pub depth: usize,
    /// The colour of the bar at the left edge; `None` for no bar.
    pub bar: Option<Color32>,
    /// A source's type, shown as its icon.
    pub source: Option<&'a SourceType>,
    pub mark: Option<Mark>,
    pub selected: bool,
}

/// A map statement, wrapped to the panel's width. Clicking it selects the
/// item on the map and reveals it.
pub(super) fn statement(ui: &mut Ui, s: &Statement, tk: &Tokens, out: &mut Vec<Action>) {
    let width = ui.available_width();
    let left = 4.0 + indent(s.depth, width) + BAR_W + GAP;
    let icon_w = if s.source.is_some() { ICON + 4.0 } else { 0.0 };
    let right_w = 4.0 + if s.mark.is_some() { MARK + GAP } else { 0.0 };
    let wrap = (width - left - icon_w - right_w).max(1.0);
    let one_line = s.text.split_whitespace().collect::<Vec<_>>().join(" ");
    let (text, colour) = if one_line.is_empty() {
        (t("outline.empty"), tk.text_muted)
    } else {
        (one_line, tk.box_text)
    };
    let mut job = LayoutJob::simple(text.clone(), FontId::proportional(FONT), colour, wrap);
    job.halign = egui::Align::LEFT;
    let galley = ui.painter().layout_job(job);
    let height = galley.size().y + 2.0 * PAD_Y;
    let (rect, resp) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    resp.widget_info(|| WidgetInfo::selected(Role::ListItem, true, s.selected, &text));
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if s.selected {
            painter.rect_filled(rect, 3.0, selected_fill(tk, tk.panel));
        } else if resp.hovered() {
            painter.rect_filled(rect, 3.0, hover_fill(tk, tk.panel));
        }
        let mut x = rect.left() + left - BAR_W - GAP;
        if let Some(bar) = s.bar {
            let r = Rect::from_min_max(
                pos2(x, rect.top() + PAD_Y),
                pos2(x + BAR_W, rect.bottom() - PAD_Y),
            );
            painter.rect_filled(r, 1.0, bar);
        }
        x += BAR_W + GAP;
        let first_line = rect.top() + PAD_Y + galley.rows.first().map_or(0.0, |r| r.height()) / 2.0;
        if let Some(source) = s.source {
            let icon = Rect::from_center_size(pos2(x + ICON / 2.0, first_line), vec2(ICON, ICON));
            paint_source_icon(painter, icon, source, tk.text_muted);
            x += icon_w;
        }
        painter.galley(pos2(x, rect.top() + PAD_Y), galley, colour);
        if let Some(mark) = s.mark {
            let c = pos2(rect.right() - 4.0 - MARK / 2.0, first_line);
            paint_mark(painter, c, MARK / 2.0, mark, tk);
        }
    }
    if resp.clicked() {
        out.push(Action::Select(Some(s.item)));
        out.push(Action::RevealSelection);
    }
}

/// The widget id of `tab`'s paragraph field for `slot`.
pub fn paragraph_id(tab: &DocTab, slot: EssaySlot) -> Id {
    tab.widget_id(("essay_paragraph", slot))
}

/// A paragraph field: its slot, indent depth, accessible name and hint.
pub(super) struct Paragraph<'a> {
    pub slot: EssaySlot,
    pub depth: usize,
    /// Names it for screen readers.
    pub label: &'a str,
    /// Shows while it is empty.
    pub hint: &'a str,
}

/// `tab`'s paragraph field `p`, holding the slot's text, misspellings
/// underlined. Changes go out live; losing focus ends the editing session.
pub(super) fn paragraph(
    ui: &mut Ui,
    tab: &DocTab,
    p: Paragraph,
    spell: &mut SpellFields,
    out: &mut Vec<Action>,
) {
    let Paragraph {
        slot,
        depth,
        label,
        hint,
    } = p;
    let pad = 4.0 + indent(depth, ui.available_width()) + BAR_W + GAP;
    ui.horizontal(|ui| {
        ui.add_space(pad);
        let mut buffer = tab.doc.essay.text(slot).unwrap_or("").to_owned();
        let mut layouter = spell.layouter(FieldStyle::default());
        let resp = ui.add(
            TextEdit::multiline(&mut buffer)
                .id(paragraph_id(tab, slot))
                .layouter(&mut layouter)
                .hint_text(hint)
                .desired_width(f32::INFINITY)
                .desired_rows(2),
        );
        resp.widget_info(|| WidgetInfo::labeled(Role::MultilineTextInput, true, label));
        if resp.changed() {
            out.push(Action::EditEssayText { slot, text: buffer });
        }
        if resp.lost_focus() {
            out.push(Action::StopEditing);
        }
    });
}
