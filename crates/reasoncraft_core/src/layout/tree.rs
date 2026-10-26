//! Layout units, compact packing and placement.
//!
//! Units are collected breadth-first, so each unit's children are a
//! contiguous range and every child comes after its parent. Walking the list
//! backwards is then a post-order (packing); walking it forwards is a
//! pre-order (main positions and placement). No recursion, so chains of any
//! depth are safe.
//!
//! Packing uses main and cross axes ([`Frame`]), never world x and y:
//! siblings pack in order against the [`Contour`] of those before them.

use std::collections::HashSet;
use std::mem;
use std::ops::Range;

use emath::{Rect, Vec2};

use super::axis::Frame;
use super::card;
use super::contour::Contour;
use super::ports::{Pending, PortKey};
use super::row;
use super::{BOX_TEXT_PAD, ConnectorKind, Layout, LayoutParams, Measure, image_height};
use crate::connector::Strength;
use crate::ids::{Item, NodeId};
use crate::model::{BoxKind, BoxNode, Card, CardStrength, Document, Role};

/// A card, a box outside cards, or the root.
struct Unit {
    item: Item,
    /// Own world size (a card's frame size).
    size: Vec2,
    /// Main coordinate of the unit's near edge, relative to the tree root's.
    main: f32,
    /// Cross offset of the unit's centre from its parent's centre.
    offset: f32,
    /// Indices of the child units.
    children: Range<usize>,
    /// For a card: its premises' sizes, as a range into `Tree::premise_sizes`.
    premises: Range<usize>,
    /// Where the connector to the parent attaches; `None` for a tree root.
    port: Option<PortKey>,
    /// For a premise's source or note: the premise's cross offset from its
    /// card's centre, which the premise's group of these centres on.
    anchor: Option<f32>,
}

/// One mini-tree: the main map from the root, or a parking item.
pub(super) struct Tree {
    units: Vec<Unit>,
    premise_sizes: Vec<Vec2>,
    frame: Frame,
    /// Cross distance from the tree's lowest cross extent to the root centre.
    lead: f32,
}

impl Tree {
    /// Collect the units under `root` and pack them.
    pub(super) fn build(
        doc: &Document,
        root: Item,
        m: &dyn Measure,
        p: &LayoutParams,
        frame: Frame,
    ) -> Self {
        let mut tree = Tree {
            units: Vec::new(),
            premise_sizes: Vec::new(),
            frame,
            lead: 0.0,
        };
        let mut seen = HashSet::new();
        tree.push_unit(doc, root, None, m, p, &mut seen);
        let mut kids = Vec::new();
        let mut i = 0;
        while i < tree.units.len() {
            kids.clear();
            child_items(doc, tree.units[i].item, &mut kids);
            let start = tree.units.len();
            for &(child, port) in &kids {
                tree.push_unit(doc, child, Some(port), m, p, &mut seen);
            }
            tree.units[i].children = start..tree.units.len();
            tree.anchor_attachments(doc, i, p);
            let child_main = tree.units[i].main + frame.ext(tree.units[i].size).main + p.v_gap;
            for u in &mut tree.units[start..] {
                u.main = child_main;
            }
            i += 1;
        }
        let mut contours = vec![Contour::default(); tree.units.len()];
        for i in (0..tree.units.len()).rev() {
            contours[i] = tree.pack(i, &mut contours, p);
        }
        tree.lead = contours.first().map_or(0.0, |c| -c.cross_lo());
        tree
    }

    /// Pack unit `i`'s children (whose contours are in `contours`) in a row
    /// under it (see [`row::pack`]) and return the contour of its whole
    /// subtree, centred on the unit.
    fn pack(&mut self, i: usize, contours: &mut [Contour], p: &LayoutParams) -> Contour {
        let ext = self.frame.ext(self.units[i].size);
        let half = ext.cross * 0.5;
        let main_lo = self.units[i].main;
        let main_hi = main_lo + ext.main;
        let mut contour = Contour::rect(main_lo, main_hi, -half, half);
        let kids = self.units[i].children.clone();
        if kids.is_empty() {
            return contour;
        }
        let anchors: Vec<Option<f32>> = self.units[kids.clone()].iter().map(|u| u.anchor).collect();
        let kid_contours = contours[kids.clone()].iter_mut().map(mem::take).collect();
        let row = row::pack(&anchors, kid_contours, p.h_gap);
        for (u, o) in self.units[kids].iter_mut().zip(row.offsets) {
            u.offset = o;
        }
        // Keep the band the connectors cross clear of neighbouring subtrees.
        let (lo, hi) = (row.span.0.min(-half), row.span.1.max(half));
        let band = Contour::rect(main_hi, main_hi + p.v_gap, lo, hi);
        contour.merge(&band, 0.0);
        contour.merge(&row.contour, 0.0);
        contour
    }

