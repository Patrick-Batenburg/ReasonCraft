//! Argdown (argdown.org) interchange: the map as Argdown text.
//!
//! The mapping is documented in `docs/argdown.md`. In short: the root and
//! every other box are statements with titles made from their text, every
//! card is an argument (`<R1>`, `<O1>`) with a premise-conclusion
//! structure, and ReasonCraft-only details (source types, ratings, notes,
//! comments, categories) go in data `{…}`. The parking area always follows
//! under a "Not connected" heading: the file is meant to be lossless.

mod titles;
mod write;

pub use write::to_argdown;

#[cfg(test)]
mod tests_write;
