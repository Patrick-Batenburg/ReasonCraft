use super::{ArgdownError, ImportNoteKind as K, from_argdown, to_argdown};
use crate::edit::NewLoose;
use crate::fixtures::{sample_iot_like, structurally_equal};
use crate::ids::{Item, Parent};
use crate::model::{
    Acceptance, BoxKind, CardKind, CardStrength, Document, SourceRating, SourceType, Verdict,
};

pub(super) fn import(src: &str) -> (Document, Vec<K>) {
    let (doc, report) = from_argdown(src).unwrap_or_else(|e| panic!("{e}\n{src}"));
    doc.validate().unwrap();
    (doc, report.notes.into_iter().map(|n| n.kind).collect())
}

fn round_trip(d: &Document) {
    let text = to_argdown(d);
    let (back, notes) = import(&text);
    assert_eq!(notes, vec![], "{text}");
    if let Err(e) = structurally_equal(d, &back) {
        panic!("{e}\n{text}");
    }
    // Reading the import's own text back changes nothing more.
    let again = to_argdown(&back);
    assert_eq!(to_argdown(&import(&again).0), again);
}

/// A map with odd text, links, comments and every kind of parked item.
fn parked_map() -> Document {
    let mut d = Document::new(
        "  Root with [brackets], {braces}, #hash, @at, *star* and _under_ // not a comment ",
    );
    d.set_meta("A === B {x}", "Ann \"A\"").unwrap();
    let root = d.root;
    d.set_link(root, Some("https://example.org/a?b=c#d".into()))
        .unwrap();
    let (card, p) = d
        .add_card(Parent::Box(root), CardKind::Reason, "Line one\nline two")
        .unwrap();
    d.set_acceptance(p, Some(Acceptance::Unsure)).unwrap();
    d.add_comment(p, "Bo", "Odd \"quote\" {x}\nnext\ttab \\", None)
        .unwrap();
    d.add_box(Parent::Box(p), BoxKind::Note, "A note\nwith a break")
        .unwrap();
    d.add_premise(card, None, "").unwrap();
    d.set_card_strength(card, Some(CardStrength::Nil)).unwrap();
    let Item::Card(loose) = d.add_loose(NewLoose::Card(CardKind::Objection), "Loose") else {
        unreachable!()
    };
    d.add_card(Parent::Card(loose), CardKind::Reason, "Backing")
        .unwrap();
    let Item::Box(cat) = d.add_loose(NewLoose::Category, "Parked category") else {
        unreachable!()
    };
    d.add_box(Parent::Box(cat), BoxKind::Category, "Sub")
        .unwrap();
    d.add_box(Parent::Box(cat), BoxKind::Note, "Cat note")
        .unwrap();
    let Item::Box(src) = d.add_loose(
        NewLoose::Source(SourceType::Other("Field notes".into())),
        "Notebook",
    ) else {
        unreachable!()
    };
    d.set_source_rating(src, Some(SourceRating::Shaky)).unwrap();
    d.add_box(Parent::Box(src), BoxKind::Note, "Page 4")
        .unwrap();
    let Item::Box(note) = d.add_loose(NewLoose::Note, "- Remember -") else {
        unreachable!()
    };
    d.set_link(note, Some("mailto:a@b.org".into())).unwrap();
    d.add_comment(note, "Cy", "ok", Some(Verdict::Problem))
        .unwrap();
    d
}

fn grouping_map() -> Document {
    let mut d = Document::new("Ways to get around town");
    d.set_language("nl").unwrap();
    let root = d.root;
    let bus = d
        .add_box(Parent::Box(root), BoxKind::Category, "Public transport")
        .unwrap();
    d.add_box(Parent::Box(bus), BoxKind::Note, "Check the timetables.")
        .unwrap();
    let buses = d
        .add_box(Parent::Box(bus), BoxKind::Category, "Buses")
        .unwrap();
    d.add_comment(buses, "Kim", "Late at night?", Some(Verdict::Positive))
        .unwrap();
    d.add_box(Parent::Box(bus), BoxKind::Category, "Trams")
        .unwrap();
    d.add_box(Parent::Box(root), BoxKind::Category, "Cycling")
        .unwrap();
    let web = BoxKind::Source {
        source_type: SourceType::Web,
    };
    d.add_box(Parent::Box(root), web, "City website").unwrap();
    d
}

#[test]
fn round_trip_preserves_structure() {
    round_trip(&sample_iot_like(60));
    round_trip(&parked_map());
    round_trip(&grouping_map());
    round_trip(&super::tests_write::docs_example());
    round_trip(&Document::new(""));
}

/// A long chain of objections, in non-Latin script: nothing recurses.
#[test]
fn deep_chain_round_trips() {
    let mut d = Document::new("Wurzel — 日本語");
    let mut p = d.root;
    for k in 0..600 {
        let (_, next) = d
            .add_card(
                Parent::Box(p),
                CardKind::Objection,
                &format!("Stufe {k} — 日本"),
            )
            .unwrap();
        p = next;
    }
    round_trip(&d);
}

/// The ```argdown blocks of `docs/argdown.md`.
fn docs_blocks() -> Vec<String> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/argdown.md");
    let docs = std::fs::read_to_string(path).unwrap();
    let blocks: Vec<String> = docs
        .split("```argdown\n")
        .skip(1)
        .filter_map(|b| b.split_once("```").map(|(b, _)| b.to_owned()))
        .collect();
    assert_eq!(blocks.len(), 2);
    blocks
}

