//! What an Argdown import could not keep as it was, and why it failed.

/// The largest Argdown file the import reads, in bytes.
pub const MAX_ARGDOWN_BYTES: usize = 2 * 1024 * 1024;
/// The most boxes an imported map may have.
pub const MAX_IMPORT_BOXES: usize = 5000;
/// The deepest nesting of relation lines (and of data values) read.
pub const MAX_NESTING: usize = 64;

/// Why an Argdown text could not be imported.
#[derive(Debug, thiserror::Error)]
pub enum ArgdownError {
    #[error("the file is larger than {MAX_ARGDOWN_BYTES} bytes")]
    TooLarge,
    #[error("the map would have more than {MAX_IMPORT_BOXES} boxes")]
    TooManyBoxes,
    #[error("relations are nested more than {MAX_NESTING} levels deep (line {line})")]
    TooDeep { line: usize },
    #[error("the file holds no statements or arguments")]
    Empty,
}

/// A construct the map cannot hold as written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImportNoteKind {
    /// A `><` relation: maps have no contradictions; it was left out.
    Contradiction,
    /// Another wording of a statement already defined, or another
    /// description or PCS of an argument; the first is used.
    ExtraEquivalentWording,
    /// A statement used in a second place became a copy there.
    SharedStatementCopied,
    /// A relation that would close a cycle was left out.
    CycleBroken,
    /// A reference to a title defined nowhere; the title became the text.
    UnknownReference(String),
    /// A conclusion in the middle of an argument was left out.
    IntermediateConclusion,
    /// The inference rule between the hyphens was left out.
    InferenceRuleDropped,
    /// Tags (`#tag`) were left out.
    TagsDropped,
    /// An undercut under a statement was read as an attack.
    UndercutOnStatement,
    /// An argument with no premises, text or title was left out.
    EmptyArgument,
    /// An item could not go where the file puts it (say, a reason under a
    /// category) and went to the parking area.
    MovedToParking,
    /// A relation into a conclusion that was left out (an intermediate
    /// one, or a placeholder) was left out too.
    RelationDropped,
    /// A link that is not a safe web or mail link was left out.
    LinkDropped,
    /// Text longer than a map keeps was cut: a box's text past
    /// [`MAX_BOX_CHARS`](crate::model::MAX_BOX_CHARS) characters, or a
    /// comment, title or author past its cap (see [`crate::caps`]); a
    /// link too long to keep was left out.
    TextCut,
}

impl ImportNoteKind {
    /// The snake_case name, for the `argdown.note.<kind>` message.
    pub fn key(&self) -> &'static str {
        match self {
            ImportNoteKind::Contradiction => "contradiction",
            ImportNoteKind::ExtraEquivalentWording => "extra_equivalent_wording",
            ImportNoteKind::SharedStatementCopied => "shared_statement_copied",
            ImportNoteKind::CycleBroken => "cycle_broken",
            ImportNoteKind::UnknownReference(_) => "unknown_reference",
            ImportNoteKind::IntermediateConclusion => "intermediate_conclusion",
            ImportNoteKind::InferenceRuleDropped => "inference_rule_dropped",
            ImportNoteKind::TagsDropped => "tags_dropped",
            ImportNoteKind::UndercutOnStatement => "undercut_on_statement",
            ImportNoteKind::EmptyArgument => "empty_argument",
            ImportNoteKind::MovedToParking => "moved_to_parking",
            ImportNoteKind::RelationDropped => "relation_dropped",
            ImportNoteKind::LinkDropped => "link_dropped",
            ImportNoteKind::TextCut => "text_cut",
        }
    }
}

/// One thing the import changed or left out, at a line of the file
/// (1-based).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportNote {
    pub line: usize,
    pub kind: ImportNoteKind,
}

/// Everything the import changed or left out, in the order found.
#[derive(Clone, Debug, Default)]
pub struct ImportReport {
    pub notes: Vec<ImportNote>,
}

impl ImportReport {
    pub(super) fn note(&mut self, line: usize, kind: ImportNoteKind) {
        self.notes.push(ImportNote { line, kind });
    }
}
