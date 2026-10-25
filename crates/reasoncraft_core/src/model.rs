//! The document model: boxes, cards and the document that owns them.

use chrono::{SecondsFormat, Utc};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::ids::{CardId, Item, NodeId, Parent};

/// The current time as an RFC 3339 string with second precision.
pub fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// Current file schema version.
pub const SCHEMA_VERSION: u32 = 1;

/// What a box represents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BoxKind {
    Claim,
    Premise,
    Category,
    /// `source_type` is a stable key such as `"publication"`.
    Source {
        source_type: String,
    },
    Note,
}

/// A box in the map.
///
/// `children` holds cards targeting this box plus sources, notes and
/// categories under it, in display order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoxNode {
    pub id: NodeId,
    pub kind: BoxKind,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f32>,
    #[serde(default)]
    pub children: Vec<Item>,
    /// Unknown fields, kept so files roundtrip.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl BoxNode {
    pub fn new(id: NodeId, kind: BoxKind, text: &str) -> Self {
        Self {
            id,
            kind,
            text: text.to_owned(),
            width: None,
            children: Vec::new(),
            extra: Map::new(),
        }
    }
}

/// Whether a card gives a reason or an objection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardKind {
    Reason,
    Objection,
}

/// A group of premises that together support or oppose a target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Card {
    pub id: CardId,
    pub kind: CardKind,
    pub premises: Vec<NodeId>,
    /// Cards targeting the whole card.
    #[serde(default)]
    pub children: Vec<CardId>,
    /// Unknown fields, kept so files roundtrip.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Card {
    pub fn new(id: CardId, kind: CardKind, premises: Vec<NodeId>) -> Self {
        Self {
            id,
            kind,
            premises,
            children: Vec::new(),
            extra: Map::new(),
        }
    }
}

/// Document metadata. Timestamps are RFC 3339; language is `"en"` or `"nl"`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocMeta {
    pub title: String,
    pub author: String,
    pub created: String,
    pub modified: String,
    pub language: String,
}

/// The kind of map, judged from the root's children.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapKind {
    Empty,
    Reasoning,
    Grouping,
}

/// The visual and logical role of a card, derived from structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Supports,
    Opposes,
    Rebuts,
}

/// A whole argument map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub schema_version: u32,
    pub meta: DocMeta,
    pub root: NodeId,
    pub boxes: IndexMap<NodeId, BoxNode>,
    pub cards: IndexMap<CardId, Card>,
    /// Loose items that are not attached to the tree.
    #[serde(default)]
    pub parking: Vec<Item>,
    /// Unknown fields, kept so files roundtrip.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Document {
    /// A new document whose root is a claim with the given text.
    pub fn new(claim_text: &str) -> Self {
        let root = NodeId::new();
        let mut boxes = IndexMap::new();
        boxes.insert(root, BoxNode::new(root, BoxKind::Claim, claim_text));
        let now = now_rfc3339();
        Self {
            schema_version: SCHEMA_VERSION,
            meta: DocMeta {
                title: String::new(),
                author: String::new(),
                created: now.clone(),
                modified: now,
                language: "en".to_owned(),
            },
            root,
            boxes,
            cards: IndexMap::new(),
            parking: Vec::new(),
            extra: Map::new(),
        }
    }

    /// Any card under the root makes a reasoning map, otherwise any category
    /// makes a grouping map.
    pub fn map_kind(&self) -> MapKind {
        let Some(root) = self.boxes.get(&self.root) else {
            return MapKind::Empty;
        };
        if root.children.iter().any(|i| matches!(i, Item::Card(_))) {
            return MapKind::Reasoning;
        }
        let has_category = root.children.iter().any(|i| {
            matches!(i, Item::Box(b) if self.boxes.get(b).is_some_and(|n| n.kind == BoxKind::Category))
        });
        if has_category {
            MapKind::Grouping
        } else {
            MapKind::Empty
        }
    }

    /// Reasons support. An objection whose owner sits inside an objection
    /// card rebuts; any other objection (including loose ones) opposes.
    pub fn card_role(&self, id: CardId) -> Role {
        let Some(card) = self.cards.get(&id) else {
            return Role::Opposes;
        };
        if card.kind == CardKind::Reason {
            return Role::Supports;
        }
        let in_objection = |c: &Card| c.kind == CardKind::Objection;
        let rebuts = match self.owner_of(Item::Card(id)) {
            Some(Parent::Card(c)) => self.cards.get(&c).is_some_and(in_objection),
            Some(Parent::Box(b)) => self
                .cards
                .values()
                .any(|c| in_objection(c) && c.premises.contains(&b)),
            Some(Parent::Parking) | None => false,
        };
        if rebuts { Role::Rebuts } else { Role::Opposes }
    }

    /// The parent that holds `item`: `Some(Parent::Parking)` for parked items,
    /// `None` for the root or an item that is not placed anywhere.
    /// For a premise box, the owner is the card that lists it.
    pub fn owner_of(&self, item: Item) -> Option<Parent> {
        if self.parking.contains(&item) {
            return Some(Parent::Parking);
        }
        if let Some(b) = self.boxes.values().find(|b| b.children.contains(&item)) {
            return Some(Parent::Box(b.id));
        }
        match item {
            Item::Card(id) => self
                .cards
                .values()
                .find(|c| c.children.contains(&id))
                .map(|c| Parent::Card(c.id)),
            Item::Box(id) => self
                .cards
                .values()
                .find(|c| c.premises.contains(&id))
                .map(|c| Parent::Card(c.id)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_document_has_root_claim() {
        let d = Document::new("Robots create more jobs");
        let root = &d.boxes[&d.root];
        assert_eq!(root.kind, BoxKind::Claim);
        assert_eq!(root.text, "Robots create more jobs");
        assert_eq!(d.schema_version, SCHEMA_VERSION);
        assert!(d.parking.is_empty());
        assert_eq!(d.map_kind(), MapKind::Empty);
    }

    #[test]
    fn rebuttal_is_derived_from_structure() {
        // root <- objection(card A, premise p) <- objection(card B) => B rebuts
        let mut d = Document::new("C");
        let a = CardId::new();
        let p = NodeId::new();
        let b = CardId::new();
        let q = NodeId::new();
        d.boxes.insert(p, BoxNode::new(p, BoxKind::Premise, "P"));
        d.boxes.insert(q, BoxNode::new(q, BoxKind::Premise, "Q"));
        d.cards
            .insert(a, Card::new(a, CardKind::Objection, vec![p]));
        d.cards
            .insert(b, Card::new(b, CardKind::Objection, vec![q]));
        d.boxes
            .get_mut(&d.root)
            .unwrap()
            .children
            .push(Item::Card(a));
        d.boxes.get_mut(&p).unwrap().children.push(Item::Card(b));
        assert_eq!(d.card_role(a), Role::Opposes);
        assert_eq!(d.card_role(b), Role::Rebuts);
        assert_eq!(d.map_kind(), MapKind::Reasoning);
    }
}
