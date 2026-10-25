//! Editing operations on a [`Document`].

use thiserror::Error;

use crate::ids::{CardId, Item, NodeId, Parent};
use crate::model::{BoxKind, BoxNode, Card, CardKind, Document, now_rfc3339};
use crate::rules::{NewItem, can_attach};

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
    Source,
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
    pub(crate) fn touch(&mut self) {
        self.meta.modified = now_rfc3339();
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

    /// Add a card with one premise under `parent`.
    pub fn add_card(
        &mut self,
        parent: Parent,
        kind: CardKind,
        premise_text: &str,
    ) -> Result<(CardId, NodeId), EditError> {
        self.check_parent(parent)?;
        if !can_attach(self, parent, NewItem::Card(kind.clone())) {
            return Err(EditError::NotAllowed);
        }
        let card_id = CardId::new();
        let premise = NodeId::new();
        self.insert_into(parent, Item::Card(card_id), None)?;
        self.boxes.insert(
            premise,
            BoxNode::new(premise, BoxKind::Premise, premise_text),
        );
        self.cards
            .insert(card_id, Card::new(card_id, kind, vec![premise]));
        self.touch();
        #[cfg(debug_assertions)]
        self.check_invariants();
        Ok((card_id, premise))
    }

    /// Add a co-premise to a card at `index`; `None` appends.
    pub fn add_premise(
        &mut self,
        card: CardId,
        index: Option<usize>,
        text: &str,
    ) -> Result<NodeId, EditError> {
        let c = self.cards.get_mut(&card).ok_or(EditError::NotFound)?;
        let id = NodeId::new();
        let at = index.map_or(c.premises.len(), |i| i.min(c.premises.len()));
        c.premises.insert(at, id);
        self.boxes
            .insert(id, BoxNode::new(id, BoxKind::Premise, text));
        self.touch();
        #[cfg(debug_assertions)]
        self.check_invariants();
        Ok(id)
    }

    /// Add a Category, Source or Note box under `parent`.
    pub fn add_box(
        &mut self,
        parent: Parent,
        kind: BoxKind,
        text: &str,
    ) -> Result<NodeId, EditError> {
        let new = match kind {
            BoxKind::Category => NewItem::Category,
            BoxKind::Source { .. } => NewItem::Source,
            BoxKind::Note => NewItem::Note,
            BoxKind::Claim | BoxKind::Premise => return Err(EditError::NotAllowed),
        };
        self.check_parent(parent)?;
        if !can_attach(self, parent, new) {
            return Err(EditError::NotAllowed);
        }
        let id = NodeId::new();
        self.insert_into(parent, Item::Box(id), None)?;
        self.boxes.insert(id, BoxNode::new(id, kind, text));
        self.touch();
        #[cfg(debug_assertions)]
        self.check_invariants();
        Ok(id)
    }

    /// Replace a box's text.
    pub fn set_text(&mut self, id: NodeId, text: &str) -> Result<(), EditError> {
        self.boxes.get_mut(&id).ok_or(EditError::NotFound)?.text = text.to_owned();
        self.touch();
        #[cfg(debug_assertions)]
        self.check_invariants();
        Ok(())
    }

    /// Create a loose item in the parking area. A loose card gets one
    /// premise with `text`.
    pub fn add_loose(&mut self, kind: NewLoose, text: &str) -> Item {
        let item = match kind {
            NewLoose::Card(kind) => {
                let card_id = CardId::new();
                let premise = NodeId::new();
                self.boxes
                    .insert(premise, BoxNode::new(premise, BoxKind::Premise, text));
                self.cards
                    .insert(card_id, Card::new(card_id, kind, vec![premise]));
                Item::Card(card_id)
            }
            other => {
                let kind = match other {
                    NewLoose::Category => BoxKind::Category,
                    NewLoose::Source => BoxKind::Source {
                        source_type: "publication".to_owned(),
                    },
                    _ => BoxKind::Note,
                };
                let id = NodeId::new();
                self.boxes.insert(id, BoxNode::new(id, kind, text));
                Item::Box(id)
            }
        };
        self.parking.push(item);
        self.touch();
        #[cfg(debug_assertions)]
        self.check_invariants();
        item
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

    /// Check that the document is structurally sound: every referenced id
    /// exists; the root exists, is a claim and is owned by nothing; every
    /// other box and card is owned exactly once (box children, card premises,
    /// whole-card children or parking); premise boxes appear only in card
    /// premise lists and other boxes never do; every card has a premise; and
    /// everything is reachable from the root or a parking entry.
    ///
    /// Exactly-once ownership plus reachability also rules out cycles: a
    /// cycle has no way in from the root or parking without giving one of
    /// its members a second owner.
    pub fn validate(&self) -> Result<(), String> {
        use std::collections::{HashMap, HashSet};
        let root = self
            .boxes
            .get(&self.root)
            .ok_or_else(|| "the root box is missing".to_owned())?;
        if root.kind != BoxKind::Claim {
            return Err("the root box is not a claim".to_owned());
        }
        let mut owners: HashMap<Item, usize> = HashMap::new();
        let mut claim = |what: &str, item: Item| -> Result<(), String> {
            if !self.exists(item) {
                return Err(format!("{what} refer to a missing item {item:?}"));
            }
            *owners.entry(item).or_insert(0) += 1;
            Ok(())
        };
        for &item in &self.parking {
            if self.is_premise(item) {
                return Err(format!("premise {item:?} is in the parking area"));
            }
            claim("the parking area entries", item)?;
        }
        for node in self.boxes.values() {
            for &item in &node.children {
                if self.is_premise(item) {
                    return Err(format!("premise {item:?} is a child of box {:?}", node.id));
                }
                claim("box children", item)?;
            }
        }
        for card in self.cards.values() {
            if card.premises.is_empty() {
                return Err(format!("card {:?} has no premises", card.id));
            }
            for &p in &card.premises {
                let item = Item::Box(p);
                claim("card premises", item)?;
                if !self.is_premise(item) {
                    return Err(format!("card {:?} lists non-premise box {p:?}", card.id));
                }
            }
            for &k in &card.children {
                claim("whole-card children", Item::Card(k))?;
            }
        }
        let all = self
            .boxes
            .keys()
            .map(|b| Item::Box(*b))
            .chain(self.cards.keys().map(|c| Item::Card(*c)));
        for item in all {
            let n = owners.get(&item).copied().unwrap_or(0);
            if item == Item::Box(self.root) {
                if n != 0 {
                    return Err(format!("the root is owned {n} times"));
                }
            } else if n != 1 {
                return Err(format!("{item:?} is owned {n} times (expected 1)"));
            }
        }
        let mut seen: HashSet<Item> = HashSet::new();
        let mut stack: Vec<Item> = vec![Item::Box(self.root)];
        stack.extend_from_slice(&self.parking);
        while let Some(cur) = stack.pop() {
            if seen.insert(cur) {
                stack.extend(self.child_items(cur));
            }
        }
        let total = self.boxes.len() + self.cards.len();
        if seen.len() != total {
            return Err(format!(
                "{} items are unreachable (part of a cycle)",
                total - seen.len()
            ));
        }
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::MapKind;

    fn publication() -> BoxKind {
        BoxKind::Source {
            source_type: "publication".into(),
        }
    }

    #[test]
    fn root_cannot_mix_cards_and_categories() {
        let mut d = Document::new("C");
        d.add_card(Parent::Box(d.root), CardKind::Reason, "R")
            .unwrap();
        assert_eq!(
            d.add_box(Parent::Box(d.root), BoxKind::Category, "X"),
            Err(EditError::NotAllowed)
        );
        assert!(d.add_box(Parent::Box(d.root), BoxKind::Note, "n").is_ok());
        assert_eq!(d.map_kind(), MapKind::Reasoning);
    }

    #[test]
    fn card_targets_premise_or_whole_card() {
        let mut d = Document::new("C");
        let (card, p) = d
            .add_card(Parent::Box(d.root), CardKind::Reason, "P1")
            .unwrap();
        d.add_premise(card, None, "P2").unwrap();
        let (on_premise, _) = d
            .add_card(Parent::Box(p), CardKind::Objection, "against P1")
            .unwrap();
        let (on_card, _) = d
            .add_card(Parent::Card(card), CardKind::Objection, "link fails")
            .unwrap();
        assert_eq!(d.owner_of(Item::Card(on_premise)), Some(Parent::Box(p)));
        assert_eq!(d.owner_of(Item::Card(on_card)), Some(Parent::Card(card)));
        assert_eq!(d.cards[&card].premises.len(), 2);
        assert!(d.add_box(Parent::Card(card), publication(), "s").is_err());
    }

    #[test]
    fn add_premise_respects_index_and_missing_card() {
        let mut d = Document::new("C");
        let (card, p1) = d
            .add_card(Parent::Box(d.root), CardKind::Reason, "P1")
            .unwrap();
        let p0 = d.add_premise(card, Some(0), "P0").unwrap();
        assert_eq!(d.cards[&card].premises, vec![p0, p1]);
        assert_eq!(
            d.add_premise(CardId::new(), None, "x"),
            Err(EditError::NotFound)
        );
    }

    #[test]
    fn add_box_rejects_claim_and_premise_and_missing_parent() {
        let mut d = Document::new("C");
        assert_eq!(
            d.add_box(Parent::Box(d.root), BoxKind::Claim, "x"),
            Err(EditError::NotAllowed)
        );
        assert_eq!(
            d.add_box(Parent::Box(d.root), BoxKind::Premise, "x"),
            Err(EditError::NotAllowed)
        );
        assert_eq!(
            d.add_box(Parent::Box(NodeId::new()), BoxKind::Note, "x"),
            Err(EditError::NotFound)
        );
    }

    #[test]
    fn parking_accepts_and_set_text_updates() {
        let mut d = Document::new("C");
        let (_, p) = d
            .add_card(Parent::Parking, CardKind::Objection, "loose")
            .unwrap();
        assert_eq!(d.parking.len(), 1);
        d.meta.modified = "old".into();
        d.set_text(p, "changed").unwrap();
        assert_eq!(d.boxes[&p].text, "changed");
        assert_ne!(d.meta.modified, "old");
        assert_eq!(d.set_text(NodeId::new(), "x"), Err(EditError::NotFound));
    }

    fn card_under_root(d: &mut Document, text: &str) -> (CardId, NodeId) {
        d.add_card(Parent::Box(d.root), CardKind::Reason, text)
            .unwrap()
    }

    #[test]
    fn remove_last_premise_removes_card() {
        let mut d = Document::new("C");
        let (card, p) = card_under_root(&mut d, "P");
        let (sub, _) = d
            .add_card(Parent::Box(p), CardKind::Objection, "x")
            .unwrap();
        d.remove_item(Item::Box(p), RemoveMode::KeepChildren)
            .unwrap();
        assert!(!d.cards.contains_key(&card));
        assert!(!d.boxes.contains_key(&p));
        assert!(d.boxes[&d.root].children.is_empty());
        assert_eq!(d.parking, vec![Item::Card(sub)]);
    }

    #[test]
    fn remove_non_last_premise_keeps_card() {
        let mut d = Document::new("C");
        let (card, p1) = card_under_root(&mut d, "P1");
        let p2 = d.add_premise(card, None, "P2").unwrap();
        d.remove_item(Item::Box(p1), RemoveMode::Branch).unwrap();
        assert_eq!(d.cards[&card].premises, vec![p2]);
        assert!(!d.boxes.contains_key(&p1));
    }

    #[test]
    fn remove_branch_leaves_no_orphans() {
        let mut d = Document::new("C");
        let (card, p) = card_under_root(&mut d, "P");
        d.add_box(Parent::Box(p), publication(), "s").unwrap();
        d.add_card(Parent::Card(card), CardKind::Objection, "x")
            .unwrap();
        d.remove_item(Item::Card(card), RemoveMode::Branch).unwrap();
        assert_eq!(d.boxes.len(), 1);
        assert!(d.cards.is_empty());
        assert!(d.boxes[&d.root].children.is_empty());
    }

    #[test]
    fn remove_keep_children_moves_to_parking() {
        let mut d = Document::new("C");
        let (card, p) = card_under_root(&mut d, "P");
        let (sub, _) = d.add_card(Parent::Box(p), CardKind::Reason, "Q").unwrap();
        let (attacker, _) = d
            .add_card(Parent::Card(card), CardKind::Objection, "A")
            .unwrap();
        d.remove_item(Item::Card(card), RemoveMode::KeepChildren)
            .unwrap();
        assert_eq!(d.parking, vec![Item::Card(sub), Item::Card(attacker)]);
        assert!(!d.cards.contains_key(&card));
        assert!(!d.boxes.contains_key(&p));
    }

    #[test]
    fn remove_root_or_missing_is_refused() {
        let mut d = Document::new("C");
        assert_eq!(
            d.remove_item(Item::Box(d.root), RemoveMode::Branch),
            Err(EditError::NotAllowed)
        );
        assert_eq!(
            d.remove_item(Item::Card(CardId::new()), RemoveMode::Branch),
            Err(EditError::NotFound)
        );
    }

    #[test]
    fn connect_parking_card_to_premise() {
        let mut d = Document::new("C");
        let (_, p) = card_under_root(&mut d, "P");
        let loose = d.add_loose(NewLoose::Card(CardKind::Objection), "loose");
        assert_eq!(d.parking, vec![loose]);
        d.move_item(loose, Parent::Box(p), None).unwrap();
        assert!(d.parking.is_empty());
        assert_eq!(d.boxes[&p].children, vec![loose]);
        assert_eq!(d.owner_of(loose), Some(Parent::Box(p)));
    }

    #[test]
    fn move_checks_structure_rules() {
        let mut d = Document::new("C");
        let cat = d
            .add_box(Parent::Box(d.root), BoxKind::Category, "X")
            .unwrap();
        let note = d.add_loose(NewLoose::Note, "n");
        let Item::Box(note_id) = note else {
            unreachable!()
        };
        let other = d.add_loose(NewLoose::Note, "m");
        assert_eq!(
            d.move_item(other, Parent::Box(note_id), None),
            Err(EditError::NotAllowed)
        );
        let card = d.add_loose(NewLoose::Card(CardKind::Reason), "r");
        assert_eq!(
            d.move_item(card, Parent::Box(cat), None),
            Err(EditError::NotAllowed)
        );
        assert_eq!(
            d.move_item(card, Parent::Box(d.root), None),
            Err(EditError::NotAllowed)
        );
        assert_eq!(
            d.move_item(Item::Box(d.root), Parent::Parking, None),
            Err(EditError::NotAllowed)
        );
    }

    #[test]
    fn move_into_descendant_is_cycle() {
        let mut d = Document::new("C");
        let (a, pa) = card_under_root(&mut d, "A");
        let (_, pb) = d.add_card(Parent::Box(pa), CardKind::Reason, "B").unwrap();
        assert_eq!(
            d.move_item(Item::Card(a), Parent::Box(pb), None),
            Err(EditError::Cycle)
        );
        assert_eq!(
            d.move_item(Item::Card(a), Parent::Card(a), None),
            Err(EditError::Cycle)
        );
    }

    #[test]
    fn detach_to_parking_and_reorder() {
        let mut d = Document::new("C");
        let (a, _) = card_under_root(&mut d, "A");
        let (b, _) = card_under_root(&mut d, "B");
        let (c, _) = card_under_root(&mut d, "C");
        d.move_item(Item::Card(c), Parent::Box(d.root), Some(0))
            .unwrap();
        assert_eq!(
            d.boxes[&d.root].children,
            vec![Item::Card(c), Item::Card(a), Item::Card(b)]
        );
        d.move_item(Item::Card(a), Parent::Parking, None).unwrap();
        assert_eq!(d.parking, vec![Item::Card(a)]);
        assert_eq!(
            d.boxes[&d.root].children,
            vec![Item::Card(c), Item::Card(b)]
        );
    }

    #[test]
    fn move_premise_between_cards() {
        let mut d = Document::new("C");
        let (a, pa) = card_under_root(&mut d, "A");
        let (b, _) = card_under_root(&mut d, "B");
        let pa2 = d.add_premise(a, None, "A2").unwrap();
        d.move_item(Item::Box(pa2), Parent::Card(b), Some(0))
            .unwrap();
        assert_eq!(d.cards[&a].premises, vec![pa]);
        assert_eq!(d.cards[&b].premises[0], pa2);
        let (sub, _) = d
            .add_card(Parent::Box(pa), CardKind::Objection, "s")
            .unwrap();
        // A premise cannot move under its own subtree.
        assert_eq!(
            d.move_item(Item::Box(pa), Parent::Card(sub), None),
            Err(EditError::Cycle)
        );
        // A premise only moves between cards.
        assert_eq!(
            d.move_item(Item::Box(pa), Parent::Parking, None),
            Err(EditError::NotAllowed)
        );
        // Moving the last premise away removes the emptied card.
        d.move_item(Item::Box(pa), Parent::Card(b), None).unwrap();
        assert!(!d.cards.contains_key(&a));
        assert!(!d.boxes[&d.root].children.contains(&Item::Card(a)));
        assert_eq!(d.cards[&b].premises.len(), 3);
        assert_eq!(d.owner_of(Item::Card(sub)), Some(Parent::Box(pa)));
    }

    #[test]
    fn descendants_is_iterative_and_complete() {
        let mut d = Document::new("C");
        let (card, p) = card_under_root(&mut d, "P");
        let (att, _) = d
            .add_card(Parent::Card(card), CardKind::Objection, "A")
            .unwrap();
        let mut under = p;
        let mut expected = 3; // premise, attacker, attacker's premise
        for _ in 0..500 {
            let (_, np) = d
                .add_card(Parent::Box(under), CardKind::Reason, "n")
                .unwrap();
            under = np;
            expected += 2;
        }
        let all = d.descendants(Item::Card(card));
        assert_eq!(all.len(), expected);
        assert!(all.contains(&Item::Card(att)));
        assert!(!all.contains(&Item::Card(card)));
        d.remove_item(Item::Card(card), RemoveMode::Branch).unwrap();
        assert_eq!(d.boxes.len(), 1);
    }

    #[test]
    #[should_panic(expected = "invariant")]
    fn invariants_catch_dangling_ids() {
        let mut d = Document::new("C");
        let root = d.root;
        d.boxes
            .get_mut(&root)
            .unwrap()
            .children
            .push(Item::Card(CardId::new()));
        d.check_invariants();
    }

    /// A root with one reason card `a` (premise `p`) whose premise has an
    /// objection card `b` (premise `q`).
    fn two_level() -> (Document, CardId, NodeId, CardId, NodeId) {
        let mut d = Document::new("C");
        let (a, p) = card_under_root(&mut d, "P");
        let (b, q) = d
            .add_card(Parent::Box(p), CardKind::Objection, "Q")
            .unwrap();
        (d, a, p, b, q)
    }

    #[test]
    fn validate_accepts_edited_documents() {
        let (mut d, a, _, _, _) = two_level();
        d.add_card(Parent::Card(a), CardKind::Objection, "W")
            .unwrap();
        d.add_loose(NewLoose::Note, "n");
        assert_eq!(d.validate(), Ok(()));
    }

    #[test]
    fn validate_rejects_damage() {
        let (d, a, p, b, q) = two_level();
        let root = d.root;
        let broken = |f: &dyn Fn(&mut Document)| {
            let mut x = d.clone();
            f(&mut x);
            x.validate()
        };
        // Dangling id.
        assert!(broken(&|x| x.parking.push(Item::Card(CardId::new()))).is_err());
        // Root missing, or not a claim, or owned.
        assert!(broken(&|x| x.root = NodeId::new()).is_err());
        assert!(broken(&|x| x.boxes[&root].kind = BoxKind::Note).is_err());
        assert!(broken(&|x| x.parking.push(Item::Box(root))).is_err());
        // Owned twice.
        assert!(broken(&|x| x.parking.push(Item::Card(b))).is_err());
        // Unowned.
        assert!(broken(&|x| x.boxes[&p].children.clear()).is_err());
        // A premise moved out of its card into a box or the parking area.
        let out_of_card = |to_parking: bool| {
            move |x: &mut Document| {
                x.cards[&b].premises.push(NodeId::new());
                let extra = *x.cards[&b].premises.last().unwrap();
                x.boxes
                    .insert(extra, BoxNode::new(extra, BoxKind::Premise, "x"));
                x.cards[&b].premises.retain(|i| *i != q);
                if to_parking {
                    x.parking.push(Item::Box(q));
                } else {
                    x.boxes[&root].children.push(Item::Box(q));
                }
            }
        };
        assert!(broken(&out_of_card(true)).unwrap_err().contains("premise"));
        assert!(broken(&out_of_card(false)).unwrap_err().contains("premise"));
        // A non-premise box listed as a premise.
        assert!(broken(&|x| x.boxes[&q].kind = BoxKind::Note).is_err());
        // A card without premises.
        assert!(broken(&|x| x.cards[&b].premises.clear()).is_err());
        // A card removed from the map but still referenced.
        assert!(
            broken(&|x| {
                x.cards.shift_remove(&a);
            })
            .is_err()
        );
    }

    #[test]
    fn validate_rejects_cycles_and_descendants_terminates() {
        let (mut d, a, p, b, _) = two_level();
        // Detach `a` from the root and hang it under its own descendant:
        // a -> p -> b -> q, and q now owns a.
        let root = d.root;
        d.boxes[&root].children.clear();
        let q = d.cards[&b].premises[0];
        d.boxes[&q].children.push(Item::Card(a));
        let err = d.validate().unwrap_err();
        assert!(err.contains("unreachable"), "{err}");
        let under = d.descendants(Item::Card(a));
        assert!(under.contains(&Item::Box(p)));
        assert!(under.contains(&Item::Card(b)));
        assert!(!under.contains(&Item::Card(a)));
    }
}
