//! The order in which checks visit items, and which items are parked.

use std::collections::HashSet;

use crate::ids::{Item, Parent};
use crate::layout::display_children;
use crate::model::{BoxKind, Document};

/// One item in check order, with where it hangs.
pub(crate) struct Visit {
    pub item: Item,
    pub parent: Option<Parent>,
    /// The item sits inside a top-level parking fragment.
    pub parked: bool,
}

/// Every existing item in check order: the main tree in pre-order (the
/// display order of the canvas and the outline), then the parking area.
/// Each item is listed once, even in a damaged, cyclic structure; an item
/// reachable from the root is never treated as parked.
pub(crate) fn visit_order(doc: &Document) -> Vec<Visit> {
    let mut out = Vec::with_capacity(doc.boxes.len() + doc.cards.len());
    let mut seen = HashSet::with_capacity(doc.boxes.len() + doc.cards.len());
    let tops = std::iter::once((Item::Box(doc.root), None, false)).chain(
        doc.parking
            .iter()
            .map(|&i| (i, Some(Parent::Parking), true)),
    );
    let mut stack: Vec<(Item, Option<Parent>, bool)> = Vec::new();
    for top in tops {
        stack.push(top);
        while let Some((item, parent, parked)) = stack.pop() {
            if !seen.insert(item) {
                continue;
            }
            let start = stack.len();
            match item {
                Item::Box(id) => {
                    if !doc.boxes.contains_key(&id) {
                        continue;
                    }
                    let me = Some(Parent::Box(id));
                    let children = display_children(doc, id).into_iter();
                    stack.extend(children.map(|c| (c, me, parked)));
                }
                Item::Card(id) => {
                    let Some(card) = doc.cards.get(&id) else {
                        continue;
                    };
                    let me = Some(Parent::Card(id));
                    let premises = card.premises.iter().map(|&p| (Item::Box(p), me, parked));
                    let whole = card.children.iter().map(|&c| (Item::Card(c), me, parked));
                    stack.extend(premises.chain(whole));
                }
            }
            stack[start..].reverse();
            out.push(Visit {
                item,
                parent,
                parked,
            });
        }
    }
    out
}

/// Whether `item` carries its rating; `None` for items that take no rating
/// (categories and notes).
pub(super) fn rated(doc: &Document, item: Item) -> Option<bool> {
    match item {
        Item::Box(id) => {
            let node = doc.boxes.get(&id)?;
            match node.kind {
                BoxKind::Claim | BoxKind::Premise => Some(node.acceptance.is_some()),
                BoxKind::Source { .. } => Some(node.source_rating.is_some()),
                BoxKind::Category | BoxKind::Note => None,
            }
        }
        Item::Card(id) => Some(doc.cards.get(&id)?.strength.is_some()),
    }
}
