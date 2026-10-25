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

impl Document {
    pub(crate) fn touch(&mut self) {
        self.meta.modified = now_rfc3339();
    }

    fn insert_into(&mut self, parent: Parent, item: Item) -> Result<(), EditError> {
        match (parent, item) {
            (Parent::Box(b), item) => self
                .boxes
                .get_mut(&b)
                .ok_or(EditError::NotFound)?
                .children
                .push(item),
            (Parent::Card(c), Item::Card(id)) => self
                .cards
                .get_mut(&c)
                .ok_or(EditError::NotFound)?
                .children
                .push(id),
            (Parent::Card(_), Item::Box(_)) => return Err(EditError::NotAllowed),
            (Parent::Parking, item) => self.parking.push(item),
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
        self.insert_into(parent, Item::Card(card_id))?;
        self.boxes.insert(
            premise,
            BoxNode::new(premise, BoxKind::Premise, premise_text),
        );
        self.cards
            .insert(card_id, Card::new(card_id, kind, vec![premise]));
        self.touch();
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
        self.insert_into(parent, Item::Box(id))?;
        self.boxes.insert(id, BoxNode::new(id, kind, text));
        self.touch();
        Ok(id)
    }

    /// Replace a box's text.
    pub fn set_text(&mut self, id: NodeId, text: &str) -> Result<(), EditError> {
        self.boxes.get_mut(&id).ok_or(EditError::NotFound)?.text = text.to_owned();
        self.touch();
        Ok(())
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
}
