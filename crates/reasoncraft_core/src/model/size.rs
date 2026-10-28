//! A rough count of the memory a document takes, so undo history can stay
//! within a budget (see [`crate::History`]).

use std::mem::size_of;

use serde_json::{Map, Value};

use super::{BoxNode, Card, Document};

/// Bytes counted for each entry of an index map, beside its key and value.
const MAP_ENTRY: usize = 2 * size_of::<usize>();

impl Document {
    /// About how many bytes the document takes in memory: its structs,
    /// text and lists. An estimate (allocator overhead is not counted),
    /// found in one pass without copying anything.
    pub fn approx_bytes(&self) -> usize {
        let meta = &self.meta;
        let mut n = size_of::<Self>()
            + meta.title.capacity()
            + meta.author.capacity()
            + meta.created.capacity()
            + meta.modified.capacity()
            + meta.language.capacity()
            + extra_bytes(&self.extra);
        n += self.boxes.values().map(box_bytes).sum::<usize>();
        n += self.cards.values().map(card_bytes).sum::<usize>();
        n += self.parking.capacity() * size_of::<crate::Item>();
        for s in &self.snippets {
            n += size_of_val(s) + s.text.capacity() + extra_bytes(&s.extra);
            if let Some(src) = &s.source {
                n += src.citation.capacity()
                    + src.link.as_ref().map_or(0, String::capacity)
                    + extra_bytes(&src.extra);
            }
        }
        let essay = &self.essay;
        n += essay
            .paragraphs
            .iter()
            .map(|(k, v)| MAP_ENTRY + 2 * size_of::<String>() + k.capacity() + v.capacity())
            .sum::<usize>();
        n += essay
            .unplaced
            .iter()
            .map(|(l, t)| 2 * size_of::<String>() + l.capacity() + t.capacity())
            .sum::<usize>();
        n += extra_bytes(&essay.extra);
        n += self
            .ignored_hints
            .iter()
            .map(|h| {
                size_of_val(h)
                    + h.kind.capacity()
                    + h.terms.iter().map(String::capacity).sum::<usize>()
            })
            .sum::<usize>();
        n
    }
}

fn box_bytes(b: &BoxNode) -> usize {
    MAP_ENTRY
        + size_of::<crate::NodeId>()
        + size_of::<BoxNode>()
        + b.text.capacity()
        + b.children.capacity() * size_of::<crate::Item>()
        + b.link.as_ref().map_or(0, String::capacity)
        + b.image.as_ref().map_or(0, |i| i.asset.capacity())
        + b.comments
            .iter()
            .map(|c| {
                size_of_val(c) + c.author.capacity() + c.text.capacity() + c.created.capacity()
            })
            .sum::<usize>()
        + extra_bytes(&b.extra)
}

fn card_bytes(c: &Card) -> usize {
    MAP_ENTRY
        + size_of::<crate::CardId>()
        + size_of::<Card>()
        + c.premises.capacity() * size_of::<crate::NodeId>()
        + c.children.capacity() * size_of::<crate::CardId>()
        + extra_bytes(&c.extra)
}

/// Unknown fields: their size as JSON (they are rare, and small: a file's
/// unknown fields are dropped past 1 MiB in all).
fn extra_bytes(extra: &Map<String, Value>) -> usize {
    if extra.is_empty() {
        return 0;
    }
    serde_json::to_vec(extra).map_or(0, |v| v.len())
}
