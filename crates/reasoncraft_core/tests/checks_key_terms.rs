//! The key-term check on long boxes: the missing terms in order, and its
//! speed.

use std::time::Instant;

use reasoncraft_core::checks::{CheckKind, CheckSet, Issue, run_checks};
use reasoncraft_core::{CardKind, Document, Item, Parent};

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
