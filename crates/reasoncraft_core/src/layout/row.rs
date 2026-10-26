//! Packing a parent's row of children along the cross axis.
//!
//! Without anchors the children pack in order against each other's
//! contours and the row is centred under the parent. A card's premises'
//! sources and notes are anchored: each premise's group packs as a row
//! centred on that premise (spec §5: sources sit directly under their
//! premise). Groups that would collide are pushed apart and then shifted
//! back together, so they stay as close to their premises as they can. The
//! card's other children then pack outward from the groups: the first half
//! to the left (before), the rest to the right (after).

use super::contour::Contour;

/// The packed row: each child's offset from the parent's centre, the
/// row's contour (in the parent's coordinates) and the cross range its
/// children's centres span.
pub(super) struct Row {
    pub offsets: Vec<f32>,
    pub contour: Contour,
    pub span: (f32, f32),
}

/// Pack children with contours `kids` (taken) in order. `anchors[i]` is
/// the offset child `i`'s group should be centred on (its premise's centre
/// relative to the card's), `None` for a free child. Anchored children of
/// one premise are consecutive.
pub(super) fn pack(anchors: &[Option<f32>], kids: Vec<Contour>, gap: f32) -> Row {
    let n = kids.len();
    let mut offsets = vec![0.0; n];
    let mut slots: Vec<Option<Contour>> = kids.into_iter().map(Some).collect();
    let mut take = |i: usize| slots[i].take().unwrap_or_default();

    let free: Vec<usize> = (0..n).filter(|&i| anchors[i].is_none()).collect();
    if free.len() == n {
        let all: Vec<Contour> = (0..n).map(&mut take).collect();
        let contour = in_order(&all, &mut offsets, gap);
        let half_span = offsets.last().copied().unwrap_or(0.0) * 0.5;
        for o in &mut offsets {
            *o -= half_span;
        }
        let mut row = Contour::default();
        row.merge(&contour, -half_span);
        return Row {
            offsets,
            contour: row,
            span: (-half_span, half_span),
        };
    }

    // Each premise's group, packed in order and centred on 0.
    let mut groups: Vec<(f32, Vec<usize>, Contour)> = Vec::new();
    let mut i = 0;
    while i < n {
        let Some(anchor) = anchors[i] else {
            i += 1;
            continue;
        };
        let members: Vec<usize> = (i..n).take_while(|&j| anchors[j] == Some(anchor)).collect();
        i += members.len();
        let contours: Vec<Contour> = members.iter().map(|&j| take(j)).collect();
        let mut local = vec![0.0; members.len()];
        let packed = in_order(&contours, &mut local, gap);
        let half_span = local.last().copied().unwrap_or(0.0) * 0.5;
        for (&j, o) in members.iter().zip(&local) {
            offsets[j] = o - half_span;
        }
        let mut centred = Contour::default();
        centred.merge(&packed, -half_span);
        groups.push((anchor, members, centred));
    }

    // Place the groups as close to their anchors as they can be: the
    // smallest largest distance from an anchor that leaves them clear.
    let place = |slack: f32| {
        let mut acc = Contour::default();
        let mut at = Vec::with_capacity(groups.len());
        for (k, (anchor, _, c)) in groups.iter().enumerate() {
            let mut shift = anchor - slack;
            if k > 0 {
                shift = shift.max(Contour::separation(&acc, c) + gap);
            }
            acc.merge(c, shift);
            at.push(shift);
        }
        let worst = groups
            .iter()
            .zip(&at)
            .map(|((a, ..), s)| s - a)
            .fold(0.0, f32::max);
        (at, worst <= slack + 1e-3)
    };
    let (mut lo, mut hi) = (
        0.0,
        place(0.0)
            .0
            .iter()
            .zip(&groups)
            .map(|(s, g)| s - g.0)
            .fold(0.0, f32::max),
    );
    for _ in 0..24 {
        let mid = (lo + hi) / 2.0;
        if place(mid).1 { hi = mid } else { lo = mid }
    }
    let mut row = Contour::default();
    for ((_, members, c), s) in groups.iter().zip(place(hi).0) {
        row.merge(c, s);
        for &j in members {
            offsets[j] += s;
        }
    }

    // The free children pack outward from the groups.
    let (before, after) = free.split_at(free.len() / 2);
    for &j in after {
        let c = take(j);
        offsets[j] = Contour::separation(&row, &c) + gap;
        row.merge(&c, offsets[j]);
    }
    for &j in before.iter().rev() {
        let c = take(j);
        offsets[j] = -(Contour::separation(&c, &row) + gap);
        row.merge(&c, offsets[j]);
    }
    let lo = offsets.iter().copied().fold(f32::INFINITY, f32::min);
    let hi = offsets.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    Row {
        offsets,
        contour: row,
        span: (lo, hi),
    }
}

/// Pack `kids` left to right, each `gap` clear of those before it; writes
/// each one's offset from the first and returns the merged contour.
fn in_order(kids: &[Contour], offsets: &mut [f32], gap: f32) -> Contour {
    let mut acc = Contour::default();
    for (k, (c, o)) in kids.iter().zip(offsets.iter_mut()).enumerate() {
        *o = if k == 0 {
            0.0
        } else {
            Contour::separation(&acc, c).max(0.0) + gap
        };
        acc.merge(c, *o);
    }
    acc
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boxes(n: usize, w: f32) -> Vec<Contour> {
        (0..n)
            .map(|_| Contour::rect(0.0, 10.0, -w / 2.0, w / 2.0))
            .collect()
    }

    #[test]
    fn free_row_is_centred() {
        let row = pack(&[None, None, None], boxes(3, 10.0), 2.0);
        assert_eq!(row.offsets, vec![-12.0, 0.0, 12.0]);
        assert_eq!(row.span, (-12.0, 12.0));
    }

    #[test]
    fn groups_sit_on_their_anchors_and_free_children_go_outside() {
        let a = Some(-50.0);
        let b = Some(50.0);
        let anchors = [a, a, None, b, None];
        let row = pack(&anchors, boxes(5, 10.0), 2.0);
        let o = &row.offsets;
        assert_eq!((o[0] + o[1]) / 2.0, -50.0);
        assert_eq!(o[3], 50.0);
        // One free child before the groups, one after.
        assert!(o[2] < o[0] - 10.0, "{o:?}");
        assert!(o[4] > o[3] + 10.0, "{o:?}");
    }

    #[test]
    fn colliding_groups_are_pushed_apart_evenly() {
        let anchors = [Some(-5.0), Some(5.0)];
        let row = pack(&anchors, boxes(2, 20.0), 2.0);
        let want = [-11.0, 11.0];
        for (o, w) in row.offsets.iter().zip(want) {
            assert!((o - w).abs() < 0.01, "{:?}", row.offsets);
        }
    }
}
