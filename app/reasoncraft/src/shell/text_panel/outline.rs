//! The Outline tab: the map as numbered, indented text lines with a toolbar
//! of switches and "Copy all". Rows are virtualised and the outline is
//! cached per document revision (see [`DocTab::outline`]).

use eframe::egui::{
    self, Align2, Color32, FontId, Id, Rect, Response, Role, ScrollArea, Sense, Ui, WidgetInfo,
    pos2, text::LayoutJob, vec2,
};
use reasoncraft_core::outline::{LineKind, LinkLabel, OutlineLine, outline_plain_text};
use reasoncraft_core::{Item, SourceType};

use crate::actions::Action;
use crate::canvas::rating::{Mark, card_colour, mark_for, paint_mark};
use crate::doc_tab::DocTab;
use crate::i18n::t;
use crate::settings::{OutlineToggle, Settings};
use crate::shell::source_icons::paint_source_icon;
use crate::shell::{hover_fill, selected_fill};
use crate::theme::Tokens;

/// Height of one row; fixed, so `show_rows` can place rows without
/// measuring them (long text is cut to one line).
const ROW_H: f32 = 22.0;
/// Indent per depth level.
const INDENT: f32 = 14.0;
/// Room the text always keeps, however deep the line.
const MIN_TEXT_W: f32 = 80.0;
const BAR_W: f32 = 3.0;
const ICON: f32 = 12.0;
/// Diameter of the trailing rating marker.
const MARK: f32 = 10.0;
const GAP: f32 = 6.0;
const FONT: f32 = 13.0;
/// How long "Copied" shows after "Copy all".
const COPIED_SECS: f64 = 2.0;

const TOGGLES: [(OutlineToggle, &str); 5] = [
    (OutlineToggle::Numbering, "numbering"),
    (OutlineToggle::Labels, "labels"),
    (OutlineToggle::Indent, "indent"),
    (OutlineToggle::Colour, "colour"),
    (OutlineToggle::Notes, "notes"),
];

/// The linking word shown for `label` ("" for none).
pub fn label_word(label: LinkLabel) -> String {
    match label {
        LinkLabel::Because => t("outline.label.because"),
        LinkLabel::But => t("outline.label.but"),
        LinkLabel::However => t("outline.label.however"),
        LinkLabel::None => String::new(),
    }
}

/// The name of a source type, as on the canvas.
pub fn source_word(source_type: &SourceType) -> String {
    t(&format!("source.{}", source_type.key()))
}

/// Draw the Outline tab for `tab`.
pub fn show(
    ui: &mut Ui,
    tab: &mut DocTab,
    settings: &Settings,
    tk: &Tokens,
    out: &mut Vec<Action>,
) {
    tab.outline(settings.outline);
    let tab = &*tab;
    let lines = tab.outline_cache.as_ref().map_or(&[][..], |(_, _, l)| l);
    let (doc, selection) = (&tab.doc, tab.selection);
    toolbar(ui, lines, settings, tk, out);
    ui.add_space(6.0);
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show_rows(ui, ROW_H, lines.len(), |ui, range| {
                for line in &lines[range] {
                    let selected = selection == Some(line.item);
                    let mark = match line.item {
                        Item::Box(b) if settings.show_markers => {
                            doc.boxes.get(&b).and_then(mark_for)
                        }
                        _ => None,
                    };
                    if row(ui, line, mark, selected, settings, tk).clicked() {
                        out.push(Action::Select(Some(line.item)));
                        out.push(Action::RevealSelection);
                    }
                }
            });
    });
}

fn toolbar(
    ui: &mut Ui,
    lines: &[OutlineLine],
    settings: &Settings,
    tk: &Tokens,
    out: &mut Vec<Action>,
) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (toggle, key) in TOGGLES {
            let on = settings.outline_toggle(toggle);
            let resp = ui
                .selectable_label(on, t(&format!("outline.toggle.{key}")))
                .on_hover_text(t(&format!("outline.tip.{key}")));
            if resp.clicked() {
                out.push(Action::SetOutlineOption(toggle, !on));
            }
        }
        let copied_id = Id::unique("outline_copied_at");
        if ui
            .button(t("outline.copy_all"))
            .on_hover_text(t("outline.tip.copy_all"))
            .clicked()
        {
            let text = outline_plain_text(lines, &settings.outline, &label_word, &source_word);
            ui.ctx().copy_text(text);
            let now = ui.input(|i| i.time);
            ui.data_mut(|d| d.insert_temp(copied_id, now));
        }
        let now = ui.input(|i| i.time);
        let since = ui.data(|d| d.get_temp::<f64>(copied_id)).map(|at| now - at);
        if let Some(since) = since.filter(|s| *s < COPIED_SECS) {
            ui.label(egui::RichText::new(t("outline.copied")).color(tk.text_muted));
            let left = (COPIED_SECS - since) as f32;
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_secs_f32(left));
        }
    });
}

