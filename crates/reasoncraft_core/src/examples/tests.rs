use super::*;
use crate::checks::{CheckKind, CheckSet, run_checks};
use crate::essay::EssaySlot;
use crate::ids::Item;
use crate::model::{BoxKind, CardKind, MapKind, Role};

const LANGS: [&str; 2] = ["en", "nl"];

fn all_docs(lang: &str) -> Vec<(String, Document)> {
    let examples =
        [Example::Reasoning, Example::Grouping].map(|e| (format!("{e:?}"), example(e, lang)));
    let templates = Template::ALL.map(|t| (format!("{t:?}"), template(t, lang)));
    examples.into_iter().chain(templates).collect()
}

fn issues_of(doc: &Document, kind: CheckKind) -> usize {
    run_checks(doc, &CheckSet::default())
        .iter()
        .filter(|i| i.kind == kind)
        .count()
}

#[test]
fn examples_and_templates_validate_in_both_languages() {
    for lang in LANGS {
        for (name, doc) in all_docs(lang) {
            let at = format!("{name} in {lang}");
            doc.validate().unwrap_or_else(|e| panic!("{at}: {e}"));
            assert_eq!(doc.meta.language, lang, "{at}");
            assert!(doc.parking.is_empty(), "{at}: nothing is loose");
            assert_eq!(issues_of(&doc, CheckKind::NotConnected), 0, "{at}");
            for kind in [CheckKind::Compound, CheckKind::Question] {
                assert_eq!(issues_of(&doc, kind), 0, "{at}: {kind:?}");
            }
            assert!(
                doc.boxes.values().all(|b| !b.text.trim().is_empty()),
                "{at}: every box has text"
            );
        }
    }
    // The two languages say the same thing in their own words.
    for (en, nl) in all_docs("en").into_iter().zip(all_docs("nl")) {
        assert_eq!(en.1.boxes.len(), nl.1.boxes.len(), "{}", en.0);
        assert_eq!(en.1.cards.len(), nl.1.cards.len(), "{}", en.0);
        let root = |d: &Document| d.boxes[&d.root].text.clone();
        assert_ne!(root(&en.1), root(&nl.1), "{}", en.0);
    }
    // Any other language reads as English.
    assert_eq!(example(Example::Grouping, "fr").meta.language, "en");
    assert_eq!(template(Template::PeerReview, "nl-BE").meta.language, "nl");
}

/// The reasoning example shows every part of a reasoning map, fully and
/// consistently rated, with every premise backed.
#[test]
fn reasoning_example_shows_every_part() {
    for lang in LANGS {
        let doc = example(Example::Reasoning, lang);
        assert_eq!(doc.map_kind(), MapKind::Reasoning);
        let root = &doc.boxes[&doc.root];
        let cards: Vec<_> = root
            .children
            .iter()
            .filter_map(|i| match i {
                Item::Card(c) => Some(&doc.cards[c]),
                Item::Box(_) => None,
            })
            .collect();
        let count = |k: CardKind| cards.iter().filter(|c| c.kind == k).count();
        assert_eq!(
            (count(CardKind::Reason), count(CardKind::Objection)),
            (3, 2)
        );
        assert!(
            doc.cards.values().any(|c| c.premises.len() > 1),
            "co-premises"
        );
        assert!(doc.cards.keys().any(|&c| doc.card_role(c) == Role::Rebuts));
        let whole_card = doc.cards.values().flat_map(|c| &c.children).count();
        assert!(whole_card >= 1, "a whole-card objection");
        let mut types: Vec<_> = doc
            .boxes
            .values()
            .filter_map(|b| match &b.kind {
                BoxKind::Source { source_type } => Some(source_type.key()),
                _ => None,
            })
            .collect();
        types.sort_unstable();
        types.dedup();
        assert!(types.len() >= 5, "sources of several types: {types:?}");
        assert!(doc.boxes.values().any(|b| b.kind == BoxKind::Note));
        assert!(root.acceptance.is_some());
        for kind in [
            CheckKind::Unsupported,
            CheckKind::IncompleteEvaluation,
            CheckKind::InconsistentEvaluation,
        ] {
            assert_eq!(issues_of(&doc, kind), 0, "{lang}: {kind:?}");
        }
        assert!(!doc.meta.title.is_empty());
    }
}

/// The grouping example goes three levels of categories deep.
#[test]
fn grouping_example_is_three_levels_deep() {
    fn depth(doc: &Document, id: crate::ids::NodeId) -> usize {
        let node = &doc.boxes[&id];
        let below = node.children.iter().filter_map(|i| match i {
            Item::Box(b) if doc.boxes[b].kind == BoxKind::Category => Some(depth(doc, *b)),
            _ => None,
        });
        let own = usize::from(node.kind == BoxKind::Category);
        own + below.max().unwrap_or(0)
    }
    for lang in LANGS {
        let doc = example(Example::Grouping, lang);
        assert_eq!(doc.map_kind(), MapKind::Grouping);
        assert_eq!(depth(&doc, doc.root), 3, "{lang}");
        assert_eq!(issues_of(&doc, CheckKind::EmptyCategory), 0);
        assert!(doc.boxes.values().any(|b| b.kind == BoxKind::Note));
    }
}

#[test]
fn templates_have_their_parts() {
    for lang in LANGS {
        let root_cards = |d: &Document, kind: CardKind| {
            let root = &d.boxes[&d.root];
            root.children
                .iter()
                .filter(|i| matches!(i, Item::Card(c) if d.cards[c].kind == kind))
                .count()
        };
        let d = template(Template::BlankReasoning, lang);
        assert_eq!(root_cards(&d, CardKind::Reason), 1);
        assert_eq!(root_cards(&d, CardKind::Objection), 1);

        let d = template(Template::BlankGrouping, lang);
        assert_eq!(d.map_kind(), MapKind::Grouping);
        assert_eq!(d.boxes[&d.root].children.len(), 3);

        let d = template(Template::EssayPlan, lang);
        assert_eq!(root_cards(&d, CardKind::Reason), 3);
        assert_eq!(root_cards(&d, CardKind::Objection), 1);
        assert!(d.cards.keys().any(|&c| d.card_role(c) == Role::Rebuts));
        assert!(d.essay.text(EssaySlot::Intro).is_some());
        assert!(d.essay.text(EssaySlot::Conclusion).is_some());

        let d = template(Template::PeerReview, lang);
        let note = d.boxes[&d.root]
            .children
            .iter()
            .any(|i| matches!(i, Item::Box(b) if d.boxes[b].kind == BoxKind::Note));
        assert!(note, "a note explains the presets");
        assert!(d.meta.title.is_empty(), "a template's title is the user's");
    }
}
