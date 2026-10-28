//! The Outline tab: the map as numbered, indented text lines with a toolbar
//! of switches and "Copy all". Rows are virtualised and the outline is
//! cached per document revision (see [`DocTab::outline`]).

use eframe::egui::{
    self, Color32, FontId, Id, Painter, Rect, Response, Role, ScrollArea, Sense, Ui, WidgetInfo,
    pos2, text::LayoutJob, vec2,
};
use reasoncraft_core::outline::{LineKind, LinkLabel, OutlineLine};
use reasoncraft_core::outline_md::{outline_plain_text_lang, source_name};
use reasoncraft_core::words::linking_words;
use reasoncraft_core::{Item, SourceType};

use crate::actions::Action;
use crate::canvas::scene_paint::paint_mark;
use crate::doc_tab::DocTab;
use crate::i18n::{FluentArgs, t, t_lang, t_lang_args, ui_language};
use crate::scene::rating::{Mark, card_colour, mark_for};
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

/// The linking word shown for `label` in the document language `lang`
/// ("" for none): the outline is document text, like its exports.
fn label_word(lang: &str, label: LinkLabel) -> String {
    linking_words(lang).word(label).to_owned()
}

/// The tooltip of the switch named `key` in the UI language `lang`; the
/// linking words in the Labels tip come from the core language table.
fn toggle_tip(lang: &str, key: &str) -> String {
    let key = format!("outline.tip.{key}");
    let w = linking_words(lang);
    let mut args = FluentArgs::new();
    args.set("words", format!("{}, {}, {}", w.because, w.but, w.however));
    t_lang_args(lang, &key, &args)
}

/// The name of a source type in the document language `lang`, as on the
/// canvas.
fn source_word(lang: &str, source_type: &SourceType) -> String {
    source_name(source_type, &|key| t_lang(lang, key))
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
    let lines = tab.outline_cache.last().map_or(&[][..], Vec::as_slice);
    let (doc, selection) = (&tab.doc, tab.selection);
    let lang = doc.meta.language.as_str();
    toolbar(ui, lines, lang, settings, tk, out);
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
                    if row(ui, line, lang, mark, selected, settings, tk).clicked() {
                        out.push(Action::Select(Some(line.item)));
                        out.push(Action::RevealSelection);
                    }
                }
            });
    });
}

