//! Structural validation.

use crate::ids::Item;
use crate::model::{BoxKind, BoxNode, Document};
use crate::rules::{NewItem, allowed_child};

impl Document {
    /// Check that the document is structurally sound: every box and card is
    /// stored under its own id; every referenced id
    /// exists; the root exists, is a claim and is owned by nothing; every
    /// other box and card is owned exactly once (box children, card premises,
    /// whole-card children or parking); premise boxes appear only in card
    /// premise lists and other boxes never do; every card has a premise; and
    /// everything is reachable from the root or a parking entry; and every
    /// box holds only what the structure rules allow under it.
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
        for (key, node) in &self.boxes {
            let id = node.id;
            if id != *key {
                return Err(format!("box {id:?} is stored under the key {key:?}"));
            }
            if node.acceptance.is_some() && !matches!(node.kind, BoxKind::Claim | BoxKind::Premise)
            {
                return Err(format!(
                    "box {id:?} has an acceptance but is not a claim or premise"
                ));
            }
            if node.source_rating.is_some() && !matches!(node.kind, BoxKind::Source { .. }) {
                return Err(format!(
                    "box {id:?} has a source rating but is not a source"
                ));
            }
            if node.image.is_some() && node.kind == BoxKind::Note {
                return Err(format!("note {id:?} has an image"));
            }
            let mut ids = HashSet::new();
            if let Some(c) = node.comments.iter().find(|c| !ids.insert(c.id)) {
                return Err(format!("box {id:?} has duplicate comment id {}", c.id));
            }
        }
        for (key, card) in &self.cards {
            if card.id != *key {
                return Err(format!(
                    "card {:?} is stored under the key {key:?}",
                    card.id
                ));
            }
        }
        let mut snippet_ids = HashSet::new();
        if let Some(s) = self.snippets.iter().find(|s| !snippet_ids.insert(s.id)) {
            return Err(format!("duplicate snippet id {}", s.id));
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
                self.check_attachment(node, item)?;
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
}

impl Document {
    /// Check that `owner` may hold `item` under the structure rules (the
    /// table in [`crate::rules`]); a claim also never holds both cards and
    /// categories.
    fn check_attachment(&self, owner: &BoxNode, item: Item) -> Result<(), String> {
        let child = match item {
            Item::Card(c) => NewItem::Card(self.cards[&c].kind.clone()),
            Item::Box(b) => match self.boxes[&b].kind {
                BoxKind::Category => NewItem::Category,
                BoxKind::Source { .. } => NewItem::Source,
                BoxKind::Note => NewItem::Note,
                // Premises are refused by the caller; a claim is only the root.
                BoxKind::Claim | BoxKind::Premise => {
                    return Err(format!("{item:?} cannot be under box {:?}", owner.id));
                }
            },
        };
        if !allowed_child(&owner.kind, &child) {
            return Err(format!("{item:?} cannot be under box {:?}", owner.id));
        }
        if owner.kind == BoxKind::Claim {
            let cards = owner.children.iter().any(|i| matches!(i, Item::Card(_)));
            let categories = owner
                .children
                .iter()
                .any(|i| matches!(i, Item::Box(b) if self.boxes[b].kind == BoxKind::Category));
            if cards && categories {
                return Err(format!("claim {:?} mixes cards and categories", owner.id));
            }
        }
        Ok(())
    }
}
