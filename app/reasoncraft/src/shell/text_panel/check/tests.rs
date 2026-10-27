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
}

#[test]
fn compound_help_in_document_language() {
    // The help names the words the check looks for in the document's
    // language, whatever language the interface is in.
    let en = check_help(CheckKind::Compound, "en");
    assert!(en.contains("“and”") && en.contains("“or”"), "{en}");
    let nl = check_help(CheckKind::Compound, "nl-NL");
    assert!(nl.contains("“en”") && nl.contains("“of”"), "{nl}");
    assert!(!nl.contains("“and”"), "{nl}");
    // Other checks' help does not depend on the document.
    assert_eq!(
        check_help(CheckKind::KeyTerm, "nl"),
        t("check.help.key_term")
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
