//! Packing a parent's row of children along the cross axis.
//!
//! Without attachments the children pack in order against each other's
//! contours and the row is centred under the parent.
//!
//! Under a card, each premise's sources and notes form a group centred on
//! that premise (spec §5: sources sit directly under their premise). The
//! card's other children keep their premises' order, so no two connectors
//! of the card cross. A card attacking a premise that has a group goes
//! next to that group, on the side toward the card's outside: before it
//! for a premise left of (above) the card's centre, after it for one
//! right of (below) it, and split in half around a centred premise. A
//! whole-card child, or one attacking a premise without a group, stays
//! where the display order puts it among the groups. Everything packs in
//! that order; groups that would collide are moved as little as possible
//! from their premises, and every group that does not have to move sits
//! exactly on its premise.

use super::contour::Contour;

/// What a child of the row is, for [`pack`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Kid {
    /// A child with no premise of its own (a whole-card child, or any
    /// child of a box).
    Free,
    /// A card attacking the premise at this cross offset from the card's
    /// centre.
    Targets(f32),
    /// A source or note of the premise at this offset.
    Attached(f32),
}

/// The packed row: each child's offset from the parent's centre, the
/// row's contour (in the parent's coordinates) and the cross range its
/// children's centres span.
pub(super) struct Row {
    pub offsets: Vec<f32>,
    pub contour: Contour,
    pub span: (f32, f32),
}

/// An entry of the row: one premise's group of attachments (with an
/// anchor), or a single other child.
struct Group {
    anchor: Option<f32>,
    /// Children and their offsets from the entry's centre.
    members: Vec<(usize, f32)>,
    contour: Contour,
}

