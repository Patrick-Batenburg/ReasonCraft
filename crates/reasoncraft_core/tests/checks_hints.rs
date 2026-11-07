//! Ignored hints and quoted text in the checks (step 5, task 14).

use reasoncraft_core::checks::{CheckKind, CheckSet, run_checks};
use reasoncraft_core::{CardKind, Document, IgnoredHint, Item, Parent, RemoveMode};

fn doc(lang: &str, claim: &str) -> Document {
    let mut d = Document::new(claim);
    d.meta.language = lang.to_owned();
    d
}

fn compound_count(d: &Document) -> usize {
    run_checks(d, &CheckSet::default())
        .iter()
        .filter(|i| i.kind == CheckKind::Compound)
        .count()
}

#[test]
fn ignored_hint_hides_issue_and_survives_save() {
    let mut d = doc("en", "Robots help or hurt");
    let issue = run_checks(&d, &CheckSet::default())
        .into_iter()
        .find(|i| i.kind == CheckKind::Compound)
        .unwrap();
    d.ignore_hint(&issue).unwrap();
    assert_eq!(compound_count(&d), 0);
    assert!(d.ignore_hint(&issue).is_err(), "twice is a no-op");

    let json = serde_json::to_string(&d).unwrap();
    let back: Document = serde_json::from_str(&json).unwrap();
    assert_eq!(back.ignored_hints, d.ignored_hints);
    assert_eq!(compound_count(&back), 0);
    assert!(back.extra.is_empty());

    // A changed set of terms is a different hint.
    let mut changed = back.clone();
    let root = changed.root;
    changed.boxes[&root].text = "Robots help and hurt or fail".into();
    assert_eq!(compound_count(&changed), 1);

    d.restore_hints().unwrap();
    assert_eq!(compound_count(&d), 1);
    assert!(d.restore_hints().is_err());
    let json = serde_json::to_string(&d).unwrap();
    assert!(!json.contains("ignored_hints"), "{json}");
}

#[test]
fn dangling_hint_is_harmless() {
    let mut d = doc("en", "Robots help");
    let (card, premise) = d
        .add_card(Parent::Box(d.root), CardKind::Reason, "Robots help")
        .unwrap();
    d.boxes[&premise].text = "Robots help or hurt".into();
    let issue = run_checks(&d, &CheckSet::default())
        .into_iter()
        .find(|i| i.kind == CheckKind::Compound)
        .unwrap();
    d.ignore_hint(&issue).unwrap();
    d.remove_item(Item::Card(card), RemoveMode::Branch).unwrap();
    assert!(d.ignored_hints.is_empty(), "removed with the item");

    // A hint left in a file for an item that does not exist.
    d.ignored_hints.push(IgnoredHint {
        kind: "compound".into(),
        item: issue.item,
        terms: vec!["or".into()],
        extra: Default::default(),
    });
    d.ignored_hints.push(IgnoredHint {
        kind: "future_kind".into(),
        item: Item::Box(d.root),
        terms: Vec::new(),
        extra: Default::default(),
    });
    d.validate().unwrap();
    assert!(run_checks(&d, &CheckSet::default()).is_empty());
}

#[test]
fn compound_skips_quotes() {
    let d = doc("nl", "Hij zei \"brood en kaas\" vandaag");
    assert_eq!(compound_count(&d), 0);
    assert_eq!(compound_count(&doc("nl", "Brood en kaas zijn duur")), 1);
    for quoted in [
        "Hij zei \u{201C}brood en kaas\u{201D} vandaag",
        "Hij zei \u{201E}brood en kaas\u{201D} vandaag",
        "Hij zei \u{2018}brood en kaas\u{2019} vandaag",
    ] {
        assert_eq!(compound_count(&doc("nl", quoted)), 0, "{quoted}");
    }
    // Text outside the quotes is still checked; an open quote hides nothing.
    let outside = doc("nl", "Brood en \"kaas\" zijn duur");
    assert_eq!(compound_count(&outside), 1);
    assert_eq!(compound_count(&doc("nl", "Hij zei \"brood en kaas")), 1);
}
