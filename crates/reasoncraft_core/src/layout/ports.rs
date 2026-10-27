//! Attach points and port spreading.
//!
//! Ports sit on the parent's far edge along the main axis (the bottom edge
//! top-down). Connectors that share an attach point are spread
//! `port_spacing` apart along the cross axis, ordered by the child's cross
//! position, within the parent's cross extent minus `2 * port_margin`.
//!
//! A card frame's far-edge centre usually lies straight beyond one of its
//! premises (always for a single premise). Whole-card ports there would land
//! on the same cross position as that premise's ports, so they join the
//! premise's group and are spread together; each port keeps its own main
//! position (premise edge or frame edge).
//!
//! Bottom-up, a card's label band lies beyond its premises' far edges, so
//! premise ports sit on the frame's far edge instead and lines stay clear
//! of the label.
//!
//! When an edge is too short for its ports `port_spacing` apart, the ports
//! are squeezed together about the edge centre and the stubs fan out: each
//! stub still ends at its port but approaches at an angle, from where the
//! port would be at full spacing, so the arrowheads spread apart.

use std::collections::HashMap;

use emath::{Pos2, Rect, Vec2};
use indexmap::IndexMap;

use crate::connector::{STUB_PX, Strength};

use super::axis::Frame;
use super::{Connector, ConnectorKind, Layout, LayoutParams, TargetKind};
use crate::ids::{CardId, Item, NodeId};
use crate::model::{Document, LayoutDirection};

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
    let merges: Vec<(PortKey, PortKey)> = groups
        .keys()
        .filter_map(|&key| {
            let PortKey::Whole(card) = key else {
                return None;
            };
            let premise = PortKey::Box(premise_above_centre(doc, out, card, frame)?);
            groups.contains_key(&premise).then_some((key, premise))
        })
        .collect();
    for (whole, premise) in merges {
        if let Some(moved) = groups.shift_remove(&whole) {
            groups[&premise].extend(moved);
        }
    }
    let rect_of = |key: PortKey| -> Option<&Rect> {
        match key {
            PortKey::Box(id) => out.boxes.get(&id),
            PortKey::Whole(id) => out.cards.get(&id),
        }
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
        let Some(parent) = rect_of(key) else {
            continue;
        };
        let cross_of = |i: usize| frame.cross_of(pending[i].from);
        members.sort_by(|&a, &b| cross_of(a).total_cmp(&cross_of(b)));
        let (lo, hi) = frame.cross_range(*parent);
        let spots = spread((lo + hi) * 0.5, hi - lo, members.len(), p);
        for (&i, (at, full)) in members.iter().zip(spots) {
            let edge = rect_of(edge_of(pending[i].port)).map(|r| frame.main_range(*r).1);
            // A stub STUB_PX long starts across from `full`.
            let approach = toward + cross_axis * ((at - full) / STUB_PX);
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

/// The premise of `card` whose rect spans the frame's cross centre, if any.
fn premise_above_centre(
    doc: &Document,
    out: &Layout,
    card: CardId,
    frame: Frame,
) -> Option<NodeId> {
    let (lo, hi) = frame.cross_range(*out.cards.get(&card)?);
    let centre = (lo + hi) * 0.5;
    doc.cards.get(&card)?.premises.iter().copied().find(|id| {
        out.boxes.get(id).is_some_and(|r| {
            let (lo, hi) = frame.cross_range(*r);
            lo <= centre && centre <= hi
        })
    })
}