    /// Anchor the sources and notes of card unit `i`'s premises on their
    /// premises.
    fn anchor_attachments(&mut self, doc: &Document, i: usize, p: &LayoutParams) {
        let Item::Card(id) = self.units[i].item else {
            return;
        };
        let sizes = &self.premise_sizes[self.units[i].premises.clone()];
        let offsets = card::premise_offsets(sizes, p, self.frame);
        let premises: Vec<NodeId> = (doc.cards[&id].premises.iter())
            .filter(|pid| doc.boxes.contains_key(*pid))
            .copied()
            .collect();
        for k in self.units[i].children.clone() {
            let u = &mut self.units[k];
            if let (Item::Box(_), Some(PortKey::Box(pid))) = (u.item, u.port) {
                let at = premises.iter().position(|&q| q == pid);
                u.anchor = at.map(|at| offsets[at]);
            }
        }
    }

    /// Add a unit for `item` unless it is missing or already placed.
    fn push_unit(
        &mut self,
        doc: &Document,
        item: Item,
        port: Option<PortKey>,
        m: &dyn Measure,
        p: &LayoutParams,
        seen: &mut HashSet<Item>,
    ) {
        if !seen.insert(item) {
            return;
        }
        let start = self.premise_sizes.len();
        let size = match item {
            Item::Box(id) => match doc.boxes.get(&id) {
                Some(node) => box_size(node, m, p),
                None => return,
            },
            Item::Card(id) => {
                let Some(card) = doc.cards.get(&id) else {
                    return;
                };
                let sizes = card.premises.iter().filter_map(|pid| doc.boxes.get(pid));
                self.premise_sizes
                    .extend(sizes.map(|node| box_size(node, m, p)));
                card::frame_size(&self.premise_sizes[start..], p, self.frame)
            }
        };
        self.units.push(Unit {
            item,
            size,
            main: 0.0,
            offset: 0.0,
            children: 0..0,
            premises: start..self.premise_sizes.len(),
            port,
            anchor: None,
        });
    }

    /// Place the tree with its lowest cross extent at `cross` and its root's
    /// near edge at `main`. Writes rects into `out`, queues connectors in
    /// `pending` and returns the world bounds of everything placed
    /// (`Rect::NOTHING` for an empty tree).
    pub(super) fn place(
        &self,
        doc: &Document,
        (cross, main): (f32, f32),
        p: &LayoutParams,
        out: &mut Layout,
        pending: &mut Vec<Pending>,
    ) -> Rect {
        let mut centre = vec![cross + self.lead; self.units.len()];
        let mut bounds = Rect::NOTHING;
        for (i, u) in self.units.iter().enumerate() {
            let ext = self.frame.ext(u.size);
            let near = main + u.main;
            let rect = self.frame.to_world(centre[i] - ext.cross * 0.5, near, ext);
            bounds = bounds.union(rect);
            self.emit(doc, u, rect, p, out);
            if let Some(port) = u.port {
                let (strength, nil) = connector_strength(out, u.item);
                pending.push(Pending {
                    child: u.item,
                    from: self.frame.point(centre[i], near),
                    port,
                    kind: connector_kind(doc, out, u.item),
                    strength,
                    nil,
                });
            }
            for k in u.children.clone() {
                centre[k] = centre[i] + self.units[k].offset;
            }
        }
        bounds
    }

    /// Record a placed unit's rects in `out`.
    fn emit(&self, doc: &Document, u: &Unit, rect: Rect, p: &LayoutParams, out: &mut Layout) {
        match u.item {
            Item::Box(id) => {
                out.boxes.insert(id, rect);
            }
            Item::Card(id) => {
                let premises = doc.cards[&id]
                    .premises
                    .iter()
                    .filter(|pid| doc.boxes.contains_key(*pid));
                let sizes = &self.premise_sizes[u.premises.clone()];
                let premises = premises.copied().zip(sizes.iter().copied());
                card::place(id, rect, premises, p, self.frame, out);
            }
        }
    }
}

/// The child items of a unit, in display order, with their attach points.
///
/// A box's children are its own, attachments first. A card's children are
/// each premise's children in turn, with the card's whole-card children
/// inserted at premise index `ceil(n / 2)` so they sit centred under it.
fn child_items(doc: &Document, item: Item, out: &mut Vec<(Item, PortKey)>) {
    match item {
        Item::Box(id) => box_children(doc, id, out),
        Item::Card(id) => {
            let Some(card) = doc.cards.get(&id) else {
                return;
            };
            let whole_at = card.premises.len().div_ceil(2);
            for (i, &pid) in card.premises.iter().enumerate() {
                if i == whole_at {
                    whole_children(card, out);
                }
                box_children(doc, pid, out);
            }
            if whole_at >= card.premises.len() {
                whole_children(card, out);
            }
        }
    }
}

