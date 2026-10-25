//! Adding cards, premises and boxes.

use crate::ids::{CardId, Item, NodeId, Parent};
use crate::model::{BoxKind, BoxNode, Card, CardKind, Document};
use crate::rules::{NewItem, can_attach};

use super::{EditError, NewLoose};

impl Document {
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
                    NewLoose::Source(source_type) => BoxKind::Source { source_type },
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
}