/// Pack children with contours `contours` (taken) in display order.
/// `kids[i]` says what child `i` is; one premise's attachments are
/// consecutive.
pub(super) fn pack(kids: &[Kid], contours: Vec<Contour>, gap: f32) -> Row {
    let n = kids.len();
    let mut slots: Vec<Option<Contour>> = contours.into_iter().map(Some).collect();
    let mut take = |i: usize| slots[i].take().unwrap_or_default();
    let mut offsets = vec![0.0; n];
    if !kids.iter().any(|k| matches!(k, Kid::Attached(_))) {
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

    // Each premise's attachments, packed in order and centred on 0.
    let mut groups: Vec<Group> = Vec::new();
    let mut first_of = Vec::new();
    let mut i = 0;
    while i < n {
        let Kid::Attached(anchor) = kids[i] else {
            i += 1;
            continue;
        };
        let len = (i..n)
            .take_while(|&j| kids[j] == Kid::Attached(anchor))
            .count();
        let parts: Vec<Contour> = (i..i + len).map(&mut take).collect();
        let mut local = vec![0.0; len];
        let packed = in_order(&parts, &mut local, gap);
        let half_span = local.last().copied().unwrap_or(0.0) * 0.5;
        let mut contour = Contour::default();
        contour.merge(&packed, -half_span);
        let members = (i..i + len).zip(local.iter().map(|o| o - half_span));
        groups.push(Group {
            anchor: Some(anchor),
            members: members.collect(),
            contour,
        });
        first_of.push(i);
        i += len;
    }

    // The other children: slot 0 comes before the first group, slot k + 1
    // rides after group k.
    let mut slot_of = vec![0; n];
    for (j, kid) in kids.iter().enumerate() {
        slot_of[j] = match *kid {
            Kid::Attached(_) => continue,
            Kid::Targets(p) => match groups.iter().position(|g| g.anchor == Some(p)) {
                // Toward the outside of the card; the middle premise's
                // attackers split around its group.
                Some(k) if p < -0.5 => k,
                Some(k) if p > 0.5 => k + 1,
                Some(k) => {
                    let mine: Vec<usize> = (0..n).filter(|&x| kids[x] == *kid).collect();
                    let r = mine.iter().position(|&x| x == j).unwrap_or(0);
                    if r < mine.len() / 2 { k } else { k + 1 }
                }
                None => first_of.iter().filter(|&&f| f < j).count(),
            },
            Kid::Free => first_of.iter().filter(|&&f| f < j).count(),
        };
    }
    let slot_of = &slot_of;
    let in_slot = |s: usize| (0..n).filter(move |&j| !is_attached(kids[j]) && slot_of[j] == s);
    // In order: each group, then the children riding after it.
    let mut seq: Vec<Group> = Vec::with_capacity(n);
    for (k, g) in groups.into_iter().enumerate() {
        seq.push(g);
        for j in in_slot(k + 1) {
            seq.push(Group {
                anchor: None,
                members: vec![(j, 0.0)],
                contour: take(j),
            });
        }
    }

    let at = place(&seq, gap);
    let mut row = Contour::default();
    for (g, s) in seq.iter().zip(&at) {
        row.merge(&g.contour, *s);
        for &(j, o) in &g.members {
            offsets[j] = s + o;
        }
    }
    let leading: Vec<usize> = in_slot(0).collect();
    for &j in leading.iter().rev() {
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

fn is_attached(kid: Kid) -> bool {
    matches!(kid, Kid::Attached(_))
}

/// Each entry's offset, packed left to right in order, with no group
/// before its anchor by more than a slack and the slack, the largest
/// distance of a group from its anchor, as small as it can be. Then every
/// group left of its anchor is pulled back toward it, together with the
/// children riding after it, as far as the entries after them allow, so a
/// group that need not move sits on its anchor.
fn place(seq: &[Group], gap: f32) -> Vec<f32> {
    let greedy = |slack: f32| {
        let mut acc = Contour::default();
        let mut at = Vec::with_capacity(seq.len());
        for (k, g) in seq.iter().enumerate() {
            let mut shift = g.anchor.map_or(f32::NEG_INFINITY, |a| a - slack);
            if k > 0 {
                shift = shift.max(Contour::separation(&acc, &g.contour) + gap);
            }
            acc.merge(&g.contour, shift);
            at.push(shift);
        }
        at
    };
    let worst = |at: &[f32]| {
        (at.iter().zip(seq))
            .filter_map(|(s, g)| Some(s - g.anchor?))
            .fold(0.0, f32::max)
    };
    let (mut lo, mut hi) = (0.0, worst(&greedy(0.0)));
    for _ in 0..24 {
        let mid = (lo + hi) / 2.0;
        if worst(&greedy(mid)) <= mid + 1e-3 {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    let mut at = greedy(hi);
    // Pull each group, with the children riding after it up to the next
    // group, back toward its anchor as far as the entries after allow.
    for k in (0..seq.len()).rev() {
        let Some(anchor) = seq[k].anchor.filter(|&a| at[k] < a) else {
            continue;
        };
        let end = (k + 1..seq.len())
            .find(|&j| seq[j].anchor.is_some())
            .unwrap_or(seq.len());
        let room = (k..end)
            .flat_map(|e| (end..seq.len()).map(move |j| (e, j)))
            .map(|(e, j)| {
                at[j] - at[e] - Contour::separation(&seq[e].contour, &seq[j].contour) - gap
            })
            .fold(f32::INFINITY, f32::min);
        let delta = (anchor - at[k]).min(room);
        if delta > 0.0 {
            for a in &mut at[k..end] {
                *a += delta;
            }
        }
    }
    at
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

    fn close(got: &[f32], want: &[f32]) {
        for (o, w) in got.iter().zip(want) {
            assert!((o - w).abs() < 0.01, "{got:?} != {want:?}");
        }
    }

    #[test]
    fn free_row_is_centred() {
        let row = pack(&[Kid::Free; 3], boxes(3, 10.0), 2.0);
        assert_eq!(row.offsets, vec![-12.0, 0.0, 12.0]);
        assert_eq!(row.span, (-12.0, 12.0));
    }

    #[test]
    fn groups_sit_on_their_anchors_and_attackers_beside_them() {
        let (a, b) = (Kid::Attached(-50.0), Kid::Attached(50.0));
        let (ta, tb) = (Kid::Targets(-50.0), Kid::Targets(50.0));
        let kids = [a, a, ta, ta, b, tb];
        let row = pack(&kids, boxes(6, 10.0), 2.0);
        let o = &row.offsets;
        assert_eq!((o[0] + o[1]) / 2.0, -50.0);
        assert_eq!(o[4], 50.0);
        // A is left of centre: its attackers before its group. B is right
        // of centre: its attacker after its group.
        assert!(o[3] < o[0] - 10.0 && o[2] < o[3], "{o:?}");
        assert!(o[5] > o[4] + 10.0, "{o:?}");
    }

    /// A centred premise's attackers split around its group.
    #[test]
    fn centred_premise_attackers_split() {
        let kids = [Kid::Attached(0.0), Kid::Targets(0.0), Kid::Targets(0.0)];
        let row = pack(&kids, boxes(3, 10.0), 2.0);
        assert_eq!(row.offsets, vec![0.0, -12.0, 12.0]);
    }

    #[test]
    fn colliding_groups_are_pushed_apart_evenly() {
        let kids = [Kid::Attached(-5.0), Kid::Attached(5.0)];
        let row = pack(&kids, boxes(2, 20.0), 2.0);
        close(&row.offsets, &[-11.0, 11.0]);
    }

    /// Premises at -170, 0 and 300: A's three wide sources push B's, but
    /// C's single source does not have to move and stays exactly on C.
    #[test]
    fn a_group_that_need_not_move_stays_on_its_anchor() {
        let (a, b, c) = (
            Kid::Attached(-170.0),
            Kid::Attached(0.0),
            Kid::Attached(300.0),
        );
        let row = pack(&[a, a, a, b, c], boxes(5, 128.0), 18.0);
        let o = &row.offsets;
        assert_eq!(o[4], 300.0, "{o:?}");
        let a_centre = o[1];
        assert!(a_centre < -170.0 && o[3] > 0.0, "A and B both move: {o:?}");
        assert!(o[3] + 64.0 + 18.0 <= o[4] - 64.0 + 0.01, "B clear of C");
    }

    /// The same with an attacker of C riding after C's group: C's group
    /// and its attacker are pulled back together, C exactly on its anchor.
    #[test]
    fn a_group_with_a_rider_still_reaches_its_anchor() {
        let (a, b, c) = (
            Kid::Attached(-170.0),
            Kid::Attached(0.0),
            Kid::Attached(300.0),
        );
        let kids = [a, a, a, b, c, Kid::Targets(300.0)];
        let row = pack(&kids, boxes(6, 128.0), 18.0);
        let o = &row.offsets;
        assert_eq!(o[4], 300.0, "{o:?}");
        assert_eq!(o[5], 300.0 + 128.0 + 18.0, "the attacker rides after C");
    }

    /// A card attacking the middle premise stays between the groups of the
    /// outer premises, pushing the right one along.
    #[test]
    fn attackers_keep_premise_order() {
        let (a, c) = (Kid::Attached(-100.0), Kid::Attached(100.0));
        let kids = [a, Kid::Targets(0.0), c];
        let row = pack(&kids, boxes(3, 60.0), 10.0);
        let o = &row.offsets;
        assert!(o[0] < o[1] && o[1] < o[2], "{o:?}");
    }
}
