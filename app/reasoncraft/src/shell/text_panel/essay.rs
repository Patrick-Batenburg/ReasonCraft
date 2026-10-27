//! The Essay tab: the introduction, one section per branch under the claim
//! (strongest first), and the conclusion, each map statement with a
//! paragraph field under it. Below them: paragraphs of items that are not
//! in the essay right now, and unplaced notes of deleted items.
//!
//! The order is cached per document revision (see [`DocTab::essay`]).

use eframe::egui::{
    self, CollapsingHeader, Frame, Margin, Response, RichText, Role, Sense, Stroke, Ui, WidgetInfo,
    pos2, text::LayoutJob, vec2,
};
use reasoncraft_core::essay::{ElementRole, EssayElement, EssaySection, EssaySlot};
use reasoncraft_core::{BoxKind, Document, Item, Role as CardRole};

use super::essay_rows::{Part, parts};
use super::essay_slot::{self, Paragraph, Statement, paragraph_id};
use super::virtual_rows::{self, Rows};
use crate::actions::Action;
use crate::canvas::rating::{Mark, card_colour, mark_for, rating_dots, rating_label};
use crate::doc_tab::{DocTab, EssayRow, EssayView};
use crate::i18n::{FluentArgs, t, t_args};
use crate::settings::Settings;
use crate::spell::SpellFields;
use crate::theme::Tokens;

const HEADING_FONT: f32 = 14.0;
const DOT_R: f32 = 3.0;
const DOT_STEP: f32 = 8.0;

/// What one frame of the tab needs to draw statements.
struct Ctx<'a> {
    tab: &'a DocTab,
    doc: &'a Document,
    view: &'a EssayView,
    selection: Option<Item>,
    settings: &'a Settings,
    tk: &'a Tokens,
}

/// The height assumed for a row not laid out yet: a statement with its
/// paragraph field.
const ROW_ESTIMATE: f32 = 70.0;

/// Draw the Essay tab for `tab` and queue the user's commands in `out`.
/// Only the rows near the view are laid out.
pub fn show(
    ui: &mut Ui,
    tab: &mut DocTab,
    settings: &Settings,
    tk: &Tokens,
    spell: &mut SpellFields,
    out: &mut Vec<Action>,
) {
    tab.essay();
    let mut heights = std::mem::take(&mut tab.essay_heights);
    let tab_ref = &*tab;
    if let Some((_, view)) = &tab_ref.essay_cache {
        let cx = Ctx {
            tab: tab_ref,
            doc: &tab_ref.doc,
            view,
            selection: tab_ref.selection,
            settings,
            tk,
        };
        let parts = parts(&tab_ref.doc, view);
        let keys: Vec<EssayRow> = parts.iter().map(Part::key).collect();
        // The row whose field has the keyboard focus stays laid out.
        let focused = ui.memory(|m| m.focused());
        let keep = focused.and_then(|id| {
            keys.iter().find(|k| match k {
                EssayRow::Slot(slot) => paragraph_id(tab_ref, *slot) == id,
                _ => false,
            })
        });
        let rows = Rows {
            keys: &keys,
            estimate: ROW_ESTIMATE,
            revision: tab_ref.revision(),
            keep,
            draw: |ui: &mut Ui, i: usize| part(ui, &cx, &parts[i], spell, out),
        };
        virtual_rows::show(ui, &mut heights, rows);
    }
    tab.essay_heights = heights;
}

/// Draws one row of the tab.
fn part(ui: &mut Ui, cx: &Ctx, part: &Part, spell: &mut SpellFields, out: &mut Vec<Action>) {
    match *part {
        Part::Help => {
            help(ui, cx.tk);
            ui.add_space(6.0);
        }
        Part::End(slot) => {
            if slot == EssaySlot::Conclusion {
                ui.add_space(10.0);
            }
            end_slot(ui, cx, slot, spell, out);
        }
        Part::NoSections => {
            ui.label(RichText::new(t("essay.no_sections")).color(cx.tk.text_muted));
        }
        Part::Section(n, section) => {
            ui.add_space(10.0);
            section_heading(ui, cx, n, section);
        }
        Part::Element(e) => element(ui, cx, e, spell, out),
        Part::InactiveHeading => list_heading(ui, cx.tk, "essay.inactive"),
        Part::Inactive(item, label, text) => inactive(ui, cx, item, label, text, out),
        Part::UnplacedHeading => list_heading(ui, cx.tk, "essay.unplaced"),
        Part::Unplaced(i, label, text) => unplaced(ui, cx, i, label, text, out),
    }
}

