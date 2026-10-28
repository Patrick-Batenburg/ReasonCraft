//! Inside a card frame: the rating label band and the premises.
//!
//! The label band is always at the top of the frame in world space, with
//! the label right-aligned in it. The premises follow each other along the
//! cross axis, `premise_gap` apart: side by side when the cross axis is x,
//! stacked below the band when it is y (left-to-right).

use emath::{Pos2, Rect, Vec2};

use super::axis::Frame;
use super::{Layout, LayoutParams};
use crate::ids::{CardId, NodeId};

/// The world size of a card frame around premises of the given sizes.
pub(super) fn frame_size(premises: &[Vec2], p: &LayoutParams, frame: Frame) -> Vec2 {
    let gaps = premises.len().saturating_sub(1) as f32 * p.premise_gap;
    let sum = |f: fn(&Vec2) -> f32| premises.iter().map(f).sum::<f32>() + gaps;
    let max = |f: fn(&Vec2) -> f32| premises.iter().map(f).fold(0.0, f32::max);
    let (width, height) = if frame.cross_is_y() {
        (max(|s| s.x), sum(|s| s.y))
    } else {
        (sum(|s| s.x), max(|s| s.y))
    };
    Vec2::new(
        width + 2.0 * p.card_pad,
        p.card_label_height + height + p.card_pad + p.card_bottom_extra,
    )
}

/// Each premise's cross centre relative to the frame's cross centre, for
/// premises of the given sizes. Where the cross axis is y, the centre
/// depends on the whole frame height (label band, `card_bottom_extra`
/// included), so this goes through [`frame_size`] like [`place`] does.
pub(super) fn premise_offsets(premises: &[Vec2], p: &LayoutParams, frame: Frame) -> Vec<f32> {
    let size = frame_size(premises, p, frame);
    let (mut at, centre) = if frame.cross_is_y() {
        (p.card_label_height, size.y / 2.0)
    } else {
        (p.card_pad, size.x / 2.0)
    };
    premises
        .iter()
        .map(|s| {
            let along = frame.ext(*s).cross;
            let offset = at + along / 2.0 - centre;
            at += along + p.premise_gap;
            offset
        })
        .collect()
}

/// Record card `id`'s frame `rect`, its label anchor and its premises'
/// rects (each id with its size, in order) in `out`.
pub(super) fn place(
    id: CardId,
    rect: Rect,
    premises: impl Iterator<Item = (NodeId, Vec2)>,
    p: &LayoutParams,
    frame: Frame,
    out: &mut Layout,
) {
    out.cards.insert(id, rect);
    out.card_labels.insert(
        id,
        Pos2::new(
            rect.right() - p.card_pad,
            rect.top() + p.card_label_height * 0.5,
        ),
    );
    let mut at = rect.min + Vec2::new(p.card_pad, p.card_label_height);
    for (pid, size) in premises {
        out.boxes.insert(pid, Rect::from_min_size(at, size));
        if frame.cross_is_y() {
            at.y += size.y + p.premise_gap;
        } else {
            at.x += size.x + p.premise_gap;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::LayoutDirection;

    fn params() -> LayoutParams {
        LayoutParams {
            card_bottom_extra: 5.0,
            ..LayoutParams::default()
        }
    }

    /// The frame's height is the label band, the tallest premise (or the
    /// stack), the bottom padding and `card_bottom_extra`.
    #[test]
    fn frame_height_names_every_part() {
        let p = params();
        let premises = [Vec2::new(100.0, 40.0), Vec2::new(80.0, 60.0)];
        let top_down = frame_size(&premises, &p, Frame::new(LayoutDirection::TopDown));
        let want = p.card_label_height + 60.0 + p.card_pad + p.card_bottom_extra;
        assert_eq!(top_down.y, want);
        let ltr = frame_size(&premises, &p, Frame::new(LayoutDirection::LeftRight));
        let stack = 40.0 + p.premise_gap + 60.0;
        assert_eq!(ltr.y, p.card_label_height + stack + p.card_pad + p.card_bottom_extra);
    }

    /// `premise_offsets` agrees with where `place` puts the premises, in
    /// every direction, whatever `card_bottom_extra` is.
    #[test]
    fn offsets_match_placed_premises() {
        let p = params();
        let sizes = [Vec2::new(100.0, 40.0), Vec2::new(80.0, 60.0)];
        for dir in [
            LayoutDirection::TopDown,
            LayoutDirection::BottomUp,
            LayoutDirection::LeftRight,
        ] {
            let frame = Frame::new(dir);
            let rect = Rect::from_min_size(Pos2::new(10.0, 20.0), frame_size(&sizes, &p, frame));
            let ids = [NodeId::new(), NodeId::new()];
            let doc = crate::Document::new("");
            let mut out = super::super::layout(&doc, &crate::fixtures::FixedMeasure, &p);
            let premises = ids.iter().copied().zip(sizes.iter().copied());
            place(CardId::new(), rect, premises, &p, frame, &mut out);
            let offsets = premise_offsets(&sizes, &p, frame);
            for (id, offset) in ids.iter().zip(offsets) {
                let placed = frame.cross_of(out.boxes[id].center());
                let want = frame.cross_of(rect.center()) + offset;
                assert!((placed - want).abs() < 1e-4, "{dir:?}: {placed} vs {want}");
            }
        }
    }
}