fn whole_children(card: &Card, out: &mut Vec<(Item, PortKey)>) {
    out.extend(
        card.children
            .iter()
            .map(|&c| (Item::Card(c), PortKey::Whole(card.id))),
    );
}

/// A box's children with sources and notes first, otherwise in order. The
/// canvas and the outline share this order.
pub(crate) fn display_children(doc: &Document, id: NodeId) -> Vec<Item> {
    let Some(node) = doc.boxes.get(&id) else {
        return Vec::new();
    };
    let is_attachment = |item: &&Item| match item {
        Item::Box(b) => doc
            .boxes
            .get(b)
            .is_some_and(|n| matches!(n.kind, BoxKind::Source { .. } | BoxKind::Note)),
        Item::Card(_) => false,
    };
    let first = node.children.iter().filter(is_attachment);
    let rest = node.children.iter().filter(|i| !is_attachment(i));
    first.chain(rest).copied().collect()
}

fn box_children(doc: &Document, id: NodeId, out: &mut Vec<(Item, PortKey)>) {
    let port = PortKey::Box(id);
    out.extend(display_children(doc, id).into_iter().map(|i| (i, port)));
}

/// Size of a box from its measured text.
fn box_size(node: &BoxNode, m: &dyn Measure, p: &LayoutParams) -> Vec2 {
    let is_source = matches!(node.kind, BoxKind::Source { .. });
    let default_width = if is_source {
        p.source_width
    } else {
        p.box_width
    };
    let width = node.width.unwrap_or(default_width);
    let text_height = m.text_height(&node.text, width - 2.0 * BOX_TEXT_PAD, &node.kind);
    let image_height = node
        .image
        .as_ref()
        .map_or(0.0, |img| image_height(width, img));
    let height = if is_source {
        // Room for the icon and the source type line.
        text_height + 30.0 + image_height
    } else {
        p.min_box_height
            .max(text_height + 2.0 * BOX_TEXT_PAD + image_height)
    };
    Vec2::new(width, height)
}

/// How the connector from `child` to its parent is drawn.
fn connector_kind(doc: &Document, out: &Layout, child: Item) -> ConnectorKind {
    match child {
        Item::Card(id) => match out.card_roles.get(&id) {
            Some(Role::Supports) => ConnectorKind::Supports,
            Some(Role::Rebuts) => ConnectorKind::Rebuts,
            Some(Role::Opposes) | None => ConnectorKind::Opposes,
        },
        Item::Box(id) => match doc.boxes.get(&id).map(|n| &n.kind) {
            Some(BoxKind::Category) => ConnectorKind::Neutral,
            _ => ConnectorKind::Attachment,
        },
    }
}

/// Connector strength and nil flag from a card's rating.
fn connector_strength(out: &Layout, child: Item) -> (Strength, bool) {
    let rating = match child {
        Item::Card(id) => out.card_strength.get(&id).copied().flatten(),
        Item::Box(_) => None,
    };
    match rating {
        Some(CardStrength::Strong) => (Strength::Strong, false),
        Some(CardStrength::Weak) => (Strength::Weak, false),
        Some(CardStrength::Nil) => (Strength::Weak, true),
        None => (Strength::Normal, false),
    }
}

/// Fill `out.card_roles` and `out.card_strength` for every card.
pub(super) fn card_roles(doc: &Document, out: &mut Layout) {
    out.card_roles = crate::roles::card_roles(doc);
    for (&id, card) in &doc.cards {
        out.card_strength.insert(id, card.strength);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::{FixedMeasure, sample_iot_like};
    use crate::layout::layout;
    use crate::roles::owner_index;

    #[test]
    fn owner_index_matches_owner_of() {
        let doc = sample_iot_like(117);
        let idx = owner_index(&doc);
        for &id in doc.cards.keys() {
            assert_eq!(
                idx.get(&Item::Card(id)).copied(),
                doc.owner_of(Item::Card(id))
            );
        }
        for &id in doc.boxes.keys() {
            assert_eq!(
                idx.get(&Item::Box(id)).copied(),
                doc.owner_of(Item::Box(id))
            );
        }
    }

    #[test]
    fn index_derived_roles_equal_card_role() {
        let doc = sample_iot_like(117);
        let l = layout(&doc, &FixedMeasure, &LayoutParams::default());
        assert_eq!(l.card_roles.len(), doc.cards.len());
        for &id in doc.cards.keys() {
            assert_eq!(l.card_roles[&id], doc.card_role(id));
        }
    }
}