/// The switches and "Copy all", which copies the outline as the Outline
/// (text) export writes it, in the document language `lang`.
fn toolbar(
    ui: &mut Ui,
    lines: &[OutlineLine],
    lang: &str,
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
                .on_hover_text(toggle_tip(ui_language(), key));
            if resp.clicked() {
                out.push(Action::SetOutlineOption(toggle, !on));
            }
        }
        // `Id::unique` hashes with fixed seeds: the same id every frame.
        let copied_id = Id::unique("outline_copied_at");
        if ui
            .button(t("outline.copy_all"))
            .on_hover_text(t("outline.tip.copy_all"))
            .clicked()
        {
            let label = |key: &str| t_lang(lang, key);
            let text = outline_plain_text_lang(lines, &settings.outline, lang, &label);
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

/// The words of a row before its text: number, linking word, source type
/// (in the document language `lang`).
fn lead(line: &OutlineLine, lang: &str, settings: &Settings) -> (String, String) {
    let opts = settings.outline;
    let number = if opts.numbering {
        line.number.clone()
    } else {
        String::new()
    };
    let word = match &line.kind {
        LineKind::Source(source_type) => source_word(lang, source_type),
        _ if opts.labels => label_word(lang, line.label),
        _ => String::new(),
    };
    (number, word)
}

/// The row's text on one line (a box's line breaks become spaces).
pub(super) fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The most characters of a box's text a row lays out: more than the
/// widest panel shows.
const SHOWN_CHARS: usize = 500;

/// The start of [`one_line`]`(text)`, at most about `max` characters, and
/// whether more follows: a row lays out only what it can show.
fn one_line_start(text: &str, max: usize) -> (String, bool) {
    let mut out = String::new();
    let mut chars = 0;
    for word in text.split_whitespace() {
        if chars >= max {
            return (out, true);
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
        chars += word.chars().count() + 1;
    }
    (out, false)
}

/// The row's accessible name and tooltip: what it shows, uncut.
fn row_label(line: &OutlineLine, lang: &str, settings: &Settings) -> String {
    let (number, word) = lead(line, lang, settings);
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
    lang: &str,
    mark: Option<Mark>,
    selected: bool,
    settings: &Settings,
    tk: &Tokens,
) -> Response {
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(width, ROW_H), Sense::click());
    // Made only when asked for: the box's whole text can be long.
    let label = || row_label(line, lang, settings);
    resp.widget_info(|| WidgetInfo::selected(Role::ListItem, true, selected, &label()));
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
    let (number, word) = lead(line, lang, settings);
    let mut elided = false;
    // Everything before the marker is cut at `right`, the lead words too.
    let mut put = |x: &mut f32, text: String, colour: Color32| {
        let (end, cut) = fitted(painter, *x, cy, right, text, &font, colour);
        *x = end + GAP;
        elided |= cut;
    };
    if !number.is_empty() {
        put(&mut x, number, tk.text_muted);
    }
    if let LineKind::Source(source_type) = &line.kind
        && x + ICON <= right
    {
        let icon = Rect::from_center_size(pos2(x + ICON / 2.0, cy), vec2(ICON, ICON));
        paint_source_icon(painter, icon, source_type, tk.text_muted);
        x += ICON + 4.0;
    }
    if !word.is_empty() {
        put(&mut x, word, tk.text_muted);
    }
    let (text, more) = one_line_start(&line.text, SHOWN_CHARS);
    let (text, colour) = match line.kind {
        _ if text.is_empty() => (t("outline.empty"), tk.text_muted),
        LineKind::Note => (text, tk.text_muted),
        _ => (text, tk.box_text),
    };
    put(&mut x, text, colour);
    elided |= more;

    if let Some(mark) = mark {
        let c = pos2(rect.right() - 4.0 - MARK / 2.0, cy);
        paint_mark(painter, c, MARK / 2.0, mark, tk);
    }
    if elided {
        resp.on_hover_ui(|ui| {
            ui.label(label());
        })
    } else {
        resp
    }
}

/// Paint `text` on one line from `x`, centred on `cy` and cut with "…" at
/// `right`. Returns its right edge and whether it was cut. Nothing is
/// painted (but the text counts as cut) when there is no room left.
fn fitted(
    painter: &Painter,
    x: f32,
    cy: f32,
    right: f32,
    text: String,
    font: &FontId,
    colour: Color32,
) -> (f32, bool) {
    if right - x < 1.0 {
        return (x, true);
    }
    let mut job = LayoutJob::simple_singleline(text, font.clone(), colour);
    job.wrap = egui::text::TextWrapping::truncate_at_width(right - x);
    let galley = painter.layout_job(job);
    let (end, cut) = (x + galley.size().x, galley.elided);
    painter.galley(pos2(x, cy - galley.size().y / 2.0), galley, colour);
    (end, cut)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_tip_shows_the_table_linking_words() {
        for lang in crate::i18n::UI_LANGUAGES {
            let w = linking_words(lang);
            let words = format!("{}, {}, {}", w.because, w.but, w.however);
            let tip = toggle_tip(lang, "labels");
            assert!(tip.contains(&words), "{lang}: {tip}");
            // The words come from the placeable, not from the .ftl text.
            let mut args = crate::i18n::FluentArgs::new();
            args.set("words", "W1, W2, W3");
            let tip = crate::i18n::t_lang_args(lang, "outline.tip.labels", &args);
            assert!(tip.contains("W1, W2, W3") && !tip.contains(&words), "{tip}");
        }
        assert_eq!(toggle_tip("en", "notes"), "Include notes");
    }

    #[test]
    fn rows_flatten_only_the_start_of_long_text() {
        assert_eq!(
            one_line_start(
                "a  b
c", 10
            ),
            ("a b c".to_owned(), false)
        );
        let long = "word ".repeat(10_000);
        let (start, more) = one_line_start(&long, SHOWN_CHARS);
        assert!(more);
        assert!(start.len() <= SHOWN_CHARS + 5, "{}", start.len());
        assert!(long.starts_with(&start));
        assert_eq!(one_line_start("", 10), (String::new(), false));
    }

    #[test]
    fn fitted_text_never_passes_its_right_edge() {
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(Default::default(), |ui| {
            let p = ui.painter();
            let font = FontId::proportional(FONT);
            let long = "1a.1.1.1.1.1.1.1.1.1.1.1".to_owned();
            let (end, cut) = fitted(p, 10.0, 20.0, 60.0, long, &font, Color32::WHITE);
            assert!(end <= 60.0 && cut, "{end}");
            let (end, cut) = fitted(p, 10.0, 20.0, 200.0, "1".into(), &font, Color32::WHITE);
            assert!(end < 30.0 && !cut);
            // No room at all: nothing painted, counted as cut.
            assert_eq!(
                fitted(p, 60.0, 20.0, 60.0, "x".into(), &font, Color32::WHITE),
                (60.0, true)
            );
        });
        out.textures_delta.clear();
    }
}
