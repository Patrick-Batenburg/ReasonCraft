//! How deep a map nests. Every edit that places an item keeps the map
//! within [`MAX_DEPTH`] levels, and opening a file or importing Argdown
//! moves what lies deeper to the parking area, so outlines, numbers and
//! layouts never grow with an unbounded depth (SEC-R4).

use std::collections::HashSet;

use super::{EditError, Fragment};
use crate::ids::{Item, Parent};
use crate::model::Document;

/// The deepest level an item may sit at, counted as the outline indents:
/// the claim and every parked item at 0, a box's children one level
/// below it, a card's premises at the card's own level and its whole-card
/// objections one level below.
pub const MAX_DEPTH: usize = 512;

/// The level `child` sits at under `parent` at level `at`: a premise
/// shares its card's level, everything else goes one level down.
fn level_below(parent: Item, child: Item, at: usize) -> usize {
    match (parent, child) {
        (Item::Card(_), Item::Box(_)) => at,
        _ => at + 1,
    }
}

/// How many levels the deepest item reachable from `top` through
/// `children` lies below `top` (0 for an item with nothing under it).
/// Walks with its own stack; an item met twice (a damaged, cyclic
/// structure) is walked once.
fn height_by(top: Item, children: impl Fn(Item) -> Vec<Item>) -> usize {
    let mut seen = HashSet::from([top]);
    let mut stack = vec![(top, 0)];
    let mut deepest = 0;
    while let Some((item, at)) = stack.pop() {
        deepest = deepest.max(at);
        for child in children(item) {
            if seen.insert(child) {
                stack.push((child, level_below(item, child, at)));
            }
        }
    }
    deepest
}

impl Document {
    /// The level `item` sits at (see [`MAX_DEPTH`]); `None` for an item
    /// that is not placed in the map.
    pub fn depth_of(&self, item: Item) -> Option<usize> {
        let mut seen = HashSet::new();
        let mut stack = vec![(Item::Box(self.root), 0)];
        stack.extend(self.parking.iter().map(|&i| (i, 0)));
        while let Some((cur, at)) = stack.pop() {
            if cur == item {
                return Some(at);
            }
            if seen.insert(cur) {
                for child in self.child_items(cur) {
                    stack.push((child, level_below(cur, child, at)));
                }
            }
        }
        None
    }

    /// The level `item` would sit at under `parent`.
    fn level_under(&self, parent: Parent, item: Item) -> Result<usize, EditError> {
        let owner = match parent {
            Parent::Parking => return Ok(0),
            Parent::Box(b) => Item::Box(b),
            Parent::Card(c) => Item::Card(c),
        };
        let at = self.depth_of(owner).ok_or(EditError::NotFound)?;
        Ok(level_below(owner, item, at))
    }

    /// Refuse with [`EditError::TooDeep`] when `item`, with `height`
    /// levels under it, would reach past [`MAX_DEPTH`] under `parent`.
    pub(crate) fn check_depth(
        &self,
        parent: Parent,
        item: Item,
        height: usize,
    ) -> Result<(), EditError> {
        if self.level_under(parent, item)? + height > MAX_DEPTH {
            return Err(EditError::TooDeep);
        }
        Ok(())
    }

    /// How many levels the deepest item under `item` lies below it.
    pub(crate) fn height(&self, item: Item) -> usize {
        height_by(item, |i| self.child_items(i))
    }

    /// Move every item that sits deeper than [`MAX_DEPTH`] to the end of
    /// the parking area, with everything under it (a branch deeper still
    /// is split again further down). Premises stay with their card, which
    /// is never deeper than they are. True when anything moved. Not an
    /// edit: the modified time is left alone.
    pub(crate) fn park_too_deep(&mut self) -> bool {
        let mut seen = HashSet::new();
        let mut stack = vec![(Item::Box(self.root), 0)];
        stack.extend(self.parking.iter().map(|&i| (i, 0)));
        let mut parked: Vec<(Item, Item)> = Vec::new();
        while let Some((cur, at)) = stack.pop() {
            if !seen.insert(cur) {
                continue;
            }
            for child in self.child_items(cur) {
                let level = level_below(cur, child, at);
                if level > MAX_DEPTH {
                    parked.push((cur, child));
                    stack.push((child, 0));
                } else {
                    stack.push((child, level));
                }
            }
        }
        for &(owner, item) in &parked {
            let owner = match owner {
                Item::Box(b) => Parent::Box(b),
                Item::Card(c) => Parent::Card(c),
            };
            self.detach(owner, item);
            self.parking.push(item);
        }
        !parked.is_empty()
    }
}

impl Fragment {
    /// How many levels the fragment's deepest item lies below its top
    /// (a copied premise counts from the card it is pasted in).
    pub fn height(&self) -> usize {
        height_by(self.top, |item| match item {
            Item::Box(b) => self
                .boxes
                .get(&b)
                .map_or_else(Vec::new, |n| n.children.clone()),
            Item::Card(c) => self.cards.get(&c).map_or_else(Vec::new, |card| {
                let premises = card.premises.iter().map(|p| Item::Box(*p));
                premises
                    .chain(card.children.iter().map(|k| Item::Card(*k)))
                    .collect()
            }),
        })
    }
}

#[cfg(test)]
#[path = "depth_tests.rs"]
mod tests;
