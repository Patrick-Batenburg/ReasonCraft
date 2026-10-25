//! Attach points and port spreading.
//!
//! Connectors that share an attach point are spread `port_spacing` apart,
//! ordered by child centre x, within the parent width minus
//! `2 * port_margin`.

use emath::{Pos2, Rect};
use indexmap::IndexMap;

use super::{Connector, ConnectorKind, Layout, LayoutParams, TargetKind};
use crate::ids::{CardId, Item, NodeId};

/// Where a child's connector attaches on its parent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum PortKey {
    /// The bottom centre of a box (a premise, or a box outside cards).
    Box(NodeId),
    /// The bottom centre of a card frame.
    Whole(CardId),
}

/// A connector whose parent port is not yet known.
pub(super) struct Pending {
    pub child: Item,
    pub from: Pos2,
    pub port: PortKey,
    pub kind: ConnectorKind,
}

/// Turn pending connectors into connectors with spread ports, in the
/// original order. Connectors whose parent has no rect are dropped.
pub(super) fn assign(pending: Vec<Pending>, out: &Layout, p: &LayoutParams) -> Vec<Connector> {
    let mut groups: IndexMap<PortKey, Vec<usize>> = IndexMap::new();
    for (i, c) in pending.iter().enumerate() {
        groups.entry(c.port).or_default().push(i);
    }
    let mut ports: Vec<Option<Pos2>> = vec![None; pending.len()];
    for (key, mut members) in groups {
        let parent: Option<&Rect> = match key {
            PortKey::Box(id) => out.boxes.get(&id),
            PortKey::Whole(id) => out.cards.get(&id),
        };
        let Some(parent) = parent else {
            continue;
        };
        members.sort_by(|&a, &b| pending[a].from.x.total_cmp(&pending[b].from.x));
        let xs = spread(parent.center().x, parent.width(), members.len(), p);
        for (&i, x) in members.iter().zip(xs) {
            ports[i] = Some(Pos2::new(x, parent.bottom()));
        }
    }
    pending
        .into_iter()
        .zip(ports)
        .filter_map(|(c, to)| {
            Some(Connector {
                child: c.child,
                from: c.from,
                to: to?,
                kind: c.kind,
                target: match c.port {
                    PortKey::Box(_) => TargetKind::Box,
                    PortKey::Whole(_) => TargetKind::WholeCard,
                },
            })
        })
        .collect()
}

/// X positions of `n` ports centred on `centre`, `port_spacing` apart but
/// squeezed to fit within `width - 2 * port_margin`.
fn spread(centre: f32, width: f32, n: usize, p: &LayoutParams) -> impl Iterator<Item = f32> {
    let steps = n.saturating_sub(1) as f32;
    let room = (width - 2.0 * p.port_margin).max(0.0);
    let span = (steps * p.port_spacing).min(room);
    let step = if steps > 0.0 { span / steps } else { 0.0 };
    let first = centre - span * 0.5;
    (0..n).map(move |i| first + i as f32 * step)
}
