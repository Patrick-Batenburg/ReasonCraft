//! The Check tab: hints from the course-rule checks, grouped by check.
//! Issues are cached per document revision (see [`DocTab::issues`]);
//! clicking one selects its item and shows it on the map.

use eframe::egui::{
    self, FontId, Frame, Label, Margin, Response, RichText, Role, Sense, Shape, Ui, WidgetInfo,
};
use reasoncraft_core::checks::{CheckKind, Issue, compound_conjunctions};
use reasoncraft_core::{CardKind, CardStrength, Document, Item};

use super::outline::one_line;
use super::virtual_rows::{self, Rows};
use crate::a11y::labelled;
use crate::actions::Action;
use crate::doc_tab::{CheckRow, DocTab};
use crate::help::HelpPage;
use crate::i18n::{FluentArgs, t, t_args};
use crate::settings::Settings;
use crate::shell::{hover_fill, selected_fill};
use crate::theme::Tokens;

const TITLE: f32 = 13.0;
const EXPLAIN: f32 = 12.0;

/// The check's name in i18n keys (`check.kind.<key>`) and files.
pub fn check_key(kind: CheckKind) -> &'static str {
    kind.key()
}

/// The rule of check `kind`, for a document in `doc_lang`: the Compound
/// help names the conjunctions the check looks for in that language.
pub fn check_help(kind: CheckKind, doc_lang: &str) -> String {
    let key = format!("check.help.{}", check_key(kind));
    if kind != CheckKind::Compound {
        return t(&key);
    }
    let [and, or] = compound_conjunctions(doc_lang);
    let mut args = FluentArgs::new();
    args.set("and", and);
    args.set("or", or);
    t_args(&key, &args)
}

/// One row of the tab, with what it draws.
enum Part<'a> {
    /// A check's heading with its count; the first heading has no space
    /// above it.
    Heading(CheckKind, usize, bool),
    /// The `n`th issue of its check.
    Issue(usize, &'a Issue),
    /// "Show ignored hints", with the number ignored.
    Footer(usize),
}

impl Part<'_> {
    fn key(&self) -> CheckRow {
        match *self {
            Part::Heading(kind, ..) => CheckRow::Heading(kind),
            Part::Issue(n, issue) => CheckRow::Issue(issue.kind, n),
            Part::Footer(_) => CheckRow::Footer,
        }
    }
}

/// The rows of the tab: the issues grouped by check, then the footer.
fn parts(issues: &[Issue], ignored: usize) -> Vec<Part<'_>> {
    let mut parts = Vec::with_capacity(issues.len() + CheckKind::ALL.len() + 1);
    for kind in CheckKind::ALL {
        let group: Vec<&Issue> = issues.iter().filter(|i| i.kind == kind).collect();
        if group.is_empty() {
            continue;
        }
        // "Not rated yet" is one row standing for many items.
        let count = match group.as_slice() {
            [one] if kind == CheckKind::IncompleteEvaluation => {
                one.terms.first().and_then(|n| n.parse().ok()).unwrap_or(1)
            }
            _ => group.len(),
        };
        parts.push(Part::Heading(kind, count, parts.is_empty()));
        parts.extend(
            group
                .into_iter()
                .enumerate()
                .map(|(n, i)| Part::Issue(n, i)),
        );
    }
    if ignored > 0 {
        parts.push(Part::Footer(ignored));
    }
    parts
}

/// The height assumed for an issue row not laid out yet.
const ROW_ESTIMATE: f32 = 44.0;

