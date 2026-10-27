//! Moving and removing items.

use crate::ids::{Item, Parent};
use crate::model::{BoxKind, Document};
use crate::rules::{NewItem, can_attach};

use super::{EditError, RemoveMode};

impl Document {
    /// Take `item` out of the list of `owner` (not out of the maps).
    fn detach(&mut self, owner: Parent, item: Item) {
        match owner {
            Parent::Box(b) => {
                if let Some(n) = self.boxes.get_mut(&b) {
                    n.children.retain(|i| *i != item);
                }
            }
            Parent::Card(c) => {
                if let Some(card) = self.cards.get_mut(&c) {
                    match item {
                        Item::Card(id) => card.children.retain(|k| *k != id),
                        Item::Box(id) => card.premises.retain(|p| *p != id),
                    }
                }
            }
            Parent::Parking => self.parking.retain(|i| *i != item),
        }
    }

    /// Move `item` under `to` at `index` (`None` appends; the index counts
    /// siblings after `item` has been taken out, so the same parent
    /// reorders). Covers connecting a parked item, detaching to the parking
    /// area and reordering. A premise only moves to another card, where it
    /// becomes a co-premise; a card left without premises is removed and its
    /// whole-card children are parked.
    pub fn move_item(
        &mut self,
        item: Item,
        to: Parent,
        index: Option<usize>,
    ) -> Result<(), EditError> {
        if !self.exists(item) {
            return Err(EditError::NotFound);
        }
        if item == Item::Box(self.root) {
            return Err(EditError::NotAllowed);
        }
        self.check_parent(to)?;
        let owner = self.owner_of(item).ok_or(EditError::NotFound)?;
        if self.is_premise(item) {
            self.move_premise(item, owner, to, index)?;
        } else {
            let new = match item {
                Item::Card(c) => NewItem::Card(self.cards[&c].kind.clone()),
                Item::Box(b) => match self.boxes[&b].kind {
                    BoxKind::Category => NewItem::Category,
                    BoxKind::Source { .. } => NewItem::Source,
                    BoxKind::Note => NewItem::Note,
                    BoxKind::Claim | BoxKind::Premise => return Err(EditError::NotAllowed),
                },
            };
            let target = match to {
                Parent::Box(b) => Some(Item::Box(b)),
                Parent::Card(c) => Some(Item::Card(c)),
                Parent::Parking => None,
            };
            if let Some(t) = target
                && (t == item || self.descendants(item).contains(&t))
            {
                return Err(EditError::Cycle);
            }
            if owner != to && !can_attach(self, to, new) {
                return Err(EditError::NotAllowed);
            }
            self.detach(owner, item);
            self.insert_into(to, item, index)?;
        }
        self.touch();
        #[cfg(debug_assertions)]
        self.check_invariants();
        Ok(())
    }

    fn move_premise(
        &mut self,
        item: Item,
        owner: Parent,
        to: Parent,
        index: Option<usize>,
    ) -> Result<(), EditError> {
        let (Item::Box(p), Parent::Card(src), Parent::Card(dst)) = (item, owner, to) else {
            return Err(EditError::NotAllowed);
        };
        if src != dst && self.descendants(item).contains(&Item::Card(dst)) {
            return Err(EditError::Cycle);
        }
        if src != dst && self.cards.get(&src).is_some_and(|c| c.premises.len() == 1) {
            // The card is about to vanish; label its paragraph while it
            // still lists the moved premise.
            self.salvage_essay_text(&[Item::Card(src)]);
        }
        self.detach(owner, item);
        let dst_card = self.cards.get_mut(&dst).ok_or(EditError::NotFound)?;
        let at = index.map_or(dst_card.premises.len(), |i| i.min(dst_card.premises.len()));
        dst_card.premises.insert(at, p);
        if self.cards[&src].premises.is_empty() {
            self.remove_inner(Item::Card(src), RemoveMode::KeepChildren)?;
        }
        Ok(())
    }

    /// Remove `item` from the document. The root cannot be removed.
    /// Removing a card's last premise removes the card, in the same mode.
    pub fn remove_item(&mut self, item: Item, mode: RemoveMode) -> Result<(), EditError> {
        self.remove_inner(item, mode)?;
        self.touch();
        #[cfg(debug_assertions)]
        self.check_invariants();
        Ok(())
    }

    fn remove_inner(&mut self, item: Item, mode: RemoveMode) -> Result<(), EditError> {
        if !self.exists(item) {
            return Err(EditError::NotFound);
        }
        if item == Item::Box(self.root) {
            return Err(EditError::NotAllowed);
        }
        let owner = self.owner_of(item).ok_or(EditError::NotFound)?;
        if let (Item::Box(_), Parent::Card(c)) = (item, owner)
            && self.is_premise(item)
            && self.cards[&c].premises.len() == 1
        {
            return self.remove_inner(Item::Card(c), mode);
        }
        self.detach(owner, item);
        let mut doomed = vec![item];
        match mode {
            RemoveMode::Branch => doomed.extend(self.descendants(item)),
            RemoveMode::KeepChildren => {
                let mut promoted = Vec::new();
                match item {
                    Item::Box(b) => promoted.extend(self.boxes[&b].children.iter().copied()),
                    Item::Card(c) => {
                        let card = &self.cards[&c];
                        for p in &card.premises {
                            doomed.push(Item::Box(*p));
                            promoted.extend(self.boxes[p].children.iter().copied());
                        }
                        promoted.extend(card.children.iter().map(|k| Item::Card(*k)));
                    }
                }
                self.parking.extend(promoted);
            }
        }
        self.salvage_essay_text(&doomed);
        self.ignored_hints.retain(|h| !doomed.contains(&h.item));
        for gone in doomed {
            match gone {
                Item::Box(b) => {
                    self.boxes.shift_remove(&b);
                }
                Item::Card(c) => {
                    self.cards.shift_remove(&c);
                }
            }
        }
        Ok(())
    }
}
