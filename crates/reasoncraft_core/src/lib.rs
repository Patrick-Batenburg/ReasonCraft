//! ReasonCraft core: document model, editing, layout and file format.

pub mod connector;
pub mod edit;
pub mod evaluate;
pub mod file;
#[cfg(feature = "test-fixtures")]
pub mod fixtures;
pub mod history;
pub mod ids;
pub mod layout;
pub mod model;
pub mod rules;

pub use edit::{EditError, NewLoose, RemoveMode};
pub use evaluate::ClearScope;
pub use history::History;
pub use ids::{CardId, Item, NodeId, Parent};
pub use layout::{Connector, ConnectorKind, Layout, LayoutParams, Measure, TargetKind, layout};
pub use model::{
    Acceptance, BoxKind, BoxNode, Card, CardKind, CardStrength, Comment, DocMeta, Document,
    ImageRef, MapKind, Role, SCHEMA_VERSION, SourceRating, SourceType, Verdict, owner_index,
};
pub use rules::{NewItem, can_attach};
