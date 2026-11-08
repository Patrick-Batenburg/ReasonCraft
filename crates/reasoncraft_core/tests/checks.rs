//! Course-rule check tests.

use std::collections::HashSet;
use std::time::Instant;

use reasoncraft_core::checks::{CheckKind, CheckSet, Issue, run_checks};
use reasoncraft_core::fixtures::sample_iot_like;
use reasoncraft_core::{
    Acceptance, BoxKind, CardKind, CardStrength, Document, Item, NewLoose, Parent, SourceRating,
    SourceType,
};

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

fn source() -> BoxKind {
    BoxKind::Source {
        source_type: SourceType::Publication,
    }
}

#[test]
fn question_and_unsupported() {
    let mut d = doc("en", "  Do robots create jobs?  ");
    let root = Parent::Box(d.root);
    let (_, p1) = d.add_card(root, CardKind::Reason, "Robots work").unwrap();
    let (_, p2) = d.add_card(root, CardKind::Reason, "Robots learn").unwrap();
    let (_, p3) = d.add_card(root, CardKind::Reason, "Robots help?").unwrap();
    d.add_box(Parent::Box(p1), source(), "Report").unwrap();
    let (_, p2a) = d
        .add_card(Parent::Box(p2), CardKind::Reason, "Studies say so")
        .unwrap();
    // An objection does not support its target.
    d.add_card(Parent::Box(p3), CardKind::Objection, "Not really")
        .unwrap();

    let issues = run_checks(&d, &all_on());
    let questions: Vec<Item> = of_kind(&issues, CheckKind::Question)
        .iter()
        .map(|i| i.item)
        .collect();
    assert_eq!(questions, [Item::Box(d.root), Item::Box(p3)]);
    let unsupported: Vec<Item> = of_kind(&issues, CheckKind::Unsupported)
        .iter()
        .map(|i| i.item)
        .collect();
    assert!(!unsupported.contains(&Item::Box(p1)));
    assert!(!unsupported.contains(&Item::Box(p2)));
    assert!(unsupported.contains(&Item::Box(p2a)));
    assert!(unsupported.contains(&Item::Box(p3)));
    assert!(!unsupported.contains(&Item::Box(d.root)));
}

#[test]
fn empty_category_grouping_only() {
    let empty = |d: &Document| -> Vec<Item> {
        of_kind(&run_checks(d, &all_on()), CheckKind::EmptyCategory)
            .iter()
            .map(|i| i.item)
            .collect()
    };
    let mut d = doc("en", "Topic");
    let root = Parent::Box(d.root);
    // An empty heading directly under the root is flagged.
    let a = d.add_box(root, BoxKind::Category, "A").unwrap();
    // A childless category deeper down is content, not an empty heading.
    let b = d.add_box(root, BoxKind::Category, "B").unwrap();
    d.add_box(Parent::Box(b), BoxKind::Category, "B1").unwrap();
    // A root child holding only a note is not empty.
    let c = d.add_box(root, BoxKind::Category, "C").unwrap();
    d.add_box(Parent::Box(c), BoxKind::Note, "Note").unwrap();
    // A parked empty category is not flagged.
    d.add_loose(NewLoose::Category, "Loose group");
    assert_eq!(empty(&d), [Item::Box(a)]);

    // Reasoning maps never get the check.
    let mut d = doc("en", "Claim");
    d.add_card(Parent::Box(d.root), CardKind::Reason, "R")
        .unwrap();
    d.add_loose(NewLoose::Category, "Loose group");
    assert!(empty(&d).is_empty());
}

