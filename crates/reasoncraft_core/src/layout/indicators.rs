//! The indicator row: a band at the top of a box, above its image and
//! text, that holds the box's small marks (map number, rating marker, link
//! glyph, comment badge). A box only gets the row when it shows one of
//! them, so the marks never cover text and plain boxes keep their size.

use std::collections::HashMap;

use crate::ids::NodeId;
use crate::model::{BoxNode, Document};
use crate::outline::box_numbers;

/// Which marks the boxes show (what the app's settings turn on).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndicatorOptions {
    /// Rating markers (✓ / ? / ×) on rated boxes.
    pub markers: bool,
    /// The outline number ("1.2a") on each numbered box.
    pub numbers: bool,
    /// Number boxes as the Outline does with its Notes switch on (parked
    /// notes get a "P" number), so map and Outline numbers agree.
    pub numbers_notes: bool,
}

impl Default for IndicatorOptions {
    fn default() -> Self {
        Self {
            markers: true,
            numbers: false,
            numbers_notes: false,
        }
    }
}

/// Whether `node` shows a mark and so needs an indicator row: its number
/// (when numbers are on and it is `numbered`), a rating marker (when
/// markers are on), a link or comments.
pub fn needs_indicator_row(node: &BoxNode, numbered: bool, opts: &IndicatorOptions) -> bool {
    let rated = node.acceptance.is_some() || node.source_rating.is_some();
    (opts.numbers && numbered)
        || (opts.markers && rated)
        || node.link.is_some()
        || !node.comments.is_empty()
}

/// The marks of a layout: the indicator row height (`row`) of each box of
/// `doc` that needs one, and the number of each numbered box (none when
/// numbers are off). When one premise of a card needs the row, its
/// co-premises reserve it too, so their first text lines align.
pub(super) struct Marks {
    pub rows: HashMap<NodeId, f32>,
    pub numbers: HashMap<NodeId, String>,
}