/// Draw the Check tab for `tab`, with the checks turned on in `settings`.
/// Only the rows near the view are laid out.
pub fn show(
    ui: &mut Ui,
    tab: &mut DocTab,
    settings: &Settings,
    tk: &Tokens,
    out: &mut Vec<Action>,
) {
    tab.issues(&settings.checks);
    let mut heights = std::mem::take(&mut tab.check_heights);
    let tab_ref = &*tab;
    let issues = tab_ref.issue_cache.as_ref().map_or(&[][..], |(_, _, i)| i);
    let ignored = tab_ref.doc.ignored_hints.len();
    if issues.is_empty() {
        ui.label(RichText::new(t("check.all_clear")).color(tk.text_muted));
        footer(ui, ignored, out);
        ui.take_available_space();
    } else {
        let parts = parts(issues, ignored);
        let keys: Vec<CheckRow> = parts.iter().map(Part::key).collect();
        let draw = |ui: &mut Ui, i: usize| match parts[i] {
            Part::Heading(kind, count, first) => {
                if !first {
                    ui.add_space(10.0);
                }
                heading(ui, kind, count, &tab_ref.doc.meta.language, tk, out);
            }
            Part::Issue(n, issue) => {
                let selected = tab_ref.selection == Some(issue.item);
                let id = ui.scope_id().with((check_key(issue.kind), n));
                if row(ui, id, &tab_ref.doc, issue, selected, tk, out).clicked() {
                    out.push(Action::Select(Some(issue.item)));
                    out.push(Action::RevealSelection);
                }
            }
            Part::Footer(ignored) => {
                ui.add_space(10.0);
                footer(ui, ignored, out);
            }
        };
        let rows = Rows {
            keys: &keys,
            estimate: ROW_ESTIMATE,
            revision: tab_ref.revision(),
            draw,
        };
        virtual_rows::show(ui, &mut heights, rows);
    }
    tab.check_heights = heights;
}

/// "Show ignored hints (n)", when the document ignores any.
fn footer(ui: &mut Ui, ignored: usize, out: &mut Vec<Action>) {
    if ignored == 0 {
        return;
    }
    let mut args = FluentArgs::new();
    args.set("count", ignored);
    if ui.button(t_args("check.restore", &args)).clicked() {
        out.push(Action::RestoreHints);
    }
}

/// The group heading, "Questions (2)", with the rule (for a document in
/// `doc_lang`) as its tooltip, and a "?" that opens the checks' help topic.
fn heading(
    ui: &mut Ui,
    kind: CheckKind,
    count: usize,
    doc_lang: &str,
    tk: &Tokens,
    out: &mut Vec<Action>,
) {
    let name = t(&format!("check.kind.{}", check_key(kind)));
    let mut args = FluentArgs::new();
    args.set("kind", name.as_str());
    args.set("count", count);
    let text = RichText::new(t_args("check.heading", &args))
        .strong()
        .color(tk.box_text);
    ui.horizontal_top(|ui| {
        let help = egui::Button::new(RichText::new("?").size(EXPLAIN)).small();
        let help_w = 22.0;
        ui.scope(|ui| {
            ui.set_max_width(ui.available_width() - help_w);
            ui.add(Label::new(text).wrap().selectable(false))
                .on_hover_text(check_help(kind, doc_lang));
        });
        let mut args = FluentArgs::new();
        args.set("kind", name.as_str());
        let help_name = t_args("help.check_link", &args);
        let resp = labelled(ui.add(help), help_name.clone()).on_hover_text(help_name);
        if resp.clicked() {
            out.push(Action::OpenHelp(HelpPage::Topic("checks")));
        }
    });
    ui.add_space(2.0);
}

/// The text that names `item`: a box's text on one line, or a card's kind
/// and premises.
pub fn item_text(doc: &Document, item: Item) -> String {
    let text = match item {
        Item::Box(b) => doc.boxes.get(&b).map(|n| one_line(&n.text)),
        Item::Card(c) => doc.cards.get(&c).map(|card| {
            let premises: Vec<String> = card
                .premises
                .iter()
                .filter_map(|p| doc.boxes.get(p))
                .map(|p| one_line(&p.text))
                .filter(|p| !p.is_empty())
                .collect();
            let kind = match card.kind {
                CardKind::Reason => t("card_kind.reason"),
                CardKind::Objection => t("card_kind.objection"),
            };
            let mut args = FluentArgs::new();
            args.set("kind", kind);
            args.set("premises", premises.join(" + "));
            t_args("check.card", &args)
        }),
    };
    text.filter(|s| !s.is_empty())
        .unwrap_or_else(|| t("outline.empty"))
}

