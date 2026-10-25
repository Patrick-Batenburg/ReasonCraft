//! ReasonCraft core: document model, editing, layout and file format.

pub mod checks;
pub mod connector;
pub mod edit;
pub mod evaluate;
pub mod file;
#[cfg(feature = "test-fixtures")]
pub mod fixtures;
pub mod history;
pub mod ids;
pub mod layout;
pub mod meta;
pub mod model;
pub mod outline;
pub mod package;
pub mod rules;
pub mod snippets;

pub use edit::{EditError, NewLoose, RemoveMode};
pub use evaluate::ClearScope;
pub use history::History;
pub use ids::{CardId, Item, NodeId, Parent};
pub use layout::{Connector, ConnectorKind, Layout, LayoutParams, Measure, TargetKind, layout};
pub use model::{
    Acceptance, BoxKind, BoxNode, Card, CardKind, CardStrength, Comment, DocMeta, Document,
    ImageRef, MapKind, Role, SCHEMA_VERSION, SourceRating, SourceType, Verdict, card_roles,
    owner_index,
};
pub use package::{Assets, Package};
pub use rules::{NewItem, can_attach};
pub use snippets::{MAX_SNIPPET_CHARS, Snippet, SnippetSource};
/// Comment ids.
pub use uuid::Uuid;
