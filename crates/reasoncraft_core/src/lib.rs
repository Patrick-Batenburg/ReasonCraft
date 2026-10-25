//! ReasonCraft core: document model, editing, layout and file format.

pub mod ids;
pub mod model;

pub use ids::{CardId, Item, NodeId, Parent};
pub use model::{
    BoxKind, BoxNode, Card, CardKind, DocMeta, Document, MapKind, Role, SCHEMA_VERSION,
};
