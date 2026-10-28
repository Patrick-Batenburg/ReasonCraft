//! Copying a part of a map and pasting it elsewhere, in the same document
//! or another: an item with everything under it, given fresh ids when it
//! is pasted.

use std::collections::HashMap;

use indexmap::IndexMap;
use uuid::Uuid;

use super::EditError;
use crate::ids::{CardId, Item, NodeId, Parent};
use crate::model::{BoxKind, BoxNode, Card, CardKind, Document};
use crate::rules::{NewItem, can_attach};

/// A copied item and everything under it, with the ids it had where it
/// was copied. Pasting gives every box, card and comment a new id, so one
/// fragment can be pasted any number of times.
#[derive(Debug, Clone, PartialEq)]
pub struct Fragment {
    /// The copied item: a card, or a box other than the root claim.
    pub top: Item,
    /// For a copied premise, the kind of the card it was in: pasted, it
    /// becomes the only premise of a new card of that kind.
    pub premise_of: Option<CardKind>,
    pub boxes: IndexMap<NodeId, BoxNode>,
    pub cards: IndexMap<CardId, Card>,
}

/// The deepest indent of the clipboard text, in levels.
pub const MAX_INDENT: usize = 32;

impl Fragment {
    /// What the pasted top item is for the structure rules.
    pub fn new_item(&self) -> NewItem {
        if let Some(kind) = &self.premise_of {
            return NewItem::Card(kind.clone());
        }
        match self.top {
            Item::Card(c) => NewItem::Card(self.cards[&c].kind.clone()),
            Item::Box(b) => match &self.boxes[&b].kind {
                BoxKind::Category => NewItem::Category,
                BoxKind::Source { .. } => NewItem::Source,
                // Claims are never copied, premises are handled above.
                BoxKind::Note | BoxKind::Claim | BoxKind::Premise => NewItem::Note,
            },
        }
    }

    /// The image assets the fragment's boxes show.
    pub fn assets(&self) -> impl Iterator<Item = &str> {
        self.boxes
            .values()
            .filter_map(|b| b.image.as_ref())
            .map(|i| i.asset.as_str())
    }

    /// The fragment's text, one box per line, indented two spaces a level
    /// (cards add no level) up to [`MAX_INDENT`] levels, in the order the
    /// map shows them: what goes on the system clipboard for other
    /// programs. It walks with its own stack, so any depth fits.
    pub fn plain_text(&self) -> String {
        let mut out = String::new();
        let mut stack = vec![(self.top, 0)];
        // Bounded by the item count, so even a damaged cycle ends.
        let mut budget = self.boxes.len() + self.cards.len();
        while let Some((item, depth)) = stack.pop() {
            if budget == 0 {
                break;
            }
            budget -= 1;
            // Pushed last to first, so they come off in order.
            match item {
                Item::Card(c) => {
                    let Some(card) = self.cards.get(&c) else {
                        continue;
                    };
                    let kids = card.children.iter().map(|&k| (Item::Card(k), depth + 1));
                    let premises = card.premises.iter().map(|&p| (Item::Box(p), depth));
                    let order: Vec<_> = premises.chain(kids).collect();
                    stack.extend(order.into_iter().rev());
                }
                Item::Box(b) => {
                    let Some(node) = self.boxes.get(&b) else {
                        continue;
                    };
                    let indent = "  ".repeat(depth.min(MAX_INDENT));
                    for line in node.text.lines() {
                        out.push_str(&indent);
                        out.push_str(line);
                        out.push('\n');
                    }
                    let first = node.children.iter().filter(|i| self.is_attachment(**i));
                    let rest = node.children.iter().filter(|i| !self.is_attachment(**i));
                    let order: Vec<_> = first.chain(rest).map(|&i| (i, depth + 1)).collect();
                    stack.extend(order.into_iter().rev());
                }
            }
        }
        out
    }

    fn is_attachment(&self, item: Item) -> bool {
        matches!(item, Item::Box(b) if self.boxes.get(&b).is_some_and(|n| n.kind.is_attachment()))
    }
}

