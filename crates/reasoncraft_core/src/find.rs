//! Find and replace across the text of every box: claims, premises,
//! categories, source citations and notes.
//!
//! Boxes are searched in [`search_order`]: the main tree in outline order,
//! then the parking area. Hits are byte ranges in the box's own text, in
//! text order, and never overlap.
//!
//! Matching goes one `char` at a time, not by grapheme: an accent typed
//! as a separate combining mark ("e" + U+0301) is its own character, so
//! it neither matches nor is matched by the precomposed "é", and a query
//! "e" finds the base letter of the decomposed form.

use std::ops::Range;

use crate::checks::walk::visit_order;
use crate::edit::EditError;
use crate::ids::{Item, NodeId};
use crate::model::{Document, MAX_BOX_CHARS};

/// How a query matches text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FindOptions {
    /// Letters must match in case too.
    pub match_case: bool,
    /// The hit must not touch a letter, digit or `_` on either side.
    pub whole_word: bool,
}

/// What [`Document::replace_all`] did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Replaced {
    /// How many hits were replaced.
    pub count: usize,
    /// Whether a box had to be cut to [`MAX_BOX_CHARS`].
    pub cut: bool,
}

/// One match: a byte range in a box's text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FindHit {
    pub node: NodeId,
    pub range: Range<usize>,
}

/// Every box in search order: the main tree in outline order (pre-order,
/// as the canvas and the outline show it), then the parking area.
pub fn search_order(doc: &Document) -> Vec<NodeId> {
    visit_order(doc)
        .into_iter()
        .filter_map(|v| match v.item {
            Item::Box(b) => Some(b),
            Item::Card(_) => None,
        })
        .collect()
}

/// Every hit of `query` in the document, in [`search_order`] and, within a
/// box, in text order. An empty query has no hits.
pub fn find_all(doc: &Document, query: &str, opts: FindOptions) -> Vec<FindHit> {
    if query.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for node in search_order(doc) {
        if let Some(b) = doc.boxes.get(&node) {
            let ranges = find_in_text(&b.text, query, opts);
            out.extend(ranges.into_iter().map(|range| FindHit { node, range }));
        }
    }
    out
}

/// The non-overlapping hits of `query` in `text`, left to right.
///
/// Without match case, text and query are compared one character at a
/// time through `char::to_lowercase`, so the ranges are always ranges of
/// the original text, on character boundaries, even where lowercasing
/// changes a character's length ("İ"). A character whose lowercase is
/// several characters only matches a query character with the same
/// lowercase ("ß" never matches "ss").
fn find_in_text(text: &str, query: &str, opts: FindOptions) -> Vec<Range<usize>> {
    let query: Vec<char> = query.chars().collect();
    if query.is_empty() {
        return Vec::new();
    }
    let same =
        |a: char, b: char| a == b || (!opts.match_case && a.to_lowercase().eq(b.to_lowercase()));
    let mut out = Vec::new();
    let mut from = 0;
    while from < text.len() {
        let mut chars = text[from..].char_indices();
        let mut end = None;
        for (i, &q) in query.iter().enumerate() {
            match chars.next() {
                Some((at, c)) if same(c, q) => {
                    if i + 1 == query.len() {
                        end = Some(from + at + c.len_utf8());
                    }
                }
                _ => break,
            }
        }
        match end {
            Some(end) if !opts.whole_word || is_whole_word(text, from..end) => {
                out.push(from..end);
                from = end;
            }
            _ => from += text[from..].chars().next().map_or(1, char::len_utf8),
        }
    }
    out
}

/// Whether the characters just before and after `range` (if any) are
/// neither alphanumeric nor `_`.
fn is_whole_word(text: &str, range: Range<usize>) -> bool {
    let word = |c: char| c.is_alphanumeric() || c == '_';
    let before = text[..range.start].chars().next_back();
    let after = text[range.end..].chars().next();
    !before.is_some_and(word) && !after.is_some_and(word)
}

/// `text` with each of `ranges` (in order, not overlapping) replaced by
/// `with`, built in one pass that stops at [`MAX_BOX_CHARS`] characters;
/// true when it had to stop there.
fn replaced(text: &str, ranges: &[Range<usize>], with: &str) -> (String, bool) {
    let mut out = String::with_capacity(text.len().min(MAX_BOX_CHARS * 4));
    let mut left = MAX_BOX_CHARS;
    // Appends `part`, or as much as still fits; false once full.
    let mut push = |out: &mut String, part: &str| {
        let end = crate::caps::capped_len(part, left);
        out.push_str(&part[..end]);
        if end < part.len() {
            return false;
        }
        left -= part.chars().count();
        true
    };
    let mut at = 0;
    for range in ranges {
        if !push(&mut out, &text[at..range.start]) || !push(&mut out, with) {
            return (out, true);
        }
        at = range.end;
    }
    let whole = push(&mut out, &text[at..]);
    (out, !whole)
}

impl Document {
    /// Replace the bytes `range` of box `node`'s text with `with`, cut to
    /// [`MAX_BOX_CHARS`]; true when it was cut. `NotFound` when the box
    /// does not exist or the range is out of bounds, reversed or not on
    /// character boundaries.
    pub fn replace_range(
        &mut self,
        node: NodeId,
        range: Range<usize>,
        with: &str,
    ) -> Result<bool, EditError> {
        let b = self.boxes.get_mut(&node).ok_or(EditError::NotFound)?;
        let valid = range.start <= range.end
            && b.text.is_char_boundary(range.start)
            && b.text.is_char_boundary(range.end);
        if !valid {
            return Err(EditError::NotFound);
        }
        let cut;
        (b.text, cut) = replaced(&b.text, &[range], with);
        self.finish_edit();
        Ok(cut)
    }

    /// Replace every hit of `query` with `with`. Each box's hits are found
    /// first and its new text built in one pass, so text just put in is
    /// never searched again; a box that would grow past [`MAX_BOX_CHARS`]
    /// is cut there (see [`Replaced`]).
    ///
    /// Every hit counts, also one whose text already equals `with`. The
    /// app's Replace all reports only the hits that changed, so its count
    /// can be lower.
    pub fn replace_all(&mut self, query: &str, with: &str, opts: FindOptions) -> Replaced {
        let mut done = Replaced::default();
        for node in search_order(self) {
            let Some(b) = self.boxes.get_mut(&node) else {
                continue;
            };
            let ranges = find_in_text(&b.text, query, opts);
            if ranges.is_empty() {
                continue;
            }
            let cut;
            (b.text, cut) = replaced(&b.text, &ranges, with);
            done.count += ranges.len();
            done.cut |= cut;
        }
        if done.count > 0 {
            self.finish_edit();
        }
        done
    }
}

#[cfg(test)]
mod tests;