/// The words of a row before its text: number, linking word, source type.
fn lead(line: &OutlineLine, settings: &Settings) -> (String, String) {
    let opts = settings.outline;
    let number = if opts.numbering {
        line.number.clone()
    } else {
        String::new()
    };
    let word = match &line.kind {
        LineKind::Source(source_type) => source_word(source_type),
        _ if opts.labels => label_word(line.label),
        _ => String::new(),
    };
    (number, word)
}

/// The row's text on one line (a box's line breaks become spaces).
fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The row's accessible name and tooltip: what it shows, uncut.
fn row_label(line: &OutlineLine, settings: &Settings) -> String {
    let (number, word) = lead(line, settings);
    let label = [number, word, one_line(&line.text)]
        .into_iter()
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if label.is_empty() {
        t("outline.empty")
    } else {
        label
    }
}

/// The colour of the bar at a row's left edge, if it has one.
fn bar_colour(line: &OutlineLine, tk: &Tokens) -> Option<Color32> {
    match line.kind {
        LineKind::Claim => Some(tk.claim_border),
        LineKind::Premise { role, strength, .. } => Some(card_colour(tk, role, strength)),
        LineKind::Category | LineKind::Source(_) | LineKind::Note => None,
    }
}

fn row(
    ui: &mut Ui,
    line: &OutlineLine,
    mark: Option<Mark>,
    selected: bool,
    settings: &Settings,
    tk: &Tokens,
) -> Response {
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(width, ROW_H), Sense::click());
    let label = row_label(line, settings);
    resp.widget_info(|| WidgetInfo::selected(Role::ListItem, true, selected, &label));
    if !ui.is_rect_visible(rect) {
        return resp;
    }
    let painter = ui.painter();
    if selected {
        painter.rect_filled(rect, 3.0, selected_fill(tk, tk.panel));
    } else if resp.hovered() {
        painter.rect_filled(rect, 3.0, hover_fill(tk, tk.panel));
    }

    let indent = if settings.outline_indent {
        (line.depth as f32 * INDENT).min((width - MIN_TEXT_W).max(0.0))
    } else {
        0.0
    };
    let mut x = rect.left() + 4.0 + indent;
    let cy = rect.center().y;
    if settings.outline_colour
        && let Some(colour) = bar_colour(line, tk)
    {
        let bar = Rect::from_min_max(
            pos2(x, rect.top() + 3.0),
            pos2(x + BAR_W, rect.bottom() - 3.0),
        );
        painter.rect_filled(bar, 1.0, colour);
    }
    x += BAR_W + GAP;

    let right = rect.right() - 4.0 - if mark.is_some() { MARK + GAP } else { 0.0 };
    let font = FontId::proportional(FONT);
    let (number, word) = lead(line, settings);
    if !number.is_empty() {
        x = word_at(ui, x, cy, &number, &font, tk.text_muted) + GAP;
    }
    if let LineKind::Source(source_type) = &line.kind {
        let icon = Rect::from_center_size(pos2(x + ICON / 2.0, cy), vec2(ICON, ICON));
        paint_source_icon(painter, icon, source_type, tk.text_muted);
        x += ICON + 4.0;
    }
    if !word.is_empty() {
        x = word_at(ui, x, cy, &word, &font, tk.text_muted) + GAP;
    }

    let text = one_line(&line.text);
    let (text, colour) = match line.kind {
        _ if text.is_empty() => (t("outline.empty"), tk.text_muted),
        LineKind::Note => (text, tk.text_muted),
        _ => (text, tk.box_text),
    };
    let mut job = LayoutJob::simple_singleline(text, font, colour);
    job.wrap = egui::text::TextWrapping::truncate_at_width((right - x).max(0.0));
    let galley = painter.layout_job(job);
    let elided = galley.elided;
    painter.galley(pos2(x, cy - galley.size().y / 2.0), galley, colour);

    if let Some(mark) = mark {
        let c = pos2(rect.right() - 4.0 - MARK / 2.0, cy);
        paint_mark(painter, c, MARK / 2.0, mark, tk);
    }
    if elided {
        resp.on_hover_text(label)
    } else {
        resp
    }
}

/// Paint `text` left-aligned at `x`, centred on `cy`; returns its right edge.
fn word_at(ui: &Ui, x: f32, cy: f32, text: &str, font: &FontId, colour: Color32) -> f32 {
    let r = ui
        .painter()
        .text(pos2(x, cy), Align2::LEFT_CENTER, text, font.clone(), colour);
    r.right()
}
