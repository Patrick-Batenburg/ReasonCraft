//! Outline builder tests (step 3, task 2).

use std::time::Instant;

use reasoncraft_core::fixtures::sample_iot_like;
use reasoncraft_core::outline::{
    LineKind, LinkLabel, OutlineLine, OutlineOptions, build_outline, outline_plain_text,
};
use reasoncraft_core::{BoxKind, CardKind, Document, NewLoose, Parent, SourceType};

fn label_word(l: LinkLabel) -> String {
    match l {
        LinkLabel::Because => "because",
        LinkLabel::But => "but",
        LinkLabel::However => "however",
        LinkLabel::None => "",
    }
    .to_owned()
}

fn source_word(t: &SourceType) -> String {
    t.name().to_owned()
}

fn numbers(lines: &[OutlineLine]) -> Vec<&str> {
    lines.iter().map(|l| l.number.as_str()).collect()
}

/// Claim; reason with two co-premises and a whole-card attack; a second
/// reason; an objection with a rebuttal.
fn sample() -> Document {
    let mut d = Document::new("Claim");
    let root = Parent::Box(d.root);
    let (a, _) = d.add_card(root, CardKind::Reason, "R1a").unwrap();
    d.add_premise(a, None, "R1b").unwrap();
    d.add_card(Parent::Card(a), CardKind::Objection, "Whole")
        .unwrap();
    d.add_card(root, CardKind::Reason, "R2").unwrap();
    let (_, o) = d.add_card(root, CardKind::Objection, "O3").unwrap();
    d.add_card(Parent::Box(o), CardKind::Objection, "Reb")
        .unwrap();
    d
}

#[test]
fn numbering_and_labels() {
    let lines = build_outline(&sample(), &OutlineOptions::default());
    assert_eq!(numbers(&lines), ["", "1a", "1b", "1.w1", "2", "3", "3.1"]);
    let labels: Vec<LinkLabel> = lines.iter().map(|l| l.label).collect();
    assert_eq!(
        labels,
        [
            LinkLabel::None,
            LinkLabel::Because,
            LinkLabel::Because,
            LinkLabel::But,
            LinkLabel::Because,
            LinkLabel::But,
            LinkLabel::However
        ]
    );
    assert!(matches!(lines[0].kind, LineKind::Claim));
    assert_eq!(
        lines.iter().map(|l| l.whole_card).collect::<Vec<_>>(),
        [false, false, false, true, false, false, false]
    );
    assert!(lines.iter().all(|l| !l.parked));
}

#[test]
fn sources_under_premise_and_parked_section() {
    let mut d = Document::new("Claim");
    let root = Parent::Box(d.root);
    let (_, p) = d.add_card(root, CardKind::Reason, "Reason").unwrap();
    d.add_card(Parent::Box(p), CardKind::Objection, "Inner")
        .unwrap();
    let src = |t| BoxKind::Source { source_type: t };
    d.add_box(Parent::Box(p), src(SourceType::Web), "first")
        .unwrap();
    d.add_box(Parent::Box(p), src(SourceType::Law), "second")
        .unwrap();
    d.add_loose(NewLoose::Card(CardKind::Reason), "Loose");
    d.add_loose(NewLoose::Category, "Group");

    let lines = build_outline(&d, &OutlineOptions::default());
    let texts: Vec<&str> = lines.iter().map(|l| l.text.as_str()).collect();
    assert_eq!(
        texts,
        [
            "Claim", "Reason", "first", "second", "Inner", "Loose", "Group"
        ]
    );
    assert_eq!(numbers(&lines), ["", "1", "", "", "1.1", "P1", "P2"]);
    assert_eq!(lines[2].depth, 2);
    assert!(matches!(&lines[2].kind, LineKind::Source(SourceType::Web)));
    assert_eq!(
        lines.iter().map(|l| l.parked).collect::<Vec<_>>(),
        [false, false, false, false, false, true, true]
    );
}

#[test]
fn parked_fragments_continue_numbering() {
    let mut d = Document::new("Claim");
    let reasoncraft_core::Item::Card(c) = d.add_loose(NewLoose::Card(CardKind::Reason), "Loose")
    else {
        unreachable!()
    };
    let premise = d.cards[&c].premises[0];
    d.add_card(Parent::Box(premise), CardKind::Objection, "Inner")
        .unwrap();
    let reasoncraft_core::Item::Box(g) = d.add_loose(NewLoose::Category, "Group") else {
        unreachable!()
    };
    d.add_box(Parent::Box(g), BoxKind::Category, "Sub").unwrap();
    let lines = build_outline(&d, &OutlineOptions::default());
    assert_eq!(numbers(&lines), ["", "P1", "P1.1", "P2", "P2.1"]);
    assert!(lines[1..].iter().all(|l| l.parked));
}