#[test]
fn incomplete_only_after_first_rating() {
    let mut d = doc("en", "Claim");
    let (card, p) = d
        .add_card(Parent::Box(d.root), CardKind::Reason, "Premise")
        .unwrap();
    let s = d.add_box(Parent::Box(p), source(), "Report").unwrap();
    assert!(of_kind(&run_checks(&d, &all_on()), CheckKind::IncompleteEvaluation).is_empty());

    // One issue on the first unrated item, carrying the count.
    let incomplete = |d: &Document| -> Vec<(Item, Vec<String>)> {
        of_kind(&run_checks(d, &all_on()), CheckKind::IncompleteEvaluation)
            .iter()
            .map(|i| (i.item, i.terms.clone()))
            .collect()
    };
    d.set_acceptance(d.root, Some(Acceptance::Accept)).unwrap();
    assert_eq!(incomplete(&d), [(Item::Card(card), vec!["3".to_owned()])]);

    // A source rating alone also counts as "a rating exists".
    d.set_acceptance(d.root, None).unwrap();
    d.set_source_rating(s, Some(SourceRating::Shaky)).unwrap();
    assert_eq!(incomplete(&d), [(Item::Box(d.root), vec!["3".to_owned()])]);

    // Everything rated: no issue.
    d.set_acceptance(d.root, Some(Acceptance::Accept)).unwrap();
    d.set_acceptance(p, Some(Acceptance::Accept)).unwrap();
    d.set_card_strength(card, Some(CardStrength::Strong))
        .unwrap();
    assert!(incomplete(&d).is_empty());
}

#[test]
fn parked_items_are_drafts() {
    let mut d = doc("en", "Claim");
    let (card, p) = d
        .add_card(Parent::Box(d.root), CardKind::Reason, "Claim holds")
        .unwrap();
    d.add_box(Parent::Box(p), source(), "Report").unwrap();
    let note = d.add_loose(NewLoose::Note, "Idea");
    let parked = d.add_loose(NewLoose::Card(CardKind::Reason), "Draft");
    let Item::Card(parked_card) = parked else {
        panic!()
    };
    let parked_premise = d.cards[&parked_card].premises[0];

    // A rating on a parked item does not start the evaluation.
    d.set_acceptance(parked_premise, Some(Acceptance::Accept))
        .unwrap();
    let issues = run_checks(&d, &all_on());
    assert!(of_kind(&issues, CheckKind::IncompleteEvaluation).is_empty());
    // The parked premise has no support, but it is a draft.
    assert!(
        !of_kind(&issues, CheckKind::Unsupported)
            .iter()
            .any(|i| i.item == Item::Box(parked_premise))
    );
    // The parked card is listed; the parked note is not.
    let loose: Vec<Item> = of_kind(&issues, CheckKind::NotConnected)
        .iter()
        .map(|i| i.item)
        .collect();
    assert_eq!(loose, [parked]);
    assert!(!loose.contains(&note));

    // Once the main map is rated, parked items are not counted.
    d.set_acceptance(d.root, Some(Acceptance::Accept)).unwrap();
    let issues = run_checks(&d, &all_on());
    let incomplete = of_kind(&issues, CheckKind::IncompleteEvaluation);
    assert_eq!(incomplete.len(), 1);
    assert_eq!(incomplete[0].item, Item::Card(card));
    assert_eq!(incomplete[0].terms, ["3"]);
}

