//! Editing operations on a [`Document`].

mod add;
mod fragment;
mod move_remove;
#[cfg(test)]
mod tests;
mod validate;
#[cfg(test)]
mod validate_tests;

use thiserror::Error;

pub use fragment::Fragment;

use crate::caps::capped;
use crate::ids::{Item, NodeId, Parent};
use crate::model::{BoxKind, CardKind, Document, MAX_BOX_CHARS, SourceType, now_rfc3339};

/// Why an edit was refused.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EditError {
    #[error("this item cannot be placed there")]
    NotAllowed,
    #[error("item not found")]
    NotFound,
    #[error("a claim's last premise cannot be removed")]
    LastPremiseOfRoot,
    #[error("this move would create a cycle")]
    Cycle,
}

/// A loose item to create in the parking area.
#[derive(Debug, Clone, PartialEq)]
pub enum NewLoose {
    Card(CardKind),
    Category,
    Source(SourceType),
    Note,
}

/// What happens to the items under a removed item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveMode {
    /// Delete the item and everything under it.
    Branch,
    /// Delete only the item; its children go to the parking area.
    KeepChildren,
}

impl Document {
    fn touch(&mut self) {
        self.meta.modified = now_rfc3339();
    }

    /// The end of every successful edit: stamp the modification time and,
    /// in debug builds, check the document's invariants.
    pub(crate) fn finish_edit(&mut self) {
        self.touch();
        #[cfg(debug_assertions)]
        self.check_invariants();
    }

    /// Insert `item` under `parent` at `index` (clamped); `None` appends.
    fn insert_into(
        &mut self,
        parent: Parent,
        item: Item,
        index: Option<usize>,
    ) -> Result<(), EditError> {
        fn put<T>(list: &mut Vec<T>, index: Option<usize>, value: T) {
            let at = index.map_or(list.len(), |i| i.min(list.len()));
            list.insert(at, value);
        }
        match (parent, item) {
            (Parent::Box(b), item) => {
                let node = self.boxes.get_mut(&b).ok_or(EditError::NotFound)?;
                put(&mut node.children, index, item);
            }
            (Parent::Card(c), Item::Card(id)) => {
                let card = self.cards.get_mut(&c).ok_or(EditError::NotFound)?;
                put(&mut card.children, index, id);
            }
            (Parent::Card(_), Item::Box(_)) => return Err(EditError::NotAllowed),
            (Parent::Parking, item) => put(&mut self.parking, index, item),
        }
        Ok(())
    }

    fn check_parent(&self, parent: Parent) -> Result<(), EditError> {
        let found = match parent {
            Parent::Box(b) => self.boxes.contains_key(&b),
            Parent::Card(c) => self.cards.contains_key(&c),
            Parent::Parking => true,
        };
        if found {
            Ok(())
        } else {
            Err(EditError::NotFound)
        }
    }

    /// Replace a box's text, cut to [`MAX_BOX_CHARS`] characters; true
    /// when it was cut.
    pub fn set_text(&mut self, id: NodeId, text: &str) -> Result<bool, EditError> {
        let node = self.boxes.get_mut(&id).ok_or(EditError::NotFound)?;
        let kept = capped(text, MAX_BOX_CHARS);
        kept.clone_into(&mut node.text);
        self.finish_edit();
        Ok(kept.len() < text.len())
    }

    /// Everything under `item` (not including it): premises, whole-card
    /// children and all that hangs below. Iterative, so deep chains are safe.
    pub fn descendants(&self, item: Item) -> Vec<Item> {
        // `seen` stops a damaged, cyclic structure from looping forever.
        let mut seen = std::collections::HashSet::from([item]);
        let mut out = Vec::new();
        let mut stack = vec![item];
        while let Some(cur) = stack.pop() {
            for child in self.child_items(cur) {
                if seen.insert(child) {
                    out.push(child);
                    stack.push(child);
                }
            }
        }
        out
    }

    /// Direct structural children, premises included.
    fn child_items(&self, item: Item) -> Vec<Item> {
        match item {
            Item::Box(b) => self
                .boxes
                .get(&b)
                .map(|n| n.children.clone())
                .unwrap_or_default(),
            Item::Card(c) => self.cards.get(&c).map_or_else(Vec::new, |card| {
                let premises = card.premises.iter().map(|p| Item::Box(*p));
                premises
                    .chain(card.children.iter().map(|k| Item::Card(*k)))
                    .collect()
            }),
        }
    }

    fn exists(&self, item: Item) -> bool {
        match item {
            Item::Box(b) => self.boxes.contains_key(&b),
            Item::Card(c) => self.cards.contains_key(&c),
        }
    }

    fn is_premise(&self, item: Item) -> bool {
        matches!(item, Item::Box(b) if self.boxes.get(&b).is_some_and(|n| n.kind == BoxKind::Premise))
    }

    /// Panic with the [`Document::validate`] message if the document is
    /// inconsistent. Debug builds run this after every edit.
    #[cfg(debug_assertions)]
    pub(crate) fn check_invariants(&self) {
        if let Err(e) = self.validate() {
            panic!("invariant violated: {e}");
        }
    }
}