impl Marks {
    pub(super) fn new(doc: &Document, opts: &IndicatorOptions, row: f32) -> Self {
        let numbers = if opts.numbers {
            box_numbers(doc, opts.numbers_notes)
        } else {
            HashMap::new()
        };
        let mut rows: HashMap<NodeId, f32> = doc
            .boxes
            .iter()
            .filter(|(id, node)| needs_indicator_row(node, numbers.contains_key(id), opts))
            .map(|(&id, _)| (id, row))
            .collect();
        for card in doc.cards.values() {
            if card.premises.iter().any(|p| rows.contains_key(p)) {
                let present = card.premises.iter().filter(|p| doc.boxes.contains_key(*p));
                rows.extend(present.map(|&p| (p, row)));
            }
        }
        Self { rows, numbers }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NodeId;
    use crate::model::{Acceptance, BoxKind};

    /// Boxes with a mark grow by the row, and so do their co-premises;
    /// the others keep their height.
    #[test]
    fn indicator_row_adds_to_marked_boxes_only() {
        use crate::fixtures::FixedMeasure;
        use crate::layout::{LayoutParams, layout};
        use crate::{CardKind, Document, Parent};
        let mut doc = Document::new("Claim");
        let (card, a) = doc
            .add_card(Parent::Box(doc.root), CardKind::Reason, "A")
            .unwrap();
        let b = doc.add_premise(card, None, "B").unwrap();
        let (_, c) = doc
            .add_card(Parent::Box(doc.root), CardKind::Reason, "C")
            .unwrap();
        let p = LayoutParams::default();
        let before = layout(&doc, &FixedMeasure, &p);
        doc.boxes.get_mut(&a).unwrap().link = Some("https://example.org".into());
        doc.boxes.get_mut(&doc.root).unwrap().acceptance = Some(Acceptance::Accept);
        let after = layout(&doc, &FixedMeasure, &p);
        let h = |l: &crate::Layout, id| l.boxes[&id].height();
        assert_eq!(h(&after, c), h(&before, c), "plain box");
        assert_eq!(h(&after, a), h(&before, a) + p.indicator_row);
        assert_eq!(h(&after, b), h(&before, b) + p.indicator_row, "co-premise");
        assert_eq!(h(&after, doc.root), h(&before, doc.root) + p.indicator_row);
        assert_eq!(after.indicator_row(a), p.indicator_row);
        assert_eq!(after.indicator_row(c), 0.0);
        let body = after.body(a).unwrap();
        assert_eq!(body.height(), h(&before, a));
        // With markers hidden, a rating alone needs no row.
        let hidden = LayoutParams {
            indicators: IndicatorOptions {
                markers: false,
                ..IndicatorOptions::default()
            },
            ..LayoutParams::default()
        };
        let off = layout(&doc, &FixedMeasure, &hidden);
        assert_eq!(h(&off, doc.root), h(&before, doc.root));
    }

    #[test]
    fn row_only_for_boxes_with_marks() {
        let on = IndicatorOptions::default();
        let off = IndicatorOptions {
            markers: false,
            ..IndicatorOptions::default()
        };
        let mut b = BoxNode::new(NodeId::new(), BoxKind::Premise, "p");
        assert!(!needs_indicator_row(&b, false, &on));
        assert!(!needs_indicator_row(&b, true, &on), "numbers hidden");
        let numbers = IndicatorOptions {
            numbers: true,
            ..off
        };
        assert!(needs_indicator_row(&b, true, &numbers));
        assert!(!needs_indicator_row(&b, false, &numbers), "not numbered");
        b.acceptance = Some(Acceptance::Accept);
        assert!(needs_indicator_row(&b, false, &on));
        assert!(!needs_indicator_row(&b, false, &off), "markers hidden");
        b.link = Some("https://example.org".into());
        assert!(needs_indicator_row(&b, false, &off));
    }

    /// With numbers on, exactly the boxes the outline numbers get a row.
    #[test]
    fn numbered_boxes_get_the_row() {
        use crate::fixtures::sample_iot_like;
        let doc = sample_iot_like(40);
        let opts = IndicatorOptions {
            markers: false,
            numbers: true,
            numbers_notes: false,
        };
        let numbers = box_numbers(&doc, false);
        assert!(numbers.len() > 20, "a well-numbered sample");
        let with_marks = |id: &NodeId| needs_indicator_row(&doc.boxes[id], false, &opts);
        let on = Marks::new(&doc, &opts, 16.0);
        assert_eq!(on.numbers, numbers);
        for id in doc.boxes.keys() {
            let expect = numbers.contains_key(id) || with_marks(id);
            assert_eq!(on.rows.contains_key(id), expect, "{id:?}");
        }
        let off = IndicatorOptions {
            numbers: false,
            ..opts
        };
        let off = Marks::new(&doc, &off, 16.0);
        assert!(off.rows.len() < on.rows.len() && off.numbers.is_empty());
    }

    /// With the Outline's Notes switch on, a parked note takes a "P" number
    /// and a row, and the parked items after it are numbered past it.
    #[test]
    fn numbers_follow_the_outline_notes_switch() {
        use crate::outline::{OutlineOptions, build_outline};
        use crate::{CardKind, NewLoose};
        let mut doc = Document::new("Claim");
        let note = match doc.add_loose(NewLoose::Note, "Parked note") {
            crate::Item::Box(b) => b,
            other => panic!("{other:?}"),
        };
        doc.add_loose(NewLoose::Card(CardKind::Reason), "Parked reason");
        assert_eq!(doc.parking[0], crate::Item::Box(note), "note first");
        for notes in [false, true] {
            let opts = IndicatorOptions {
                markers: false,
                numbers: true,
                numbers_notes: notes,
            };
            let marks = Marks::new(&doc, &opts, 16.0);
            let outline = OutlineOptions {
                notes,
                ..OutlineOptions::default()
            };
            let expected: HashMap<NodeId, String> = build_outline(&doc, &outline)
                .into_iter()
                .filter_map(|l| match l.item {
                    crate::Item::Box(id) if !l.number.is_empty() => Some((id, l.number)),
                    _ => None,
                })
                .collect();
            assert_eq!(marks.numbers, expected, "notes {notes}");
            assert_eq!(marks.rows.contains_key(&note), notes, "notes {notes}");
            assert_eq!(marks.numbers.contains_key(&note), notes);
        }
    }
}
