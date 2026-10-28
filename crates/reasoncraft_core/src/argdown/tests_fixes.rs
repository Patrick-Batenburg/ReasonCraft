//! What the import does with constructs it cannot place as written.

use super::ImportNoteKind as K;
use super::tests_import::import;
use crate::ids::Item;
use crate::model::BoxKind;

fn texts(d: &crate::model::Document) -> Vec<&str> {
    d.boxes.values().map(|b| b.text.as_str()).collect()
}

#[test]
fn refused_places_go_to_parking_with_a_note() {
    // A grouping map: a reason cannot hang under a category.
    let src = "\
[Topic]: Ways to travel
    <+ [Bus]: Bus {category: true}
        <+ [Cheap]: Buses are cheap.
";
    let (d, notes) = import(src);
    assert_eq!(notes, [K::MovedToParking]);
    let root = &d.boxes[&d.root];
    assert_eq!(root.children.len(), 1);
    let [Item::Box(parked)] = d.parking[..] else {
        panic!("{:?}", d.parking)
    };
    assert_eq!(d.boxes[&parked].text, "Buses are cheap.");
    assert_eq!(d.boxes[&parked].kind, BoxKind::Claim);
}

#[test]
fn relations_into_left_out_conclusions_are_reported() {
    let src = "\
[Claim]: The claim.
    <+ <A>

<A>

(1) [P]: A premise.
----
(2) [Step]: An intermediate step.
    <- [Doubt]: A doubt about the step.
(3) [Q]: Another premise.
----
(4) [Claim]

<Parked>

(1) [R]: A parked premise.
----
(2) No target. {target: none}
    <+ [Backing]: Backing for nothing.
";
    let (d, notes) = import(src);
    assert_eq!(
        notes,
        [
            K::IntermediateConclusion,
            K::RelationDropped,
            K::RelationDropped
        ]
    );
    // The statements of dropped relations are kept, in the parking area.
    let t = texts(&d);
    assert!(t.contains(&"A doubt about the step.") && t.contains(&"Backing for nothing."));
    assert!(!t.contains(&"An intermediate step.") && !t.contains(&"No target."));
}

#[test]
fn a_stray_rule_marker_ends_at_the_next_structure() {
    let src = "\
[A]: First.
    <+ <R>

<R>

(1) [B]: Second.
--
(2) [A]

[C]: Third.
    <+ [D]: Fourth.

[E]: Fifth.
";
    let (d, notes) = import(src);
    assert_eq!(notes, []);
    let t = texts(&d);
    for want in ["Second.", "Third.", "Fourth.", "Fifth."] {
        assert!(t.contains(&want), "{want} missing from {t:?}");
    }
    // A rule written over lines ends at a blank line too.
    let (d, notes) = import("<R>\n(1) [B]: b\n--\nModus ponens\n\n[C]: c\n");
    assert_eq!(notes, [K::InferenceRuleDropped]);
    assert!(texts(&d).contains(&"c"));
}

#[test]
fn second_wordings_links_and_odd_lines_are_noted_or_kept() {
    let src = "\
[A]: a {link: \"javascript:alert(1)\", note2: 'x } y // z'}
    <+ <R>

=== not frontmatter

<R>: First description.

<R>: Second description.

(1) [P]: p
----
(2) [A]

<R>

(1) [Q]: q
----
(2) [A]
";
    let (d, notes) = import(src);
    assert_eq!(
        notes,
        [
            K::LinkDropped,
            K::ExtraEquivalentWording,
            K::ExtraEquivalentWording
        ]
    );
    let t = texts(&d);
    assert!(t.contains(&"=== not frontmatter"), "{t:?}");
    assert!(d.boxes.values().all(|b| b.link.is_none()));
}

#[test]
fn import_caps_long_text_with_note() {
    use crate::model::MAX_BOX_CHARS;
    let long = "é".repeat(MAX_BOX_CHARS + 10);
    let src = format!("[Claim]: {long}\n    <+ Short reason.\n    <- [Long]: {long}\n");
    let (d, notes) = import(&src);
    assert_eq!(
        notes,
        [K::TextCut, K::TextCut],
        "the root and the objection"
    );
    let cut: Vec<&str> = texts(&d)
        .into_iter()
        .filter(|t| t.starts_with('é'))
        .collect();
    assert_eq!(cut.len(), 2);
    assert!(cut.iter().all(|t| t.chars().count() == MAX_BOX_CHARS));
    assert!(texts(&d).contains(&"Short reason."));
}

/// Texts the import takes from data and frontmatter (comments, a title,
/// an author, a too-long link) keep the caps a file's open applies, so a
/// map saved as Argdown and imported again is not cut on its next open
/// (CORE-R2).
#[test]
fn import_caps_comments_frontmatter_and_links() {
    use crate::caps::{MAX_COMMENT_CHARS, MAX_LINK_CHARS, MAX_META_CHARS};
    let comment = "c".repeat(MAX_COMMENT_CHARS + 5);
    let title = "t".repeat(MAX_META_CHARS + 5);
    let link = format!("https://example.org/{}", "a".repeat(MAX_LINK_CHARS));
    let src = format!(
        "===\ntitle: {title}\nauthor: {title}\n===\n\n[Claim]: c {{comments: [{{author: \"Bo\", text: \"{comment}\"}}], link: \"{link}\"}}\n"
    );
    let (d, notes) = import(&src);
    assert!(notes.contains(&K::TextCut), "{notes:?}");
    let root = &d.boxes[&d.root];
    assert_eq!(root.comments[0].text.chars().count(), MAX_COMMENT_CHARS);
    assert_eq!(root.link, None, "a cut link would lead elsewhere");
    assert_eq!(d.meta.title.chars().count(), MAX_META_CHARS);
    assert_eq!(d.meta.author.chars().count(), MAX_META_CHARS);
}
