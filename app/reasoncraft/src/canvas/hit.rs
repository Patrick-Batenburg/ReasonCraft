//! What lies under a point of the map.

use eframe::egui::{Pos2, Rect};
use reasoncraft_core::{CardId, Layout, NodeId};

/// Height of the band around a card frame's bottom edge that counts as the
/// edge itself (the drop zone for whole-card targets), in screen px.
pub const BOTTOM_EDGE_BAND: f32 = 8.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    Box(NodeId),
    CardFrame(CardId),
    CardBottomEdge(CardId),
}

/// The item at `world`: a box first, then a card frame's bottom-edge band
/// ([`BOTTOM_EDGE_BAND`] screen px at `zoom`), then a card frame.
pub fn hit_test(layout: &Layout, world: Pos2, zoom: f32) -> Option<Hit> {
    if let Some((&id, _)) = layout.boxes.iter().find(|(_, r)| r.contains(world)) {
        return Some(Hit::Box(id));
    }
    let half = BOTTOM_EDGE_BAND / zoom / 2.0;
    let band =
        |r: &Rect| Rect::from_x_y_ranges(r.x_range(), (r.bottom() - half)..=(r.bottom() + half));
    if let Some((&id, _)) = layout.cards.iter().find(|(_, r)| band(r).contains(world)) {
        return Some(Hit::CardBottomEdge(id));
    }
    layout
        .cards
        .iter()
        .find(|(_, r)| r.contains(world))
        .map(|(&id, _)| Hit::CardFrame(id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{Pos2, vec2};
    use reasoncraft_core::fixtures::FixedMeasure;
    use reasoncraft_core::{CardKind, Document, LayoutParams, Parent, layout};

    fn two_premise_card() -> (Document, CardId, NodeId) {
        let mut d = Document::new("Claim");
        let root = d.root;
        let (c, p) = d
            .add_card(Parent::Box(root), CardKind::Reason, "One")
            .unwrap();
        d.add_premise(c, None, "Two").unwrap();
        (d, c, p)
    }

    #[test]
    fn hit_prefers_box_over_frame() {
        let (d, c, p) = two_premise_card();
        let l = layout(&d, &FixedMeasure, &LayoutParams::default());
        let premise = l.boxes[&p];
        let frame = l.cards[&c];
        assert!(frame.contains_rect(premise));
        assert_eq!(hit_test(&l, premise.center(), 1.0), Some(Hit::Box(p)));
        // The label strip above the premises belongs to the frame.
        let label = Pos2::new(frame.center().x, frame.top() + 4.0);
        assert_eq!(hit_test(&l, label, 1.0), Some(Hit::CardFrame(c)));
        assert_eq!(
            hit_test(&l, l.boxes[&d.root].center(), 1.0),
            Some(Hit::Box(d.root))
        );
        assert_eq!(hit_test(&l, frame.left_top() - vec2(30.0, 30.0), 1.0), None);
    }

    #[test]
    fn hit_bottom_edge_band() {
        let (d, c, _) = two_premise_card();
        let l = layout(&d, &FixedMeasure, &LayoutParams::default());
        let frame = l.cards[&c];
        let edge = frame.center_bottom();
        for dy in [-3.5, 0.0, 3.5] {
            assert_eq!(
                hit_test(&l, edge + vec2(0.0, dy), 1.0),
                Some(Hit::CardBottomEdge(c)),
                "dy = {dy}"
            );
        }
        assert_eq!(hit_test(&l, edge + vec2(0.0, 6.0), 1.0), None);
        assert_eq!(hit_test(&l, edge + vec2(frame.width(), 0.0), 1.0), None);
        // The band is 8 screen px: at 200% it is 4 world units tall.
        let below = edge + vec2(0.0, 1.75);
        assert_eq!(hit_test(&l, below, 2.0), Some(Hit::CardBottomEdge(c)));
        assert_eq!(hit_test(&l, edge + vec2(0.0, 3.0), 2.0), None);
        // At 50% it is 16 world units tall.
        assert_eq!(
            hit_test(&l, edge + vec2(0.0, 7.5), 0.5),
            Some(Hit::CardBottomEdge(c))
        );
    }
}