#[test]
fn imports_docs_examples() {
    let blocks = docs_blocks();
    let (map, notes) = import(&blocks[0]);
    assert_eq!(notes, vec![]);
    structurally_equal(&super::tests_write::docs_example(), &map).unwrap();

    let (d, notes) = import(&blocks[1]);
    assert_eq!(notes, vec![]);
    let root = &d.boxes[&d.root];
    assert_eq!(root.text, "The town centre should be car-free.");
    let [Item::Card(sample)] = root.children[..] else {
        panic!("{:?}", root.children)
    };
    let card = &d.cards[&sample];
    assert_eq!(card.kind, CardKind::Reason);
    let premise = &d.boxes[&card.premises[0]];
    assert_eq!(
        premise.text,
        "Most residents in a survey want a car-free centre."
    );
    let [undercut] = card.children[..] else {
        panic!("{:?}", card.children)
    };
    let undercut = &d.cards[&undercut];
    assert_eq!(undercut.kind, CardKind::Objection);
    let text = &d.boxes[&undercut.premises[0]].text;
    assert_eq!(text, "Only 40 of 12 000 residents answered the survey.");
    assert!(d.parking.is_empty());
}

#[test]
fn import_reports_dropped_constructs() {
    let src = "\
[A]: Coffee is good for you. #health
    + [B]: Coffee contains antioxidants.
        + [A]
    >< [D]: Coffee is bad for you.
    <+ <Chain>

[B]: Coffee has antioxidants in it.

[E]: Tea is good for you.
    + [B]

<Chain>

(1) [P]: Beans are roasted.
----
(2) [Q]: Roasting makes antioxidants.
(3) [R]: Antioxidants protect cells.
----
(4) [A]
";
    let (d, notes) = import(src);
    let mut kinds: Vec<&str> = notes.iter().map(K::key).collect();
    kinds.sort();
    assert_eq!(
        kinds,
        [
            "contradiction",
            "cycle_broken",
            "extra_equivalent_wording",
            "intermediate_conclusion",
            "shared_statement_copied",
            "tags_dropped",
        ]
    );
    let root = &d.boxes[&d.root];
    assert_eq!(root.text, "Coffee is good for you.");
    // [D] and [E] (with its copy of [B]) are parked.
    assert_eq!(d.parking.len(), 2);
    // The chain's premises: (1) and (3); (2) was an intermediate step.
    let chain = d.cards.values().find(|c| c.premises.len() == 2).unwrap();
    let texts: Vec<&str> = chain
        .premises
        .iter()
        .map(|p| d.boxes[p].text.as_str())
        .collect();
    assert_eq!(texts, ["Beans are roasted.", "Antioxidants protect cells."]);
}

#[test]
fn extra_top_level_claims_go_to_parking() {
    let src = "\
[Small]: A claim with one reason.
    <+ [S1]: One.

[Big]: The claim with the most support.
    <+ [B1]: First.
    <+ [B2]: Second.

[Lone]: A claim on its own.
";
    let (d, notes) = import(src);
    assert_eq!(notes, vec![]);
    assert_eq!(d.boxes[&d.root].text, "The claim with the most support.");
    let parked: Vec<(&BoxKind, &str)> = d
        .parking
        .iter()
        .map(|i| match i {
            Item::Box(b) => (&d.boxes[b].kind, d.boxes[b].text.as_str()),
            Item::Card(_) => panic!("a card"),
        })
        .collect();
    assert_eq!(
        parked,
        [
            (&BoxKind::Claim, "A claim with one reason."),
            (&BoxKind::Claim, "A claim on its own."),
        ]
    );
    let Item::Box(small) = d.parking[0] else {
        unreachable!()
    };
    assert_eq!(d.boxes[&small].children.len(), 1);
}

#[test]
fn import_rejects_oversized_input() {
    let big = "a".repeat(super::MAX_ARGDOWN_BYTES + 1);
    assert!(matches!(from_argdown(&big), Err(ArgdownError::TooLarge)));
    let mut deep = String::from("[A]\n");
    for d in 1..=super::MAX_NESTING + 1 {
        deep.push_str(&format!("{}<+ [B{d}]\n", "    ".repeat(d)));
    }
    assert!(matches!(
        from_argdown(&deep),
        Err(ArgdownError::TooDeep { .. })
    ));
    let many: String = (0..=super::MAX_IMPORT_BOXES)
        .map(|k| format!("[S{k}]: s\n\n"))
        .collect();
    assert!(matches!(
        from_argdown(&many),
        Err(ArgdownError::TooManyBoxes)
    ));
    assert!(matches!(
        from_argdown(" \n// only a comment\n"),
        Err(ArgdownError::Empty)
    ));
}

#[test]
fn reports_the_other_constructs() {
    let src = "\
[Claim]: A claim.
    _ [U]: An undercut of a statement.
    <+ [Nowhere]

[Other]: Another claim, parked.

----
(1) [Claim]

<Ruled>: Ruled.

(1) [P]: A premise.
-- Modus ponens --
(2) [Claim]
";
    let (d, notes) = import(src);
    assert_eq!(
        notes,
        [
            K::UnknownReference("Nowhere".into()),
            K::InferenceRuleDropped,
            K::UndercutOnStatement,
            K::EmptyArgument,
        ]
    );
    let root = &d.boxes[&d.root];
    // The undercut became an objection; the unknown reference a reason
    // with its title as text; the ruled argument a reason.
    let kinds: Vec<CardKind> = root
        .children
        .iter()
        .map(|i| match i {
            Item::Card(c) => d.cards[c].kind.clone(),
            Item::Box(_) => panic!("a box"),
        })
        .collect();
    assert_eq!(
        kinds,
        [CardKind::Objection, CardKind::Reason, CardKind::Reason]
    );
}
