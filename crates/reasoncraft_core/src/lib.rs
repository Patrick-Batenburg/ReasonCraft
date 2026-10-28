//! ReasonCraft core: document model, editing, layout and file format.

// The docs are built with `--document-private-items` (see CI), where
// links from public items to private ones resolve.
#![allow(rustdoc::private_intra_doc_links)]

pub mod argdown;
pub mod caps;
pub mod checks;
pub mod connector;
pub mod edit;
pub mod essay;
pub mod evaluate;
pub mod examples;
pub mod file;
pub mod find;
#[cfg(feature = "test-fixtures")]
pub mod fixtures;
pub mod history;
pub mod ids;
pub mod lang;
pub mod layout;
pub mod link;
pub mod meta;
pub mod model;
pub mod nav;
pub mod outline;
pub mod outline_md;
pub mod package;
pub mod roles;
pub mod rules;
pub mod snippets;
pub mod words;

pub use argdown::{
    ArgdownError, ImportNote, ImportNoteKind, ImportReport, from_argdown, to_argdown,
};
pub use connector::ConnectorStyle;
pub use edit::{EditError, Fragment, NewLoose, RemoveMode};
pub use essay::{EssayPlan, EssaySlot, essay_markdown, essay_order, essay_plain_text};
pub use evaluate::ClearScope;
pub use find::{FindHit, FindOptions, find_all};
pub use history::History;
pub use ids::{CardId, Item, NodeId, Parent};
pub use layout::{Connector, ConnectorKind, Layout, LayoutParams, Measure, TargetKind, layout};
pub use model::{
    Acceptance, BoxKind, BoxNode, Card, CardKind, CardStrength, Comment, DocMeta, Document,
    IgnoredHint, ImageRef, LayoutDirection, MAX_BOX_CHARS, MapKind, Role, SCHEMA_VERSION,
    SourceRating, SourceType, Verdict,
};
pub use package::{Assets, Package};
pub use roles::{card_roles, owner_index, role_of};
pub use rules::{BoxRating, NewItem, can_attach, is_attachment, kind_accepts, same_display_group};
pub use snippets::{MAX_SNIPPET_CHARS, Snippet, SnippetSource};
/// Comment ids.
pub use uuid::Uuid;