impl Document {
    /// A copy of `item` and everything under it; `None` for the root claim
    /// (the whole map) and for an item that does not exist.
    pub fn copy_fragment(&self, item: Item) -> Option<Fragment> {
        if item == Item::Box(self.root) {
            return None;
        }
        let premise_of = match item {
            Item::Box(b) if self.boxes.get(&b)?.kind == BoxKind::Premise => {
                let Some(Parent::Card(c)) = self.owner_of(item) else {
                    return None;
                };
                Some(self.cards.get(&c)?.kind.clone())
            }
            Item::Box(b) => {
                self.boxes.get(&b)?;
                None
            }
            Item::Card(c) => {
                self.cards.get(&c)?;
                None
            }
        };
        let mut fragment = Fragment {
            top: item,
            premise_of,
            boxes: IndexMap::new(),
            cards: IndexMap::new(),
        };
        let mut stack = vec![item];
        // Bounded by the item count, so even a damaged cycle ends.
        let mut budget = self.boxes.len() + self.cards.len();
        while let Some(next) = stack.pop() {
            if budget == 0 {
                break;
            }
            budget -= 1;
            match next {
                Item::Box(b) => {
                    let Some(node) = self.boxes.get(&b) else {
                        continue;
                    };
                    if fragment.boxes.insert(b, node.clone()).is_none() {
                        stack.extend(node.children.iter().copied());
                    }
                }
                Item::Card(c) => {
                    let Some(card) = self.cards.get(&c) else {
                        continue;
                    };
                    if fragment.cards.insert(c, card.clone()).is_none() {
                        stack.extend(card.premises.iter().map(|&p| Item::Box(p)));
                        stack.extend(card.children.iter().map(|&k| Item::Card(k)));
                    }
                }
            }
        }
        Some(fragment)
    }

    /// Paste a fresh copy of `fragment` at the end of `parent`'s children
    /// and return its new top item (for a copied premise, the new card
    /// holding it). Refused with [`EditError::NotAllowed`] where the
    /// structure rules do not let it go.
    pub fn paste_fragment(
        &mut self,
        fragment: &Fragment,
        parent: Parent,
    ) -> Result<Item, EditError> {
        self.check_parent(parent)?;
        if !can_attach(self, parent, fragment.new_item()) {
            return Err(EditError::NotAllowed);
        }
        let nodes: HashMap<NodeId, NodeId> =
            fragment.boxes.keys().map(|&b| (b, NodeId::new())).collect();
        let cards: HashMap<CardId, CardId> =
            fragment.cards.keys().map(|&c| (c, CardId::new())).collect();
        let map_item = |item: Item| match item {
            Item::Box(b) => nodes.get(&b).map(|&n| Item::Box(n)),
            Item::Card(c) => cards.get(&c).map(|&n| Item::Card(n)),
        };
        for (old, node) in &fragment.boxes {
            let mut node = node.clone();
            node.id = nodes[old];
            node.children = node.children.iter().filter_map(|&i| map_item(i)).collect();
            for comment in &mut node.comments {
                comment.id = Uuid::new_v4();
            }
            self.boxes.insert(node.id, node);
        }
        for (old, card) in &fragment.cards {
            let mut card = card.clone();
            card.id = cards[old];
            card.premises = card
                .premises
                .iter()
                .filter_map(|p| nodes.get(p).copied())
                .collect();
            card.children = card
                .children
                .iter()
                .filter_map(|k| cards.get(k).copied())
                .collect();
            self.cards.insert(card.id, card);
        }
        let top = map_item(fragment.top).ok_or(EditError::NotFound)?;
        let top = match (&fragment.premise_of, top) {
            (Some(kind), Item::Box(p)) => {
                let card = CardId::new();
                self.cards
                    .insert(card, Card::new(card, kind.clone(), vec![p]));
                Item::Card(card)
            }
            _ => top,
        };
        self.insert_into(parent, top, None)?;
        self.finish_edit();
        Ok(top)
    }
}

#[cfg(test)]
mod tests;
