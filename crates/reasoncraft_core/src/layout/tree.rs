//! Layout units, subtree widths and top-down placement.
//!
//! Units are collected breadth-first, so each unit's children are a
//! contiguous range and every child comes after its parent. Walking the list
//! backwards is then a post-order (subtree widths); walking it forwards is a
//! pre-order (placement). No recursion, so chains of any depth are safe.

use std::collections::HashSet;
use std::ops::Range;

use emath::{Pos2, Rect, Vec2};

use super::ports::{Pending, PortKey};
use super::{BOX_TEXT_PAD, ConnectorKind, Layout, LayoutParams, Measure, image_height};
use crate::connector::Strength;
use crate::ids::{Item, NodeId};
use crate::model::{BoxKind, BoxNode, Card, CardStrength, Document, Role};

/// A card, a box outside cards, or the root.
struct Unit {
    item: Item,
    /// Own size (a card's frame size).
    size: Vec2,
    /// Width of the whole subtree below and including this unit.
    subtree_width: f32,
    /// Indices of the child units.
    children: Range<usize>,
    /// For a card: its premises' sizes, as a range into `Tree::premise_sizes`.
    premises: Range<usize>,
    /// Where the connector to the parent attaches; `None` for a tree root.
    port: Option<PortKey>,
}

/// One mini-tree: the main map from the root, or a parking item.
pub(super) struct Tree {
    units: Vec<Unit>,
    premise_sizes: Vec<Vec2>,
}

impl Tree {
    /// Collect the units under `root` and measure their subtree widths.
    pub(super) fn build(doc: &Document, root: Item, m: &dyn Measure, p: &LayoutParams) -> Self {
        let mut tree = Tree {
            units: Vec::new(),
            premise_sizes: Vec::new(),
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
            i += 1;
        }
        for i in (0..tree.units.len()).rev() {
            let row = row_width(&tree.units, tree.units[i].children.clone(), p.h_gap);
            tree.units[i].subtree_width = tree.units[i].size.x.max(row);
        }
        tree
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
                frame_size(&self.premise_sizes[start..], p)
            }
        };
        self.units.push(Unit {
            item,
            size,
            subtree_width: size.x,
            children: 0..0,
            premises: start..self.premise_sizes.len(),
            port,
        });
    }

    /// Place the tree with its root slot's top-left at `origin`. Writes
    /// rects into `out`, queues connectors in `pending` and returns the
    /// bounds of everything placed (`Rect::NOTHING` for an empty tree).
    pub(super) fn place(
        &self,
        doc: &Document,
        origin: Pos2,
        p: &LayoutParams,
        out: &mut Layout,
        pending: &mut Vec<Pending>,
    ) -> Rect {
        let n = self.units.len();
        let mut slot_left = vec![origin.x; n];
        let mut top = vec![origin.y; n];
        let mut bounds = Rect::NOTHING;
        for (i, u) in self.units.iter().enumerate() {
            let centre_x = slot_left[i] + u.subtree_width * 0.5;
            let rect = Rect::from_min_size(Pos2::new(centre_x - u.size.x * 0.5, top[i]), u.size);
            bounds = bounds.union(rect);
            self.emit(doc, u, rect, p, out);
            if let Some(port) = u.port {
                let (strength, nil) = connector_strength(out, u.item);
                pending.push(Pending {
                    child: u.item,
                    from: rect.center_top(),
                    port,
                    kind: connector_kind(doc, out, u.item),
                    strength,
                    nil,
                });
            }
            let mut x = centre_x - row_width(&self.units, u.children.clone(), p.h_gap) * 0.5;
            let child_top = rect.bottom() + p.v_gap;
            for k in u.children.clone() {
                slot_left[k] = x;
                top[k] = child_top;
                x += self.units[k].subtree_width + p.h_gap;
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
                out.cards.insert(id, rect);
                out.card_labels.insert(
                    id,
                    Pos2::new(
                        rect.right() - p.card_pad,
                        rect.top() + p.card_label_height * 0.5,
                    ),
                );
                let mut x = rect.left() + p.card_pad;
                let y = rect.top() + p.card_label_height;
                let premises = doc.cards[&id]
                    .premises
                    .iter()
                    .filter(|pid| doc.boxes.contains_key(*pid));
                for (pid, size) in premises.zip(&self.premise_sizes[u.premises.clone()]) {
                    out.boxes
                        .insert(*pid, Rect::from_min_size(Pos2::new(x, y), *size));
                    x += size.x + p.premise_gap;
                }
            }
        }
    }
}

/// Total width of a row of sibling subtrees, including the gaps between them.
fn row_width(units: &[Unit], row: Range<usize>, h_gap: f32) -> f32 {
    let n = row.len();
    if n == 0 {
        return 0.0;
    }
    units[row].iter().map(|u| u.subtree_width).sum::<f32>() + (n - 1) as f32 * h_gap
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

/// A card frame around premises of the given sizes, laid side by side.
fn frame_size(premises: &[Vec2], p: &LayoutParams) -> Vec2 {
    let n = premises.len();
    let widths: f32 = premises.iter().map(|s| s.x).sum();
    let gaps = n.saturating_sub(1) as f32 * p.premise_gap;
    let tallest = premises.iter().map(|s| s.y).fold(0.0, f32::max);
    Vec2::new(
        widths + gaps + 2.0 * p.card_pad,
        p.card_label_height + tallest + p.card_pad + 2.0,
    )
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
    out.card_roles = crate::model::card_roles(doc);
    for (&id, card) in &doc.cards {
        out.card_strength.insert(id, card.strength);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::{FixedMeasure, sample_iot_like};
    use crate::layout::layout;
    use crate::model::owner_index;

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
