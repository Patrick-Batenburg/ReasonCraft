use super::to_argdown;
use crate::ids::{Item, Parent};
use crate::model::{
    Acceptance, BoxKind, CardKind, CardStrength, Document, SourceRating, SourceType, Verdict,
};

/// The frontmatter of a document without title or author, in English.
const HEAD: &str =
    "===\ntitle: \"\"\nauthor: \"\"\nlanguage: \"en\"\ngenerator: \"ReasonCraft\"\n===\n\n";

#[test]
fn writes_frontmatter_and_root() {
    let mut d = Document::new("Cats are good pets");
    d.set_meta("Pets", "Ann \"A\" \\ B").unwrap();
    d.set_language("nl").unwrap();
    d.set_acceptance(d.root, Some(Acceptance::Accept)).unwrap();
    let expected = "===
title: \"Pets\"
author: \"Ann \\\"A\\\" \\\\ B\"
language: \"nl\"
generator: \"ReasonCraft\"
===

[Cats are good pets]: Cats are good pets {acceptance: accept}
";
    assert_eq!(to_argdown(&d), expected);
}

#[test]
fn reason_with_two_premises_as_pcs() {
    let mut d = Document::new("Cats are good pets");
    let (card, _) = d
        .add_card(Parent::Box(d.root), CardKind::Reason, "They are quiet")
        .unwrap();
    d.add_premise(card, None, "They clean themselves").unwrap();
    let expected = format!(
        "{HEAD}[Cats are good pets]: Cats are good pets
    <+ <R1>

<R1> {{kind: reason}}

(1) [They are quiet]: They are quiet
(2) [They clean themselves]: They clean themselves
----
(3) [Cats are good pets]
"
    );
    assert_eq!(to_argdown(&d), expected);
}

#[test]
fn objection_targets_premise() {
    let mut d = Document::new("Claim");
    let (_, alpha) = d
        .add_card(Parent::Box(d.root), CardKind::Reason, "Alpha")
        .unwrap();
    d.add_card(Parent::Box(alpha), CardKind::Objection, "Beta")
        .unwrap();
    let expected = format!(
        "{HEAD}[Claim]: Claim
    <+ <R1>

<R1> {{kind: reason}}

(1) [Alpha]: Alpha
    <- <O1>
----
(2) [Claim]

<O1> {{kind: objection}}

(1) [Beta]: Beta
----
(2) Not: Alpha
    -> [Alpha]
"
    );
    assert_eq!(to_argdown(&d), expected);
}

#[test]
fn whole_card_attack_under_inference() {
    let mut d = Document::new("Claim");
    let (r1, _) = d
        .add_card(Parent::Box(d.root), CardKind::Reason, "Alpha")
        .unwrap();
    d.add_card(Parent::Card(r1), CardKind::Objection, "Gamma")
        .unwrap();
    d.add_card(Parent::Card(r1), CardKind::Reason, "Delta")
        .unwrap();
    let expected = format!(
        "{HEAD}[Claim]: Claim
    <+ <R1>

<R1> {{kind: reason}}

(1) [Alpha]: Alpha
----
    <_ <O1>
    <+ <R2>
(2) [Claim]

<O1> {{kind: objection}}

(1) [Gamma]: Gamma
----
(2) The inference in @<R1> does not hold.

<R2> {{kind: reason}}

(1) [Delta]: Delta
----
(2) The inference in @<R1> holds.
"
    );
    assert_eq!(to_argdown(&d), expected);
}

