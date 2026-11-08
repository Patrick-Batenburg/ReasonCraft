//! Course-rule check tests for wording: key terms,
//! compound statements and the document language.

use reasoncraft_core::checks::{CheckKind, CheckSet, Issue, run_checks};
use reasoncraft_core::{BoxKind, CardKind, Document, Item, NewLoose, Parent};

fn all_on() -> CheckSet {
    CheckSet::default()
}

fn of_kind(issues: &[Issue], kind: CheckKind) -> Vec<&Issue> {
    issues.iter().filter(|i| i.kind == kind).collect()
}

fn doc(lang: &str, claim: &str) -> Document {
    let mut d = Document::new(claim);
    d.meta.language = lang.to_owned();
    d
}

#[test]
fn key_term_normalises_case_punctuation_diacritics() {
    let mut d = doc("nl", "Coördinatie van auto's");
    d.add_card(Parent::Box(d.root), CardKind::Reason, "coordinatie AUTO")
        .unwrap();
    let issues = run_checks(&d, &all_on());
    assert!(
        of_kind(&issues, CheckKind::KeyTerm).is_empty(),
        "{issues:?}"
    );
}

#[test]
fn key_term_dutch_example_from_course() {
    let mut d = doc("nl", "Robots zorgen voor meer banen");
    let (card, _) = d
        .add_card(
            Parent::Box(d.root),
            CardKind::Reason,
            "Robots verhogen de productiviteit",
        )
        .unwrap();
    let issues = run_checks(&d, &all_on());
    let key = of_kind(&issues, CheckKind::KeyTerm);
    assert_eq!(key.len(), 1);
    assert_eq!(key[0].item, Item::Card(card));
    assert_eq!(key[0].terms, ["zorgen", "banen"]);
}

#[test]
fn key_term_hyphens_prefixes_and_co_premises() {
    // "e-mail" against "email" (both directions), a shared stem
    // ("automatisering"/"automatiseren") and a term found in a co-premise.
    let mut d = doc("nl", "E-mail versnelt automatisering bij bedrijven");
    let (card, _) = d
        .add_card(
            Parent::Box(d.root),
            CardKind::Reason,
            "Email is sneller dan post",
        )
        .unwrap();
    d.add_premise(card, None, "Bedrijven automatiseren")
        .unwrap();
    let issues = run_checks(&d, &all_on());
    let key = of_kind(&issues, CheckKind::KeyTerm);
    assert_eq!(key.len(), 1, "{issues:?}");
    assert_eq!(key[0].terms, ["versnelt"]);

    let mut d = doc("en", "Email is fast");
    d.add_card(Parent::Box(d.root), CardKind::Reason, "E-mail is fast")
        .unwrap();
    assert!(of_kind(&run_checks(&d, &all_on()), CheckKind::KeyTerm).is_empty());
}

#[test]
fn key_term_skips_whole_card_attacks_and_non_reasoning_maps() {
    let mut d = doc("en", "Robots create jobs");
    let (card, _) = d
        .add_card(Parent::Box(d.root), CardKind::Reason, "Robots create jobs")
        .unwrap();
    d.add_card(Parent::Card(card), CardKind::Objection, "Unrelated words")
        .unwrap();
    assert!(of_kind(&run_checks(&d, &all_on()), CheckKind::KeyTerm).is_empty());

    // A grouping map has no cards in the tree; a parked card has no target.
    let mut d = doc("en", "Topic");
    d.add_box(Parent::Box(d.root), BoxKind::Category, "Group")
        .unwrap();
    d.add_loose(NewLoose::Card(CardKind::Reason), "Something else");
    assert!(of_kind(&run_checks(&d, &all_on()), CheckKind::KeyTerm).is_empty());
}

#[test]
fn compound_language_aware() {
    let text = "Robots of mensen";
    let mut nl = doc("nl", text);
    nl.add_card(Parent::Box(nl.root), CardKind::Reason, "Robots of mensen")
        .unwrap();
    let nl_issues = run_checks(&nl, &all_on());
    let compound = of_kind(&nl_issues, CheckKind::Compound);
    assert_eq!(compound.len(), 2);
    assert_eq!(compound[0].item, Item::Box(nl.root));
    assert_eq!(compound[0].terms, ["of"]);

    let en = doc("en", text);
    assert!(of_kind(&run_checks(&en, &all_on()), CheckKind::Compound).is_empty());

    let en = doc("en", "Cats AND dogs, or birds; Android and/or Oracle");
    let issues = run_checks(&en, &all_on());
    let compound = of_kind(&issues, CheckKind::Compound);
    assert_eq!(compound.len(), 1);
    assert_eq!(compound[0].terms, ["and", "or"]);

    // Unknown languages fall back to English.
    let fr = doc("fr", "Robots and humans");
    assert_eq!(
        of_kind(&run_checks(&fr, &all_on()), CheckKind::Compound).len(),
        1
    );
}

#[test]
fn key_term_ignores_english_negative_contractions() {
    let mut d = doc(
        "en",
        "Robots shouldn't replace teachers; they can't and won't",
    );
    d.add_card(
        Parent::Box(d.root),
        CardKind::Reason,
        "Teachers replace nothing",
    )
    .unwrap();
    let issues = run_checks(&d, &all_on());
    let key = of_kind(&issues, CheckKind::KeyTerm);
    assert_eq!(key.len(), 1);
    assert_eq!(key[0].terms, ["robots"]);
}

#[test]
fn language_code_variants_are_dutch() {
    for lang in ["nl", "NL", "nl-NL", "nl_BE"] {
        let d = doc(lang, "Robots of mensen");
        assert_eq!(
            of_kind(&run_checks(&d, &all_on()), CheckKind::Compound).len(),
            1,
            "{lang}"
        );
    }
    for lang in ["en", "nld", "fr", ""] {
        let d = doc(lang, "Robots of mensen");
        assert!(
            of_kind(&run_checks(&d, &all_on()), CheckKind::Compound).is_empty(),
            "{lang}"
        );
    }
}
