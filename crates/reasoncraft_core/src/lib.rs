//! ReasonCraft core: document model, editing, layout and file format.

pub mod edit;
pub mod ids;
pub mod model;
pub mod rules;

pub use edit::{EditError, NewLoose, RemoveMode};
pub use ids::{CardId, Item, NodeId, Parent};
pub use model::{
    BoxKind, BoxNode, Card, CardKind, DocMeta, Document, MapKind, Role, SCHEMA_VERSION,
};
pub use rules::{NewItem, can_attach};