#[test]
fn sources_and_ratings_in_data() {
    let mut d = Document::new("Claim");
    let root = d.root;
    let (card, alpha) = d
        .add_card(Parent::Box(root), CardKind::Reason, "Alpha")
        .unwrap();
    d.set_card_strength(card, Some(CardStrength::Weak)).unwrap();
    d.set_acceptance(alpha, Some(Acceptance::Reject)).unwrap();
    let publication = BoxKind::Source {
        source_type: SourceType::Publication,
    };
    let src = d
        .add_box(Parent::Box(alpha), publication, "Smith 2020")
        .unwrap();
    d.set_source_rating(src, Some(SourceRating::Solid)).unwrap();
    d.set_link(src, Some("https://x.org/a".into())).unwrap();
    d.add_box(Parent::Box(src), BoxKind::Note, "checked\ntwice")
        .unwrap();
    d.add_comment(src, "Bo", "Fine", Some(Verdict::Positive))
        .unwrap();
    d.add_comment(src, "Cy", "Say \"why\"", None).unwrap();
    let other = BoxKind::Source {
        source_type: SourceType::Other("podcast".into()),
    };
    d.add_box(Parent::Box(root), other, "Episode 4").unwrap();
    d.add_box(Parent::Box(root), BoxKind::Note, "Root note")
        .unwrap();

    let out = to_argdown(&d);
    let expected_root = "[Claim]: Claim {notes: [\"Root note\"]}
    <+ <R1>
    <+ [Episode 4]: Episode 4 {source: podcast}
";
    assert!(out.contains(expected_root), "{out}");
    let expected_card = "<R1> {kind: reason, strength: weak}

(1) [Alpha]: Alpha {acceptance: reject}
    <+ [Smith 2020]: Smith 2020 {source: publication, rating: solid, \
link: \"https://x.org/a\", notes: [\"checked\\ntwice\"], comments: \
[{author: \"Bo\", text: \"Fine\", verdict: positive}, {author: \"Cy\", text: \"Say \\\"why\\\"\"}]}
----
(2) [Claim]
";
    assert!(out.contains(expected_card), "{out}");
}

#[test]
fn grouping_map_uses_category_data() {
    let mut d = Document::new("Topic");
    let industry = d
        .add_box(Parent::Box(d.root), BoxKind::Category, "Industry")
        .unwrap();
    d.add_box(Parent::Box(industry), BoxKind::Note, "n1")
        .unwrap();
    d.add_box(Parent::Box(industry), BoxKind::Category, "Steel")
        .unwrap();
    d.add_box(Parent::Box(d.root), BoxKind::Category, "Farming")
        .unwrap();
    let expected = format!(
        "{HEAD}[Topic]: Topic
    <+ [Industry]: Industry {{category: true, notes: [\"n1\"]}}
        <+ [Steel]: Steel {{category: true}}
    <+ [Farming]: Farming {{category: true}}
"
    );
    assert_eq!(to_argdown(&d), expected);
}

#[test]
fn parking_after_heading() {
    use crate::edit::NewLoose;
    let mut d = Document::new("Claim");
    d.add_card(Parent::Box(d.root), CardKind::Reason, "Alpha")
        .unwrap();
    let Item::Card(loose) = d.add_loose(NewLoose::Card(CardKind::Objection), "Loose") else {
        panic!("a loose card");
    };
    let loose_premise = d.cards[&loose].premises[0];
    d.add_card(Parent::Box(loose_premise), CardKind::Reason, "Under")
        .unwrap();
    d.add_loose(NewLoose::Source(SourceType::Web), "Web page");
    d.add_loose(NewLoose::Note, "Remember");
    d.set_language("nl").unwrap();
    let out = to_argdown(&d);
    let expected_tail = "(2) [Claim]

# Niet verbonden

<O1> {kind: objection}

(1) [Loose]: Loose
    <+ <R2>
----
(2) No target. {target: none}

<R2> {kind: reason}

(1) [Under]: Under
----
(2) [Loose]

[Web page]: Web page {source: web}

[Remember]: Remember {note: true}
";
    assert!(out.ends_with(expected_tail), "{out}");
}

#[test]
fn titles_unique_and_sanitised() {
    let mut d = Document::new("Same text");
    let (card, _) = d
        .add_card(Parent::Box(d.root), CardKind::Reason, "Same text")
        .unwrap();
    for text in [
        "[a] <b> {c}: #d @e f g h",
        "Pneumonoultramicroscopicsilicovolcanoconiosis is long",
        "",
        "Statement 6",
        "snake_case *bold* // two\nlines\\",
        "- - dashed start",
    ] {
        d.add_premise(card, None, text).unwrap();
    }
    let out = to_argdown(&d);
    let expected = "(1) [Same text 2]: Same text
(2) [a b c d e]: \\[a\\] \\<b\\> \\{c\\}: \\#d \\@e f g h
(3) [Pneumonoultramicroscopicsilicovolcanocon]: Pneumonoultramicroscopicsilicovolcanoconiosis is long
(4) [Statement 5]:
(5) [Statement 6]: Statement 6
(6) [snake_case *bold* / two lines]: snake\\_case \\*bold\\* \\// two lines\\\\
(7) [dashed start]: - - dashed start
----
(8) [Same text]
";
    assert!(out.contains(expected), "{out}");
    assert!(out.starts_with(&format!("{HEAD}[Same text]: Same text\n")));
}

#[test]
fn braces_and_equals_are_escaped_in_quoted_values() {
    let mut d = Document::new("Claim");
    d.set_meta("A === B", "x=y").unwrap();
    d.add_comment(d.root, "Bo", "a} b {c", None).unwrap();
    let out = to_argdown(&d);
    let front = "===\ntitle: \"A \\u003D\\u003D\\u003D B\"\nauthor: \"x\\u003Dy\"\n";
    assert!(out.starts_with(front), "{out}");
    let comment = "{comments: [{author: \"Bo\", text: \"a\\u007D b \\u007Bc\"}]}";
    assert!(out.contains(comment), "{out}");
    assert_eq!(out.matches("===").count(), 2, "{out}");
}

#[test]
fn every_card_gets_one_argument_block() {
    let d = crate::fixtures::sample_iot_like(60);
    let out = to_argdown(&d);
    let headers = out
        .lines()
        .filter(|l| l.starts_with("<R") || l.starts_with("<O"))
        .count();
    assert_eq!(headers, d.cards.len(), "{out}");
    let definitions: Vec<&str> = out
        .lines()
        .filter_map(|l| {
            let l = l.trim_start();
            let l = l.strip_prefix("<+ ").unwrap_or(l);
            let l = l.split_once(") ").map_or(l, |(_, rest)| rest);
            l.strip_prefix('[')?.split_once("]:").map(|(t, _)| t)
        })
        .collect();
    let unique: std::collections::HashSet<_> = definitions.iter().collect();
    assert_eq!(unique.len(), definitions.len(), "a title is defined twice");
    assert!(out.ends_with('\n') && !out.contains("\n\n\n") && !out.contains(" \n"));
}

/// The map behind the first example in `docs/argdown.md`.
fn docs_example() -> Document {
    use crate::edit::NewLoose;
    let mut d = Document::new("The town centre should be car-free.");
    d.set_meta("Car-free centre", "Sam").unwrap();
    let root = d.root;
    d.set_acceptance(root, Some(Acceptance::Accept)).unwrap();
    let (r1, noise) = d
        .add_card(
            Parent::Box(root),
            CardKind::Reason,
            "Cars make the centre noisy and unsafe.",
        )
        .unwrap();
    d.add_premise(r1, None, "A quiet centre draws more visitors.")
        .unwrap();
    d.set_card_strength(r1, Some(CardStrength::Strong)).unwrap();
    let statistic = BoxKind::Source {
        source_type: SourceType::Statistic,
    };
    let report = d
        .add_box(Parent::Box(noise), statistic, "Traffic report 2024")
        .unwrap();
    d.set_source_rating(report, Some(SourceRating::Solid))
        .unwrap();
    let (_, vans) = d
        .add_card(
            Parent::Box(root),
            CardKind::Objection,
            "Shops need delivery vans.",
        )
        .unwrap();
    d.add_card(
        Parent::Box(vans),
        CardKind::Objection,
        "Deliveries can be made before 10 am.",
    )
    .unwrap();
    d.add_card(
        Parent::Card(r1),
        CardKind::Objection,
        "The report covers only one street.",
    )
    .unwrap();
    d.add_comment(
        vans,
        "Sam",
        "Ask the shop owners.",
        Some(Verdict::NeedsWork),
    )
    .unwrap();
    d.add_loose(
        NewLoose::Card(CardKind::Reason),
        "Buses could run more often.",
    );
    d.add_loose(NewLoose::Note, "Check the bus routes.");
    d
}

#[test]
fn docs_example_matches_the_writer() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/argdown.md");
    let docs = std::fs::read_to_string(path).unwrap();
    let out = to_argdown(&docs_example());
    let block = format!("```argdown\n{out}```");
    assert!(docs.contains(&block), "docs/argdown.md should show:\n{out}");
}
