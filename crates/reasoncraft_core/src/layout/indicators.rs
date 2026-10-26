//! The indicator row: a band at the top of a box, above its image and
//! text, that holds the box's small marks (rating marker, link glyph,
//! comment badge; later the map numbering). A box only gets the row when
//! it shows one of them, so the marks never cover text and plain boxes
//! keep their size.

use crate::model::BoxNode;

/// Which marks the boxes show (what the app's settings turn on).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndicatorOptions {
    /// Rating markers (✓ / ? / ×) on rated boxes.
    pub markers: bool,
}

impl Default for IndicatorOptions {
    fn default() -> Self {
        Self { markers: true }
    }
}

/// Whether `node` shows a mark and so needs an indicator row: a rating
/// marker (when markers are on), a link or comments.
pub fn needs_indicator_row(node: &BoxNode, opts: &IndicatorOptions) -> bool {
    let rated = node.acceptance.is_some() || node.source_rating.is_some();
    (opts.markers && rated) || node.link.is_some() || !node.comments.is_empty()
}

/// The height of `node`'s indicator row: `row` when it needs one, else 0.
pub(super) fn row_height(node: &BoxNode, opts: &IndicatorOptions, row: f32) -> f32 {
    if needs_indicator_row(node, opts) {
        row
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NodeId;
    use crate::model::{Acceptance, BoxKind};

    /// Boxes with a mark grow by the row; the others keep their height.
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
        let p = LayoutParams::default();
        let before = layout(&doc, &FixedMeasure, &p);
        doc.boxes.get_mut(&a).unwrap().link = Some("https://example.org".into());
        doc.boxes.get_mut(&doc.root).unwrap().acceptance = Some(Acceptance::Accept);
        let after = layout(&doc, &FixedMeasure, &p);
        let h = |l: &crate::Layout, id| l.boxes[&id].height();
        assert_eq!(h(&after, b), h(&before, b), "plain box");
        assert_eq!(h(&after, a), h(&before, a) + p.indicator_row);
        assert_eq!(h(&after, doc.root), h(&before, doc.root) + p.indicator_row);
        assert_eq!(after.indicator_row(a), p.indicator_row);
        assert_eq!(after.indicator_row(b), 0.0);
        let body = after.body(a).unwrap();
        assert_eq!(body.height(), h(&before, a));
        // With markers hidden, a rating alone needs no row.
        let hidden = LayoutParams {
            indicators: IndicatorOptions { markers: false },
            ..LayoutParams::default()
        };
        let off = layout(&doc, &FixedMeasure, &hidden);
        assert_eq!(h(&off, doc.root), h(&before, doc.root));
    }

    #[test]
    fn row_only_for_boxes_with_marks() {
        let on = IndicatorOptions::default();
        let off = IndicatorOptions { markers: false };
        let mut b = BoxNode::new(NodeId::new(), BoxKind::Premise, "p");
        assert!(!needs_indicator_row(&b, &on));
        b.acceptance = Some(Acceptance::Accept);
        assert!(needs_indicator_row(&b, &on));
        assert!(!needs_indicator_row(&b, &off), "markers hidden");
        b.link = Some("https://example.org".into());
        assert!(needs_indicator_row(&b, &off));
        assert_eq!(row_height(&b, &off, 16.0), 16.0);
    }
}
