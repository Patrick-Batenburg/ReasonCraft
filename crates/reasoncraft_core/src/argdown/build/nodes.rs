//! Builder helpers: creating boxes and cards (up to the import limit),
//! attaching them, and applying a statement's data.

use super::super::fields;

use super::super::report::MAX_IMPORT_BOXES;
use super::Builder;
use crate::ids::{CardId, Item, NodeId, Parent};
use crate::model::{BoxKind, BoxNode, Card, CardKind, CardStrength};
use crate::rules::NewItem;

impl Builder<'_> {
    /// A box of `kind` with statement `x`'s text, and with its data when
    /// `data` names it. `None` past [`MAX_IMPORT_BOXES`].
    pub(super) fn new_box(
        &mut self,
        kind: BoxKind,
        data: Option<usize>,
        x: usize,
    ) -> Option<NodeId> {
        let id = NodeId::new();
        let text = self.g.stmts[x].text.clone().unwrap_or_default();
        if !self.insert_box(BoxNode::new(id, kind, &text)) {
            return None;
        }
        if let Some(s) = data {
            self.apply_data(id, s);
        }
        Some(id)
    }

    pub(super) fn insert_box(&mut self, node: BoxNode) -> bool {
        if self.doc.boxes.len() >= MAX_IMPORT_BOXES {
            self.too_many = true;
            return false;
        }
        self.doc.boxes.insert(node.id, node);
        true
    }

    pub(super) fn new_card(
        &mut self,
        kind: CardKind,
        premises: Vec<NodeId>,
        strength: Option<CardStrength>,
        parent: Parent,
    ) -> CardId {
        let id = CardId::new();
        let mut card = Card::new(id, kind, premises);
        card.strength = strength;
        self.doc.cards.insert(id, card);
        self.attach(parent, Item::Card(id));
        id
    }

    pub(super) fn attach(&mut self, parent: Parent, item: Item) {
        match (parent, item) {
            (Parent::Box(b), item) => {
                if let Some(n) = self.doc.boxes.get_mut(&b) {
                    n.children.push(item);
                }
            }
            (Parent::Card(c), Item::Card(k)) => {
                if let Some(card) = self.doc.cards.get_mut(&c) {
                    card.children.push(k);
                }
            }
            (Parent::Card(_), Item::Box(_)) => {
                debug_assert!(false, "a box under a card");
            }
            (Parent::Parking, item) => self.doc.parking.push(item),
        }
    }

    /// Statement `s`'s data on box `id`, with its notes as boxes.
    pub(super) fn apply_data(&mut self, id: NodeId, s: usize) {
        let Some(node) = self.doc.boxes.get_mut(&id) else {
            return;
        };
        for note in fields::set_fields(node, &self.g.stmts[s]) {
            let n = NodeId::new();
            if !self.insert_box(BoxNode::new(n, BoxKind::Note, &note)) {
                return;
            }
            self.attach(Parent::Box(id), Item::Box(n));
        }
    }
}

pub(super) fn new_item(kind: &BoxKind) -> NewItem {
    match kind {
        BoxKind::Category => NewItem::Category,
        BoxKind::Note => NewItem::Note,
        _ => NewItem::Source,
    }
}