#[test]
fn grouping_categories_number_like_cards() {
    let mut d = Document::new("Claim");
    let root = Parent::Box(d.root);
    let c = d.add_box(root, BoxKind::Category, "A").unwrap();
    d.add_box(root, BoxKind::Category, "B").unwrap();
    d.add_box(Parent::Box(c), BoxKind::Category, "A1").unwrap();
    let lines = build_outline(&d, &OutlineOptions::default());
    assert_eq!(numbers(&lines), ["", "1", "1.1", "2"]);
    assert!(matches!(lines[1].kind, LineKind::Category));
}

#[test]
fn notes_only_when_enabled() {
    let mut d = Document::new("Claim");
    let root = Parent::Box(d.root);
    d.add_box(root, BoxKind::Note, "memo").unwrap();
    d.add_card(root, CardKind::Reason, "Reason").unwrap();
    d.add_loose(NewLoose::Note, "loose memo");
    let off = build_outline(&d, &OutlineOptions::default());
    assert_eq!(off.len(), 2);
    let on = build_outline(
        &d,
        &OutlineOptions {
            notes: true,
            ..OutlineOptions::default()
        },
    );
    let texts: Vec<&str> = on.iter().map(|l| l.text.as_str()).collect();
    assert_eq!(texts, ["Claim", "memo", "Reason", "loose memo"]);
    // Notes do not take a number.
    assert_eq!(numbers(&on), ["", "", "1", "P1"]);
}

#[test]
fn plain_text_matches_expected() {
    let mut d = sample();
    let root = Parent::Box(d.root);
    d.add_box(
        root,
        BoxKind::Source {
            source_type: SourceType::Web,
        },
        "site",
    )
    .unwrap();
    let opts = OutlineOptions::default();
    let text = outline_plain_text(&build_outline(&d, &opts), &opts, &label_word, &source_word);
    let expected = "Claim\n  [web] site\n  1a because R1a\n  1b because R1b\n    1.w1 but Whole\n  2 because R2\n  3 but O3\n    3.1 however Reb";
    assert_eq!(text, expected);

    let bare = OutlineOptions {
        numbering: false,
        labels: false,
        notes: false,
    };
    let text = outline_plain_text(&build_outline(&d, &bare), &bare, &label_word, &source_word);
    assert!(text.starts_with("Claim\n  [web] site\n  R1a\n  R1b\n    Whole\n"));
}

#[test]
fn plain_text_indents_continuation_lines_and_skips_empty_parts() {
    let mut d = Document::new("Claim");
    let root = Parent::Box(d.root);
    let (_, p) = d
        .add_card(root, CardKind::Reason, "first line\r\nsecond line")
        .unwrap();
    let web = BoxKind::Source {
        source_type: SourceType::Web,
    };
    d.add_box(Parent::Box(p), web.clone(), "").unwrap();
    d.add_box(Parent::Box(p), web, "page\n\nvisited").unwrap();
    d.add_card(root, CardKind::Reason, "").unwrap();
    let opts = OutlineOptions::default();
    let text = outline_plain_text(&build_outline(&d, &opts), &opts, &label_word, &source_word);
    let expected = "Claim\n  1 because first line\n  second line\n    [web]\n    [web] page\n\n    visited\n  2 because";
    assert_eq!(text, expected);
    assert!(text.lines().all(|l| !l.ends_with(' ')), "trailing space");
}

#[test]
fn deep_chain_200_no_overflow() {
    let mut d = Document::new("Top");
    let mut parent = Parent::Box(d.root);
    for i in 0..200 {
        let (_, p) = d
            .add_card(parent, CardKind::Reason, &format!("level {i}"))
            .unwrap();
        parent = Parent::Box(p);
    }
    let lines = build_outline(&d, &OutlineOptions::default());
    assert_eq!(lines.len(), 201);
    assert_eq!(lines[200].depth, 200);
    assert_eq!(lines[200].number, vec!["1"; 200].join("."));
}

#[test]
#[ignore = "timing: run in release"]
fn outline_500_boxes_fast() {
    let doc = sample_iot_like(500);
    let opts = OutlineOptions::default();
    let mut best = std::time::Duration::MAX;
    for _ in 0..20 {
        let t = Instant::now();
        let lines = build_outline(&doc, &opts);
        best = best.min(t.elapsed());
        assert!(lines.len() >= 400);
    }
    assert!(best.as_micros() < 5_000, "took {best:?}");
}
