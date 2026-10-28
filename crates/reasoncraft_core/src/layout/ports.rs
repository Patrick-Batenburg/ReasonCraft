//! Attach points and port spreading.
//!
//! Ports sit on the parent's far edge along the main axis (the bottom edge
//! top-down). Connectors that share an attach point are spread
//! `port_spacing` apart along the cross axis, ordered by the child's cross
//! position, within the parent's cross extent minus `2 * port_margin`.
//!
//! Whole-card ports go where [`whole_ports`] says, from the document alone
//! (so box sizing can count them): with an odd number of premises they
//! join the middle premise's group and are spread together with its ports,
//! each keeping its own main position (premise edge or frame edge); with an
//! even number they are spread about the gap between the two middle
//! premises, which grow to keep their ports clear of them (see `size`).
//!
//! Bottom-up, a card's label band lies beyond its premises' far edges, so
//! premise ports sit on the frame's far edge instead and lines stay clear
//! of the label.
//!
//! Boxes are sized so their ports fit `port_spacing` apart. Should an edge
//! still be too short, the ports are squeezed together about the edge
//! centre and the stubs fan out: each stub still ends at its port but
//! approaches at an angle (at most 45°, so its arrowhead stays out of the
//! parent), from where the port would be at full spacing.

use std::collections::HashMap;

use emath::{Pos2, Rect, Vec2};
use indexmap::IndexMap;

use crate::connector::{STUB_PX, Strength};

use super::axis::Frame;
use super::{Connector, ConnectorKind, Layout, LayoutParams, TargetKind};
use crate::ids::{CardId, Item, NodeId};
use crate::model::{Card, Document, LayoutDirection};

/// Where a card's whole-card ports go.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WholePorts {
    /// In the group of this (middle) premise.
    Premise(NodeId),
    /// About the gap between these two middle premises.
    Between(NodeId, NodeId),
    /// About the frame's centre (a card without premises).
    Frame,
}

/// Where `card`'s whole-card ports go, counting its premises in `doc`.
pub(super) fn whole_ports(doc: &Document, card: &Card) -> WholePorts {
    let premises: Vec<NodeId> = card
        .premises
        .iter()
        .copied()
        .filter(|p| doc.boxes.contains_key(p))
        .collect();
    let n = premises.len();
    match n {
        0 => WholePorts::Frame,
        _ if n % 2 == 1 => WholePorts::Premise(premises[n / 2]),
        _ => WholePorts::Between(premises[n / 2 - 1], premises[n / 2]),
    }
}

/// Where a child's connector attaches on its parent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum PortKey {
    /// The far edge of a box (a premise, or a box outside cards).
    Box(NodeId),
    /// The far edge of a card frame.
    Whole(CardId),
}

/// A connector whose parent port is not yet known.
pub(super) struct Pending {
    pub child: Item,
    pub from: Pos2,
    pub port: PortKey,
    pub kind: ConnectorKind,
    pub strength: Strength,
    pub nil: bool,
}

