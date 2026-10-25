//! The document model: boxes, cards and the document that owns them.

use chrono::{SecondsFormat, Utc};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::snippets::Snippet;

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
    Source { source_type: SourceType },
    Note,
}

/// The kind of a source box. A type name this build does not know (from a
/// newer file) is kept as [`SourceType::Other`], written back unchanged and
/// shown as the generic source.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SourceType {
    Source,
    MyBasis,
    Assertion,
    ByDefinition,
    CaseStudy,
    CommonBelief,
    Data,
    Event,
    Example,
    ExpertOpinion,
    Law,
    Media,
    PersonalExperience,
    Publication,
    Quote,
    Statistic,
    Web,
    /// An unknown type name, kept verbatim.
    Other(String),
}

impl SourceType {
    /// Every selectable type, in menu order (without `Other`).
    pub const ALL: [SourceType; 17] = [
        SourceType::Source,
        SourceType::MyBasis,
        SourceType::Assertion,
        SourceType::ByDefinition,
        SourceType::CaseStudy,
        SourceType::CommonBelief,
        SourceType::Data,
        SourceType::Event,
        SourceType::Example,
        SourceType::ExpertOpinion,
        SourceType::Law,
        SourceType::Media,
        SourceType::PersonalExperience,
        SourceType::Publication,
        SourceType::Quote,
        SourceType::Statistic,
        SourceType::Web,
    ];

    /// The stable snake_case name; `Other` shows as the generic `"source"`.
    pub fn key(&self) -> &'static str {
        match self {
            SourceType::Source | SourceType::Other(_) => "source",
            SourceType::MyBasis => "my_basis",
            SourceType::Assertion => "assertion",
            SourceType::ByDefinition => "by_definition",
            SourceType::CaseStudy => "case_study",
            SourceType::CommonBelief => "common_belief",
            SourceType::Data => "data",
            SourceType::Event => "event",
            SourceType::Example => "example",
            SourceType::ExpertOpinion => "expert_opinion",
            SourceType::Law => "law",
            SourceType::Media => "media",
            SourceType::PersonalExperience => "personal_experience",
            SourceType::Publication => "publication",
            SourceType::Quote => "quote",
            SourceType::Statistic => "statistic",
            SourceType::Web => "web",
        }
    }

    /// The type named `name` in a file; unknown names become `Other`.
    pub fn from_name(name: &str) -> SourceType {
        SourceType::ALL
            .into_iter()
            .find(|t| t.key() == name)
            .unwrap_or_else(|| SourceType::Other(name.to_owned()))
    }

    /// The name written to a file: the key, or an unknown name as it was read.
    pub fn name(&self) -> &str {
        match self {
            SourceType::Other(name) => name,
            known => known.key(),
        }
    }
}

impl Serialize for SourceType {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.name())
    }
}

impl<'de> Deserialize<'de> for SourceType {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let name = String::deserialize(d)?;
        Ok(SourceType::from_name(&name))
    }
}

/// The reader's verdict on a claim or premise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Acceptance {
    Accept,
    Unsure,
    Reject,
}

/// How strongly a card's premises carry its conclusion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardStrength {
    Strong,
    Weak,
    Nil,
}

/// How reliable a source is judged to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceRating {
    Solid,
    Shaky,
    Nil,
}

/// The tone of a comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Positive,
    NeedsWork,
    Problem,
}

/// A comment attached to a box. `created` is RFC 3339.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Comment {
    pub id: Uuid,
    pub author: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verdict: Option<Verdict>,
    pub created: String,
}

/// A reference to an image asset stored beside the document.
/// `aspect` is height divided by width.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageRef {
    pub asset: String,
    pub aspect: f32,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acceptance: Option<Acceptance>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_rating: Option<SourceRating>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<Comment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<ImageRef>,
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
            acceptance: None,
            source_rating: None,
            link: None,
            comments: Vec::new(),
            image: None,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strength: Option<CardStrength>,
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
            strength: None,
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
    /// Source text kept beside the map.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub snippets: Vec<Snippet>,
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
            snippets: Vec::new(),
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
    /// See [`crate::roles::role_of`].
    pub fn card_role(&self, id: CardId) -> Role {
        crate::roles::role_of(self, id)
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
mod tests;