#[test]
fn dutch_sample_is_not_noisy() {
    // The course sample: a claim, three reasons each with a source, a
    // parked card and a parked note; only the claim and one card rated.
    let mut d = doc("nl", "Robots kosten banen");
    let root = Parent::Box(d.root);
    let mut cards = Vec::new();
    for (premise, src) in [
        ("Robots kosten banen in fabrieken", "CBS-rapport"),
        ("Robots kosten banen in magazijnen", "Krantenartikel"),
        ("Robots kosten banen bij banken", "Interview"),
    ] {
        let (card, p) = d.add_card(root, CardKind::Reason, premise).unwrap();
        d.add_box(Parent::Box(p), source(), src).unwrap();
        cards.push(card);
    }
    d.add_loose(
        NewLoose::Card(CardKind::Objection),
        "Er komen ook banen bij",
    );
    d.add_loose(NewLoose::Note, "Nog uitzoeken");
    d.set_acceptance(d.root, Some(Acceptance::Accept)).unwrap();
    d.set_card_strength(cards[0], Some(CardStrength::Strong))
        .unwrap();

    let issues = run_checks(&d, &all_on());
    assert!(issues.len() <= 8, "{} issues: {issues:?}", issues.len());
    let incomplete = of_kind(&issues, CheckKind::IncompleteEvaluation);
    assert_eq!(incomplete.len(), 1, "{issues:?}");
    // Two cards, three premises and three sources are not rated yet.
    assert_eq!(incomplete[0].terms, ["8"]);
    // The first unrated item in tree order: the rated card's premise.
    let first = d.cards[&cards[0]].premises[0];
    assert_eq!(incomplete[0].item, Item::Box(first));
}

#[test]
fn inconsistent_cases() {
    let mut d = doc("en", "Claim");
    let root = Parent::Box(d.root);
    // 1. Strong card whose premises are all rejected.
    let (strong, s1) = d.add_card(root, CardKind::Reason, "One").unwrap();
    let s2 = d.add_premise(strong, None, "Two").unwrap();
    d.set_card_strength(strong, Some(CardStrength::Strong))
        .unwrap();
    d.set_acceptance(s1, Some(Acceptance::Reject)).unwrap();
    d.set_acceptance(s2, Some(Acceptance::Reject)).unwrap();
    // 2. Solid source under a rejected premise.
    let src = d.add_box(Parent::Box(s1), source(), "Report").unwrap();
    d.set_source_rating(src, Some(SourceRating::Solid)).unwrap();
    // 3. Nil card with accepted premises and no objection.
    let (nil, n1) = d.add_card(root, CardKind::Reason, "Three").unwrap();
    d.set_card_strength(nil, Some(CardStrength::Nil)).unwrap();
    d.set_acceptance(n1, Some(Acceptance::Accept)).unwrap();
    // Not inconsistent: nil card with an objection against its premise.
    let (nil2, m1) = d.add_card(root, CardKind::Reason, "Four").unwrap();
    d.set_card_strength(nil2, Some(CardStrength::Nil)).unwrap();
    d.set_acceptance(m1, Some(Acceptance::Accept)).unwrap();
    d.add_card(Parent::Box(m1), CardKind::Objection, "Against")
        .unwrap();
    // Not inconsistent: nil card attacked as a whole.
    let (nil3, k1) = d.add_card(root, CardKind::Reason, "Five").unwrap();
    d.set_card_strength(nil3, Some(CardStrength::Nil)).unwrap();
    d.set_acceptance(k1, Some(Acceptance::Accept)).unwrap();
    d.add_card(Parent::Card(nil3), CardKind::Objection, "Undercut")
        .unwrap();
    // Not inconsistent: strong card with one accepted premise.
    let (ok, o1) = d.add_card(root, CardKind::Reason, "Six").unwrap();
    let o2 = d.add_premise(ok, None, "Seven").unwrap();
    d.set_card_strength(ok, Some(CardStrength::Strong)).unwrap();
    d.set_acceptance(o1, Some(Acceptance::Reject)).unwrap();
    d.set_acceptance(o2, Some(Acceptance::Accept)).unwrap();

    let issues = run_checks(&d, &all_on());
    let bad: Vec<Item> = of_kind(&issues, CheckKind::InconsistentEvaluation)
        .iter()
        .map(|i| i.item)
        .collect();
    assert_eq!(bad, [Item::Card(strong), Item::Box(src), Item::Card(nil)]);
}