/// Turn pending connectors into connectors with spread ports, in the
/// original order. Connectors whose parent has no rect are dropped.
pub(super) fn assign(
    doc: &Document,
    pending: Vec<Pending>,
    out: &Layout,
    p: &LayoutParams,
    frame: Frame,
) -> Vec<Connector> {
    let mut groups: IndexMap<PortKey, Vec<usize>> = IndexMap::new();
    for (i, c) in pending.iter().enumerate() {
        groups.entry(c.port).or_default().push(i);
    }
    let whole_at = |card: CardId| doc.cards.get(&card).map(|c| whole_ports(doc, c));
    let merges: Vec<(PortKey, PortKey)> = groups
        .keys()
        .filter_map(|&key| match key {
            PortKey::Whole(card) => match whole_at(card)? {
                WholePorts::Premise(p) => Some((key, PortKey::Box(p))),
                _ => None,
            },
            PortKey::Box(_) => None,
        })
        .collect();
    // Each group spreads on its own, so their order does not matter:
    // `swap_remove` keeps this linear (`shift_remove` moved every later
    // group, quadratic in a map of many whole-card attacks).
    for (whole, premise) in merges {
        if let Some(moved) = groups.swap_remove(&whole) {
            groups.entry(premise).or_default().extend(moved);
        }
    }
    let rect_of = |key: PortKey| -> Option<&Rect> {
        match key {
            PortKey::Box(id) => out.boxes.get(&id),
            PortKey::Whole(id) => out.cards.get(&id),
        }
    };
    // The centre and width a group spreads in, along the cross axis.
    let span_of = |key: PortKey| -> Option<(f32, f32)> {
        let (lo, hi) = frame.cross_range(*rect_of(key)?);
        let between = match key {
            PortKey::Whole(card) => match whole_at(card)? {
                WholePorts::Between(a, b) => Some((a, b)),
                _ => None,
            },
            PortKey::Box(_) => None,
        };
        let Some((a, b)) = between else {
            return Some(((lo + hi) * 0.5, hi - lo));
        };
        let a = frame.cross_range(*out.boxes.get(&a)?).1;
        let b = frame.cross_range(*out.boxes.get(&b)?).0;
        let centre = (a + b) * 0.5;
        Some((centre, 2.0 * (centre - lo).min(hi - centre)))
    };
    let card_of: HashMap<NodeId, CardId> = if frame.dir == LayoutDirection::BottomUp {
        let premises = doc
            .cards
            .values()
            .flat_map(|c| c.premises.iter().map(|&p| (p, c.id)));
        premises.collect()
    } else {
        HashMap::new()
    };
    let edge_of = |key: PortKey| match key {
        PortKey::Box(id) => card_of.get(&id).map_or(key, |&c| PortKey::Whole(c)),
        PortKey::Whole(_) => key,
    };
    let toward = frame.dir.toward_parent();
    // The world direction of growing cross coordinates.
    let cross_axis = frame.point(1.0, 0.0) - frame.point(0.0, 0.0);
    let mut ports: Vec<Option<(Pos2, Vec2)>> = vec![None; pending.len()];
    for (key, mut members) in groups {
        let Some((centre, width)) = span_of(key) else {
            continue;
        };
        let cross_of = |i: usize| frame.cross_of(pending[i].from);
        members.sort_by(|&a, &b| cross_of(a).total_cmp(&cross_of(b)));
        let spots = spread(centre, width, members.len(), p);
        for (&i, (at, full)) in members.iter().zip(spots) {
            let edge = rect_of(edge_of(pending[i].port)).map(|r| frame.main_range(*r).1);
            // A stub STUB_PX long starts across from `full`, at most 45°
            // off the main axis.
            let slope = ((at - full) / STUB_PX).clamp(-1.0, 1.0);
            let approach = toward + cross_axis * slope;
            ports[i] = edge.map(|main| (frame.point(at, main), approach));
        }
    }
    pending
        .into_iter()
        .zip(ports)
        .filter_map(|(c, port)| {
            let (to, approach) = port?;
            Some(Connector {
                child: c.child,
                from: c.from,
                to,
                approach,
                kind: c.kind,
                strength: c.strength,
                nil: c.nil,
                dir: frame.dir,
                target: match c.port {
                    PortKey::Box(_) => TargetKind::Box,
                    PortKey::Whole(_) => TargetKind::WholeCard,
                },
            })
        })
        .collect()
}

/// Cross positions of `n` ports centred on `centre`, `port_spacing` apart
/// but squeezed to fit within `width - 2 * port_margin`, each with where it
/// would be at full spacing (the same unless squeezed).
fn spread(centre: f32, width: f32, n: usize, p: &LayoutParams) -> impl Iterator<Item = (f32, f32)> {
    let steps = n.saturating_sub(1) as f32;
    let room = (width - 2.0 * p.port_margin).max(0.0);
    let full = steps * p.port_spacing;
    let span = full.min(room);
    let step = if steps > 0.0 { span / steps } else { 0.0 };
    let (first, first_full) = (centre - span * 0.5, centre - full * 0.5);
    (0..n).map(move |i| {
        let i = i as f32;
        (first + i * step, first_full + i * p.port_spacing)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Squeezed ports keep their order about the centre, and each knows
    /// where it would be at full spacing.
    #[test]
    fn squeezed_spread_remembers_full_spacing() {
        let p = LayoutParams::default();
        let roomy: Vec<_> = spread(100.0, 200.0, 3, &p).collect();
        assert_eq!(roomy, vec![(84.0, 84.0), (100.0, 100.0), (116.0, 116.0)]);
        // 40 wide: 10 of room for 3 ports.
        let tight: Vec<_> = spread(100.0, 40.0, 3, &p).collect();
        assert_eq!(tight, vec![(95.0, 84.0), (100.0, 100.0), (105.0, 116.0)]);
    }
}
