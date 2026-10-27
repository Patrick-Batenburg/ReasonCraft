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
    kind.key()
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
    let ignored = tab.doc.ignored_hints.len();
    if issues.is_empty() {
        ui.label(RichText::new(t("check.all_clear")).color(tk.text_muted));
        footer(ui, ignored, out);
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
                // "Not rated yet" is one row standing for many items.
                let count = match group.as_slice() {
                    [one] if kind == CheckKind::IncompleteEvaluation => {
                        one.terms.first().and_then(|n| n.parse().ok()).unwrap_or(1)
                    }
                    _ => group.len(),
                };
                heading(ui, kind, count, tk);
                for (n, issue) in group.into_iter().enumerate() {
                    let selected = tab.selection == Some(issue.item);
                    let id = ui.scope_id().with((check_key(kind), n));
                    if row(ui, id, &tab.doc, issue, selected, tk, out).clicked() {
                        out.push(Action::Select(Some(issue.item)));
                        out.push(Action::RevealSelection);
                    }
                }
                ui.add_space(10.0);
            }
            footer(ui, ignored, out);
        });
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
    if ui.put(button_rect, button).clicked() {
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
    fn incomplete_evaluation_explains_the_count() {
        let mut d = Document::new("Robots help");
        d.add_card(Parent::Box(d.root), CardKind::Reason, "Robots help")
            .unwrap();
        d.set_acceptance(d.root, Some(Acceptance::Accept)).unwrap();
        let text = explanation(&d, CheckKind::IncompleteEvaluation);
        assert!(text.starts_with("2 items are not rated yet"), "{text}");
        assert!(text.contains("next one"), "{text}");
    }

    #[test]
    fn incomplete_plural_one() {
        let mut args = FluentArgs::new();
        args.set("count", 1);
        let text = t_args("check.explain.incomplete_evaluation", &args);
        assert!(text.starts_with("1 item is not rated yet,"), "{text}");
        args.set("count", 2);
        let text = t_args("check.explain.incomplete_evaluation", &args);
        assert!(text.starts_with("2 items are not rated yet,"), "{text}");
    }

    #[test]
    fn compound_explanation_asks_and_quotes_the_word() {
        let mut d = Document::new("Robots help or hurt");
        d.meta.language = "en".into();
        let text = explanation(&d, CheckKind::Compound);
        assert!(text.contains("(joined by “or”)"), "{text}");
        assert!(
            text.contains("accept one part and reject the other"),
            "{text}"
        );
        // The group help names no particular conjunction.
        let help = t("check.help.compound");
        assert!(
            !help.contains("\"and\"") && !help.contains("\"or\""),
            "{help}"
        );
    }

    #[test]
    fn key_term_explanation_lists_the_missing_words() {
        let mut d = Document::new("Robots replace teachers");
        d.add_card(Parent::Box(d.root), CardKind::Reason, "Robots are cheap")
            .unwrap();
        let text = explanation(&d, CheckKind::KeyTerm);
        assert_eq!(text, "Missing: replace, teachers");
        // The key-word rule is explained once, in the group help.
        assert!(t("check.help.key_term").contains("key-word rule"));
        let issues = run_checks(&d, &CheckSet::default());
        let card = issues
            .iter()
            .find(|i| i.kind == CheckKind::KeyTerm)
            .unwrap();
        assert_eq!(item_text(&d, card.item), "Reason: Robots are cheap");
    }
}
