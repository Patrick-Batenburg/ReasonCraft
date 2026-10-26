//! Argdown (argdown.org) interchange: the map as Argdown text.
//!
//! The mapping is documented in `docs/argdown.md`. In short: the root and
//! every other box are statements with titles made from their text, every
//! card is an argument (`<R1>`, `<O1>`) with a premise-conclusion
//! structure, and ReasonCraft-only details (source types, ratings, notes,
//! comments, categories) go in data `{…}`. The parking area always follows
//! under a "Not connected" heading: the file is meant to be lossless.
//!
//! Import reads general Argdown: [`from_argdown`] lexes and parses the
//! text, reads it as a graph of statements and arguments, and lays that
//! out as a map, noting in an [`ImportReport`] what it could not keep.

mod build;
mod data;
mod fields;
mod import;
mod lex;
mod parse;
mod report;
mod text;
mod titles;
mod write;

pub use import::from_argdown;
pub use report::{
    ArgdownError, ImportNote, ImportNoteKind, ImportReport, MAX_ARGDOWN_BYTES, MAX_IMPORT_BOXES,
    MAX_NESTING,
};
pub use write::to_argdown;

#[cfg(test)]
mod tests_corpus;
#[cfg(test)]
mod tests_import;
#[cfg(test)]
mod tests_write;
