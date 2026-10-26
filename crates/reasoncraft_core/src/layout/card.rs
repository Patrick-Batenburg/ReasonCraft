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
        p.card_label_height + height + p.card_pad + 2.0,
    )
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