/// The "How to use" note: the structure the essay follows.
fn help(ui: &mut Ui, tk: &Tokens) {
    CollapsingHeader::new(t("essay.help.title"))
        .id_salt("essay_help")
        .default_open(true)
        .show(ui, |ui| {
            ui.label(RichText::new(t("essay.help")).color(tk.text_muted));
        });
}

/// A heading: bold `text`, wrapped, with `dots` strength dots before it in
/// `colour`. Read by screen readers as a heading named `text`.
fn heading(ui: &mut Ui, text: &str, dots: usize, colour: egui::Color32, tk: &Tokens) -> Response {
    let width = ui.available_width();
    let lead = if dots > 0 {
        dots as f32 * DOT_STEP + 4.0
    } else {
        0.0
    };
    let font = egui::FontId::proportional(HEADING_FONT);
    let job = LayoutJob::simple(text.to_owned(), font, tk.box_text, (width - lead).max(1.0));
    let galley = ui.painter().layout_job(job);
    let (rect, resp) = ui.allocate_exact_size(vec2(width, galley.size().y), Sense::hover());
    resp.widget_info(|| WidgetInfo::labeled(Role::Heading, true, text));
    let painter = ui.painter();
    let cy = rect.top() + galley.rows.first().map_or(0.0, |r| r.height()) / 2.0;
    for i in 0..dots {
        let x = rect.left() + DOT_R + 1.0 + i as f32 * DOT_STEP;
        painter.circle_filled(pos2(x, cy), DOT_R, colour);
    }
    painter.galley(pos2(rect.left() + lead, rect.top()), galley, tk.box_text);
    resp
}

/// The introduction or conclusion: heading, the claim, the paragraph.
fn end_slot(
    ui: &mut Ui,
    cx: &Ctx,
    slot: EssaySlot,
    spell: &mut SpellFields,
    out: &mut Vec<Action>,
) {
    let key = if slot == EssaySlot::Intro {
        "essay.intro"
    } else {
        "essay.conclusion"
    };
    heading(ui, &t(key), 0, cx.tk.box_text, cx.tk);
    ui.add_space(2.0);
    let root = cx.doc.root;
    if let Some(claim) = cx.doc.boxes.get(&root) {
        let s = Statement {
            item: Item::Box(root),
            text: &claim.text,
            depth: 0,
            bar: Some(cx.tk.claim_border),
            source: None,
            mark: mark(cx, Item::Box(root)),
            selected: cx.selection == Some(Item::Box(root)),
        };
        essay_slot::statement(ui, &s, cx.tk, out);
    }
    let label = t(&format!("{key}.field"));
    let hint = t(&format!("{key}.hint"));
    let field = Paragraph {
        slot,
        depth: 0,
        label: &label,
        hint: &hint,
    };
    essay_slot::paragraph(ui, cx.tab, field, spell, out);
}

/// The marker for `item`, if markers are shown and it is rated.
fn mark(cx: &Ctx, item: Item) -> Option<Mark> {
    match item {
        Item::Box(b) if cx.settings.show_markers => cx.doc.boxes.get(&b).and_then(mark_for),
        _ => None,
    }
}

/// A section's heading: strength dots, "Section n" and its premises.
fn section_heading(ui: &mut Ui, cx: &Ctx, n: usize, section: &EssaySection) {
    let (role, strength) = match section.item {
        Item::Card(c) => (
            cx.doc.card_role(c),
            cx.doc.cards.get(&c).and_then(|c| c.strength),
        ),
        Item::Box(_) => (CardRole::Supports, None),
    };
    let rating = rating_label(role, strength);
    let dots = rating_dots(strength);
    let own: Vec<String> = section
        .elements
        .iter()
        .filter(|e| e.depth == 0)
        .map(|e| e.text.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|t| !t.is_empty())
        .collect();
    let mut args = FluentArgs::new();
    args.set("n", n);
    let text = if own.is_empty() {
        t_args("essay.section", &args)
    } else {
        args.set("premises", own.join("; "));
        t_args("essay.section_with", &args)
    };
    let colour = card_colour(cx.tk, role, strength);
    heading(ui, &text, dots, colour, cx.tk).on_hover_text(rating);
    ui.add_space(2.0);
}