#[test]
fn not_connected_lists_parking() {
    let mut d = doc("en", "Claim?");
    let a = d.add_loose(NewLoose::Note, "Loose note");
    let b = d.add_loose(NewLoose::Card(CardKind::Reason), "Loose reason?");
    let issues = run_checks(&d, &all_on());
    let loose: Vec<Item> = of_kind(&issues, CheckKind::NotConnected)
        .iter()
        .map(|i| i.item)
        .collect();
    // A parked note is a jotting, not an unconnected argument part.
    assert!(!loose.contains(&a));
    assert_eq!(loose, [b]);
    // Main tree first, then parking in pre-order: the parked card, then its
    // premise.
    let Item::Card(card) = b else { panic!() };
    let premise = Item::Box(d.cards[&card].premises[0]);
    let order: Vec<(Item, CheckKind)> = issues.iter().map(|i| (i.item, i.kind)).collect();
    assert_eq!(
        order,
        [
            (Item::Box(d.root), CheckKind::Question),
            (b, CheckKind::NotConnected),
            (premise, CheckKind::Question),
        ]
    );
}

#[test]
fn issues_follow_display_order_and_kind_order() {
    // Sources come first under a box, like on the canvas and in the outline;
    // within one item the kinds follow `CheckKind::ALL`.
    let mut d = doc("en", "Robots and jobs?");
    let (card, p) = d
        .add_card(Parent::Box(d.root), CardKind::Reason, "Machines or people?")
        .unwrap();
    let s = d.add_box(Parent::Box(d.root), source(), "Report").unwrap();
    d.set_source_rating(s, Some(SourceRating::Solid)).unwrap();
    let order: Vec<(Item, CheckKind)> = run_checks(&d, &all_on())
        .iter()
        .map(|i| (i.item, i.kind))
        .collect();
    assert_eq!(
        order,
        [
            (Item::Box(d.root), CheckKind::Compound),
            (Item::Box(d.root), CheckKind::Question),
            (Item::Box(d.root), CheckKind::IncompleteEvaluation),
            (Item::Card(card), CheckKind::KeyTerm),
            (Item::Box(p), CheckKind::Compound),
            (Item::Box(p), CheckKind::Question),
            (Item::Box(p), CheckKind::Unsupported),
        ]
    );
}

#[test]
fn disabled_checks_skipped() {
    let mut d = doc("en", "Robots and jobs?");
    d.add_loose(NewLoose::Note, "Loose");
    let set = CheckSet {
        disabled: vec![CheckKind::Question, CheckKind::NotConnected],
    };
    assert!(!set.is_on(CheckKind::Question));
    assert!(set.is_on(CheckKind::Compound));
    let kinds: Vec<CheckKind> = run_checks(&d, &set).iter().map(|i| i.kind).collect();
    assert_eq!(kinds, [CheckKind::Compound]);

    let none = CheckSet {
        disabled: CheckKind::ALL.to_vec(),
    };
    assert!(run_checks(&d, &none).is_empty());
}

