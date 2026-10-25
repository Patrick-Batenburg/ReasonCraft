//! The Check tab: hints from the course-rule checks, grouped by check.
//! Issues are cached per document revision (see [`DocTab::issues`]);
//! clicking one selects its item and shows it on the map.

use eframe::egui::{
    self, FontId, Frame, Label, Margin, Response, RichText, Role, ScrollArea, Sense, Shape, Ui,
    WidgetInfo,
};
use reasoncraft_core::checks::{CheckKind, Issue};
use reasoncraft_core::{CardKind, CardStrength, Document, Item};

use super::outline::one_line;
use crate::actions::Action;
use crate::doc_tab::DocTab;
use crate::i18n::{FluentArgs, t, t_args};
use crate::settings::Settings;
use crate::shell::{hover_fill, selected_fill};
use crate::theme::Tokens;

const TITLE: f32 = 13.0;
const EXPLAIN: f32 = 12.0;

/// The check's name in i18n keys (`check.kind.<key>`) and files.
pub fn check_key(kind: CheckKind) -> &'static str {
    match kind {
        CheckKind::KeyTerm => "key_term",
        CheckKind::Compound => "compound",
        CheckKind::Question => "question",
        CheckKind::Unsupported => "unsupported",
        CheckKind::EmptyCategory => "empty_category",
        CheckKind::IncompleteEvaluation => "incomplete_evaluation",
        CheckKind::InconsistentEvaluation => "inconsistent_evaluation",
        CheckKind::NotConnected => "not_connected",
    }
}

/// Draw the Check tab for `tab`, with the checks turned on in `settings`.
pub fn show(
    ui: &mut Ui,
    tab: &mut DocTab,
    settings: &Settings,
    tk: &Tokens,
    out: &mut Vec<Action>,
) {
    tab.issues(&settings.checks);
    let tab = &*tab;
    let issues = tab.issue_cache.as_ref().map_or(&[][..], |(_, _, i)| i);
    if issues.is_empty() {
        ui.label(RichText::new(t("check.all_clear")).color(tk.text_muted));
        ui.take_available_space();
        return;
    }
    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for kind in CheckKind::ALL {
                let group: Vec<&Issue> = issues.iter().filter(|i| i.kind == kind).collect();
                if group.is_empty() {
                    continue;
                }
                heading(ui, kind, group.len(), tk);
                for (n, issue) in group.into_iter().enumerate() {
                    let selected = tab.selection == Some(issue.item);
                    let id = ui.scope_id().with((check_key(kind), n));
                    if row(ui, id, &tab.doc, issue, selected, tk).clicked() {
                        out.push(Action::Select(Some(issue.item)));
                        out.push(Action::RevealSelection);
                    }
                }
                ui.add_space(10.0);
            }
        });
}

/// The group heading, "Questions (2)", with the rule as its tooltip.
fn heading(ui: &mut Ui, kind: CheckKind, count: usize, tk: &Tokens) {
    let key = check_key(kind);
    let mut args = FluentArgs::new();
    args.set("kind", t(&format!("check.kind.{key}")));
    args.set("count", count);
    let text = RichText::new(t_args("check.heading", &args))
        .strong()
        .color(tk.box_text);
    ui.add(Label::new(text).wrap().selectable(false))
        .on_hover_text(t(&format!("check.help.{key}")));
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
/// cut) and the explanation, wrapped. The whole row is one click target.
fn row(
    ui: &mut Ui,
    id: egui::Id,
    doc: &Document,
    issue: &Issue,
    selected: bool,
    tk: &Tokens,
) -> Response {
    let title = item_text(doc, issue.item);
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
            ui.add(Label::new(title).truncate().selectable(false));
            ui.add(Label::new(explain).wrap().selectable(false));
            ui.available_width()
        });
    let rect = inner.response.rect;
    let resp = ui.interact(rect, id, Sense::click());
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
mod tests {
    use super::*;
    use reasoncraft_core::checks::{CheckSet, run_checks};
    use reasoncraft_core::{Acceptance, BoxKind, Parent, SourceRating, SourceType};

    fn explanation(doc: &Document, kind: CheckKind) -> String {
        let issues = run_checks(doc, &CheckSet::default());
        let issue = issues.iter().find(|i| i.kind == kind).expect("issue");
        explain(doc, issue)
    }

    #[test]
    fn check_keys_match_the_saved_names() {
        for kind in CheckKind::ALL {
            let saved = serde_json::to_string(&kind).unwrap();
            assert_eq!(saved, format!("\"{}\"", check_key(kind)));
        }
    }

    #[test]
    fn inconsistent_evaluation_explains_each_case() {
        let mut d = Document::new("Robots help");
        let (card, premise) = d
            .add_card(Parent::Box(d.root), CardKind::Reason, "Robots help")
            .unwrap();
        let source = BoxKind::Source {
            source_type: SourceType::Web,
        };
        let src = d.add_box(Parent::Box(premise), source, "x").unwrap();
        d.set_acceptance(premise, Some(Acceptance::Reject)).unwrap();
        d.set_card_strength(card, Some(CardStrength::Strong))
            .unwrap();
        d.set_source_rating(src, Some(SourceRating::Solid)).unwrap();
        let issues = run_checks(&d, &CheckSet::default());
        let texts: Vec<String> = issues
            .iter()
            .filter(|i| i.kind == CheckKind::InconsistentEvaluation)
            .map(|i| explain(&d, i))
            .collect();
        assert_eq!(texts.len(), 2, "{issues:?}");
        assert!(texts.iter().any(|s| s.contains("strong")), "{texts:?}");
        assert!(texts.iter().any(|s| s.contains("solid")), "{texts:?}");

        d.set_acceptance(premise, Some(Acceptance::Accept)).unwrap();
        d.set_card_strength(card, Some(CardStrength::Nil)).unwrap();
        let text = explanation(&d, CheckKind::InconsistentEvaluation);
        assert!(text.contains("nil"), "{text}");
    }

    #[test]
    fn key_term_explanation_lists_the_missing_words() {
        let mut d = Document::new("Robots replace teachers");
        d.add_card(Parent::Box(d.root), CardKind::Reason, "Robots are cheap")
            .unwrap();
        let text = explanation(&d, CheckKind::KeyTerm);
        assert!(text.contains("replace, teachers"), "{text}");
        let issues = run_checks(&d, &CheckSet::default());
        let card = issues
            .iter()
            .find(|i| i.kind == CheckKind::KeyTerm)
            .unwrap();
        assert_eq!(item_text(&d, card.item), "Reason: Robots are cheap");
    }
}