/// One element of a section: its statement and its paragraph.
fn element(
    ui: &mut Ui,
    cx: &Ctx,
    e: &EssayElement,
    spell: &mut SpellFields,
    out: &mut Vec<Action>,
) {
    let EssaySlot::Item(item) = e.slot else {
        return;
    };
    let node = match item {
        Item::Box(b) => cx.doc.boxes.get(&b),
        Item::Card(_) => None,
    };
    let source = node.and_then(|n| match &n.kind {
        BoxKind::Source { source_type } => Some(source_type),
        _ => None,
    });
    let bar = match e.role {
        ElementRole::Evidence => None,
        ElementRole::Claim => Some(cx.tk.claim_border),
        ElementRole::Reason | ElementRole::Counter | ElementRole::Rebuttal => {
            let role = match e.role {
                ElementRole::Reason => CardRole::Supports,
                ElementRole::Counter => CardRole::Opposes,
                _ => CardRole::Rebuts,
            };
            let strength = match item {
                Item::Box(b) => cx.view.premise_strength.get(&b).copied().flatten(),
                Item::Card(_) => None,
            };
            Some(card_colour(cx.tk, role, strength))
        }
    };
    let s = Statement {
        item,
        text: &e.text,
        depth: e.depth,
        bar,
        source,
        mark: mark(cx, item),
        selected: cx.selection == Some(item),
    };
    ui.add_space(4.0);
    essay_slot::statement(ui, &s, cx.tk, out);
    let mut args = FluentArgs::new();
    args.set("statement", one_line(&e.text));
    let label = t_args("essay.paragraph_for", &args);
    let hint = t("essay.hint");
    let field = Paragraph {
        slot: e.slot,
        depth: e.depth,
        label: &label,
        hint: &hint,
    };
    essay_slot::paragraph(ui, cx.tab, field, spell, out);
}

fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A note in a frame, like a snippet card.
fn note_frame(ui: &mut Ui, tk: &Tokens, body: impl FnOnce(&mut Ui)) {
    Frame::new()
        .fill(tk.box_fill)
        .stroke(Stroke::new(1.0, tk.attachment_line.gamma_multiply(0.5)))
        .corner_radius(6.0)
        .inner_margin(Margin::same(6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            body(ui);
        });
    ui.add_space(6.0);
}

/// A list heading with an explanation under it.
fn list_heading(ui: &mut Ui, tk: &Tokens, key: &str) {
    ui.add_space(14.0);
    heading(ui, &t(key), 0, tk.box_text, tk);
    ui.label(RichText::new(t(&format!("{key}.hint"))).color(tk.text_muted));
    ui.add_space(4.0);
}

/// The paragraph of a parked item, or of one no longer in an argument:
/// read-only, with Copy. It counts again when the item returns.
fn inactive(ui: &mut Ui, cx: &Ctx, item: Item, label: &str, text: &str, out: &mut Vec<Action>) {
    note_frame(ui, cx.tk, |ui| {
        let s = Statement {
            item,
            text: label,
            depth: 0,
            bar: None,
            source: None,
            mark: None,
            selected: cx.selection == Some(item),
        };
        essay_slot::statement(ui, &s, cx.tk, out);
        ui.label(text);
        if ui.button(t("essay.copy")).clicked() {
            ui.ctx().copy_text(text.to_owned());
        }
    });
}

/// The `i`th note whose item was deleted, with Copy and Remove.
fn unplaced(ui: &mut Ui, cx: &Ctx, i: usize, label: &str, text: &str, out: &mut Vec<Action>) {
    note_frame(ui, cx.tk, |ui| {
        ui.label(RichText::new(label).strong().color(cx.tk.text_muted));
        ui.label(text);
        ui.horizontal(|ui| {
            if ui.button(t("essay.copy")).clicked() {
                ui.ctx().copy_text(text.to_owned());
            }
            if ui.button(t("essay.remove")).clicked() {
                out.push(Action::RemoveUnplaced(i));
            }
        });
    });
}