#[test]
fn check_set_serde() {
    assert!(CheckKind::ALL.iter().all(|&k| CheckSet::default().is_on(k)));
    assert_eq!(
        serde_json::to_string(&CheckKind::IncompleteEvaluation).unwrap(),
        "\"incomplete_evaluation\""
    );
    let set: CheckSet = serde_json::from_str("{}").unwrap();
    assert_eq!(set, CheckSet::default());
    let set = CheckSet {
        disabled: vec![CheckKind::KeyTerm, CheckKind::NotConnected],
    };
    let json = serde_json::to_string(&set).unwrap();
    assert_eq!(json, r#"{"disabled":["key_term","not_connected"]}"#);
    assert_eq!(serde_json::from_str::<CheckSet>(&json).unwrap(), set);
}

#[test]
fn damaged_cycle_does_not_hang() {
    // A card listed under its own premise: traversal must stop.
    let mut d = doc("en", "Claim");
    let (card, p) = d
        .add_card(Parent::Box(d.root), CardKind::Reason, "P")
        .unwrap();
    d.boxes.get_mut(&p).unwrap().children.push(Item::Card(card));
    let issues = run_checks(&d, &all_on());
    let pairs: HashSet<(Item, CheckKind)> = issues.iter().map(|i| (i.item, i.kind)).collect();
    assert_eq!(
        pairs.len(),
        issues.len(),
        "an item was visited twice: {issues:?}"
    );
    assert!(pairs.contains(&(Item::Card(card), CheckKind::KeyTerm)));
}

#[test]
#[ignore = "timing: run in release"]
fn checks_500_boxes_fast() {
    let mut doc = sample_iot_like(500);
    doc.set_acceptance(doc.root, Some(Acceptance::Accept))
        .unwrap();
    let set = CheckSet::default();
    let mut best = std::time::Duration::MAX;
    for _ in 0..20 {
        let t = Instant::now();
        let issues = run_checks(&doc, &set);
        best = best.min(t.elapsed());
        assert!(!issues.is_empty());
    }
    eprintln!("run_checks on 500 boxes: {best:?}");
    assert!(best.as_millis() < 20, "took {best:?}");
}

/// `n` distinct made-up words of five letters, spread over many starts.
fn made_up_words(n: usize, salt: usize) -> Vec<String> {
    (0..n)
        .map(|i| {
            let mut k = i * 7919 + salt;
            (0..5)
                .map(|_| {
                    let c = (b'a' + (k % 26) as u8) as char;
                    k /= 26;
                    c
                })
                .collect()
        })
        .collect()
}

#[test]
fn key_terms_of_long_boxes_keep_order_and_matching() {
    // Repeats, near matches by prefix and long text: the missing terms
    // stay in order, once each, and prefix matches still count.
    let mut target: Vec<String> = made_up_words(600, 0);
    target.extend(["automatisering", "robots", "robots", "zzzzz"].map(String::from));
    let mut premise: Vec<String> = made_up_words(600, 0)[..300].to_vec();
    premise.extend(["automatiseren", "robot"].map(String::from));
    let mut d = doc("en", &target.join(" "));
    let (card, _) = d
        .add_card(Parent::Box(d.root), CardKind::Reason, &premise.join(" "))
        .unwrap();
    let issues = run_checks(&d, &all_on());
    let key = of_kind(&issues, CheckKind::KeyTerm);
    assert_eq!(key.len(), 1);
    assert_eq!(key[0].item, Item::Card(card));
    let mut expected: Vec<String> = Vec::new();
    for w in &made_up_words(600, 0)[300..] {
        if !expected.contains(w) && !premise.iter().any(|p| shares_term(w, p)) {
            expected.push(w.clone());
        }
    }
    expected.push("zzzzz".into());
    assert_eq!(key[0].terms, expected);
}

/// The key-term rule, written out: equal, or sharing at least
/// `max(4, shorter - 2)` leading characters.
fn shares_term(a: &str, b: &str) -> bool {
    let shorter = a.chars().count().min(b.chars().count());
    let shared = a.chars().zip(b.chars()).take_while(|(x, y)| x == y).count();
    a == b || shared >= 4.max(shorter.saturating_sub(2))
}

#[test]
#[ignore = "timing: run in release"]
fn key_terms_of_7000_word_boxes_fast() {
    let target = made_up_words(7000, 1).join(" ");
    let premise = made_up_words(7000, 2).join(" ");
    let mut d = doc("en", &target);
    d.add_card(Parent::Box(d.root), CardKind::Reason, &premise)
        .unwrap();
    let set = CheckSet::default();
    let mut best = std::time::Duration::MAX;
    for _ in 0..5 {
        let t = Instant::now();
        let issues = run_checks(&d, &set);
        best = best.min(t.elapsed());
        assert!(!of_kind(&issues, CheckKind::KeyTerm).is_empty());
    }
    eprintln!("run_checks on two 7000-word boxes: {best:?}");
    assert!(best.as_millis() < 30, "took {best:?}");
}
