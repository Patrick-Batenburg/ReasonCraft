//! Sources under their premise and uncrossed connectors (step 5, Task 5,
//! ruling T4-3).

use emath::{Pos2, Rect, pos2};
use reasoncraft_core::fixtures::{FixedMeasure, sample_iot_like};
use reasoncraft_core::layout::{Layout, LayoutParams, layout};
use reasoncraft_core::{BoxKind, Document, Item, LayoutDirection, NodeId, Parent, owner_index};

fn run_in(doc: &mut Document, dir: LayoutDirection) -> Layout {
    doc.layout_direction = dir;
    layout(doc, &FixedMeasure, &LayoutParams::default())
}

/// A world point in top-down terms: x along the cross axis, y along the
/// main axis away from the parent.
fn axes(dir: LayoutDirection, p: Pos2) -> Pos2 {
    match dir {
        LayoutDirection::TopDown => p,
        LayoutDirection::BottomUp => pos2(p.x, -p.y),
        LayoutDirection::LeftRight => pos2(p.y, p.x),
    }
}

fn axes_rect(dir: LayoutDirection, r: Rect) -> Rect {
    Rect::from_two_pos(axes(dir, r.min), axes(dir, r.max))
}

/// `id` and every box under it.
fn subtree(doc: &Document, id: NodeId) -> Vec<NodeId> {
    let mut out = vec![id];
    let mut i = 0;
    while i < out.len() {
        let kids = doc.boxes[&out[i]].children.iter();
        out.extend(kids.filter_map(|k| match k {
            Item::Box(b) => Some(*b),
            Item::Card(_) => None,
        }));
        i += 1;
    }
    out
}

/// Spec §5: a premise's sources and notes form a group centred on that
/// premise, to within half a pixel. A group may sit off its premise only
/// when something beside it on the side of its premise holds it there (a
/// neighbouring premise's group or the card's other children, which keep
/// the premises' order). That such a blocker could not itself move is
/// covered by the `layout::row` unit tests.
#[test]
fn sources_sit_under_their_premise() {
    let p = LayoutParams::default();
    let mut doc = sample_iot_like(300);
    let (mut centred, mut held) = (0, 0);
    for dir in LayoutDirection::ALL {
        let l = run_in(&mut doc, dir);
        let rect = |item: Item| {
            let r = match item {
                Item::Box(b) => l.boxes[&b],
                Item::Card(c) => l.cards[&c],
            };
            axes_rect(dir, r)
        };
        let all: Vec<Item> = (l.boxes.keys().map(|&b| Item::Box(b)))
            .chain(l.cards.keys().map(|&c| Item::Card(c)))
            .collect();
        for card in doc.cards.values() {
            for &pid in &card.premises {
                let firsts: Vec<NodeId> = (doc.boxes[&pid].children.iter())
                    .filter_map(|item| match item {
                        Item::Box(b) => {
                            matches!(doc.boxes[b].kind, BoxKind::Source { .. } | BoxKind::Note)
                                .then_some(*b)
                        }
                        Item::Card(_) => None,
                    })
                    .collect();
                if firsts.is_empty() {
                    continue;
                }
                // The group's centre: halfway between its outer members'
                // centres, which is what the layout centres on the premise.
                let centres = firsts.iter().map(|&b| rect(Item::Box(b)).center().x);
                let (lo, hi) = centres.fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), x| {
                    (lo.min(x), hi.max(x))
                });
                let premise = rect(Item::Box(pid));
                let centre = (lo + hi) / 2.0;
                if (centre - premise.center().x).abs() <= 0.5 {
                    centred += 1;
                    continue;
                }
                // Off its premise: something beside it, on the side of the
                // premise, must hold it there.
                let toward = (premise.center().x - centre).signum();
                let members: Vec<Item> = (firsts.iter())
                    .flat_map(|&b| subtree(&doc, b))
                    .map(Item::Box)
                    .collect();
                // Other items, and the bands their connectors cross.
                let others: Vec<Rect> = (all.iter())
                    .filter(|o| !members.contains(o))
                    .map(|&o| rect(o))
                    .chain(
                        (l.connectors.iter())
                            .filter(|c| !members.contains(&c.child))
                            .map(|c| Rect::from_two_pos(axes(dir, c.from), axes(dir, c.to))),
                    )
                    .collect();
                let blocked = members.iter().any(|&m| {
                    let m = rect(m);
                    others.iter().any(|&o| {
                        let near_main = m.y_range().expand(p.v_gap).intersects(o.y_range());
                        let gap = if toward > 0.0 {
                            o.min.x - m.max.x
                        } else {
                            m.min.x - o.max.x
                        };
                        near_main && (-0.5..=p.h_gap + 0.5).contains(&gap)
                    })
                });
                assert!(
                    blocked,
                    "{dir:?}: group centre {centre} off premise {:?} with nothing holding it",
                    premise.x_range()
                );
                held += 1;
            }
        }
    }
    assert!(centred >= 30, "only {centred} premises centred");
    assert!(held * 3 < centred, "{held} held off vs {centred} centred");
}

/// The connectors into one card (its frame or any of its premises) never
/// cross: ordered by their children along the cross axis, their ports are
/// in the same order. (Straight from-to segments are not used: a premise
/// port and a frame port sit at different heights, so two long, nearly flat
/// lines in the right order can still meet.)
#[test]
fn connectors_of_a_card_do_not_cross() {
    let mut doc = sample_iot_like(300);
    let owners = owner_index(&doc);
    for dir in LayoutDirection::ALL {
        let l = run_in(&mut doc, dir);
        let card_of = |child: Item| match owners.get(&child)? {
            Parent::Card(c) => Some(*c),
            Parent::Box(b) => doc
                .cards
                .values()
                .find(|k| k.premises.contains(b))
                .map(|k| k.id),
            Parent::Parking => None,
        };
        for card in doc.cards.keys() {
            let mut lines: Vec<_> = (l.connectors.iter())
                .filter(|c| card_of(c.child) == Some(*card))
                .map(|c| (axes(dir, c.from).x, axes(dir, c.to).x, c.child))
                .collect();
            lines.sort_by(|a, b| a.0.total_cmp(&b.0));
            for w in lines.windows(2) {
                assert!(
                    w[0].1 <= w[1].1 + 0.01,
                    "{dir:?}: connectors of {:?} and {:?} cross: {w:?}",
                    w[0].2,
                    w[1].2
                );
            }
        }
    }
}