/// The short explanation of `issue`, with its terms filled in.
pub fn explain(doc: &Document, issue: &Issue) -> String {
    let mut args = FluentArgs::new();
    args.set("terms", issue.terms.join(", "));
    if issue.kind == CheckKind::IncompleteEvaluation {
        // The one "not rated yet" issue carries the number of unrated items.
        let count: usize = issue
            .terms
            .first()
            .and_then(|n| n.parse().ok())
            .unwrap_or(1);
        args.set("count", count);
    }
    if issue.kind == CheckKind::InconsistentEvaluation {
        // The issue carries no terms: which rule it broke follows from the item.
        let case = match issue.item {
            Item::Box(_) => "solid",
            Item::Card(c) => match doc.cards.get(&c).and_then(|c| c.strength) {
                Some(CardStrength::Strong) => "strong",
                _ => "nil",
            },
        };
        args.set("case", case);
    }
    t_args(&format!("check.explain.{}", check_key(issue.kind)), &args)
}

/// One issue: the item's text on one line (the full text in a tooltip when
/// cut) and the explanation, wrapped. The whole row is one click target,
/// except its "Ignore" button.
fn row(
    ui: &mut Ui,
    id: egui::Id,
    doc: &Document,
    issue: &Issue,
    selected: bool,
    tk: &Tokens,
    out: &mut Vec<Action>,
) -> Response {
    let title = item_text(doc, issue.item);
    let ignore = t("check.ignore");
    let button_w = ui
        .painter()
        .layout_no_wrap(ignore.clone(), FontId::proportional(EXPLAIN), tk.box_text)
        .size()
        .x
        + 2.0 * ui.spacing().button_padding.x
        + 4.0;
    let background = ui.painter().add(Shape::Noop);
    let inner = Frame::new()
        .inner_margin(Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 2.0;
            let title = RichText::new(&title).size(TITLE).color(tk.box_text);
            let explain = RichText::new(explain(doc, issue))
                .size(EXPLAIN)
                .color(tk.text_muted);
            // The title leaves room for the Ignore button drawn over its end.
            let text_w = ui.available_width() - button_w;
            ui.scope(|ui| {
                ui.set_max_width(text_w);
                ui.add(Label::new(title).truncate().selectable(false));
            });
            ui.add(Label::new(explain).wrap().selectable(false));
            text_w
        });
    let rect = inner.response.rect;
    let resp = ui.interact(rect, id, Sense::click());
    // Added after the row, so it sits on top and takes its own clicks.
    let button_rect = egui::Rect::from_min_size(
        egui::pos2(rect.right() - button_w - 4.0, rect.top() + 2.0),
        egui::vec2(button_w, TITLE + 4.0),
    );
    let button = egui::Button::new(RichText::new(ignore).size(EXPLAIN)).small();
    let mut args = FluentArgs::new();
    args.set("item", title.as_str());
    args.set("kind", t(&format!("check.kind.{}", check_key(issue.kind))));
    let name = t_args("check.ignore_name", &args);
    if labelled(ui.place(button_rect, button), name).clicked() {
        out.push(Action::IgnoreHint(issue.clone()));
    }
    resp.widget_info(|| WidgetInfo::selected(Role::ListItem, true, selected, &title));
    let fill = if selected {
        Some(selected_fill(tk, tk.panel))
    } else if resp.hovered() {
        Some(hover_fill(tk, tk.panel))
    } else {
        None
    };
    if let Some(fill) = fill {
        ui.painter()
            .set(background, Shape::rect_filled(rect, 3.0, fill));
    }
    let text_w = inner.inner;
    let full_w = ui
        .painter()
        .layout_no_wrap(title.clone(), FontId::proportional(TITLE), tk.box_text)
        .size()
        .x;
    let resp = if full_w > text_w {
        resp.on_hover_text(title)
    } else {
        resp
    };
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

#[cfg(test)]
mod tests;
