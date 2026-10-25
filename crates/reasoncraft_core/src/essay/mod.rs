//! The essay plan: text written for the parts of the map.
//!
//! Paragraphs are stored by slot key (see [`EssaySlot::key`]). A key whose
//! item no longer exists (for example from a hand-edited file) is harmless:
//! it stays in the file and is ignored by [`essay_order`] and the exports.
//! When an edit removes an item that has a paragraph, the paragraph moves
//! to [`EssayPlan::unplaced`] instead of being lost.

mod order;
mod text;

use indexmap::IndexMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::edit::EditError;
use crate::ids::{CardId, Item, NodeId};
use crate::model::Document;
use crate::snippets::capped_len;

pub use order::{ElementRole, EssayElement, EssaySection, essay_order};
pub use text::{essay_markdown, essay_plain_text};

/// The most characters one essay paragraph keeps.
pub const MAX_ESSAY_CHARS: usize = 50_000;

/// How many characters of an item's text label an unplaced note.
const LABEL_CHARS: usize = 60;

/// One place in the essay that holds text: the introduction, the
/// conclusion, or the paragraph for a box or card of the map.
///
/// Serialized as its key: `"intro"`, `"conclusion"`, `"box:<uuid>"` or
/// `"card:<uuid>"`. Boxes and cards get distinct prefixes so a key always
/// names one item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EssaySlot {
    Intro,
    Conclusion,
    Item(Item),
}

impl EssaySlot {
    /// The key this slot's paragraph is stored under.
    pub fn key(&self) -> String {
        match self {
            EssaySlot::Intro => "intro".to_owned(),
            EssaySlot::Conclusion => "conclusion".to_owned(),
            EssaySlot::Item(Item::Box(b)) => format!("box:{}", b.0),
            EssaySlot::Item(Item::Card(c)) => format!("card:{}", c.0),
        }
    }

    /// The slot for `key`, or `None` for a key this build does not know.
    pub fn from_key(key: &str) -> Option<EssaySlot> {
        match key {
            "intro" => return Some(EssaySlot::Intro),
            "conclusion" => return Some(EssaySlot::Conclusion),
            _ => {}
        }
        let (kind, id) = key.split_once(':')?;
        let id = Uuid::parse_str(id).ok()?;
        match kind {
            "box" => Some(EssaySlot::Item(Item::Box(NodeId(id)))),
            "card" => Some(EssaySlot::Item(Item::Card(CardId(id)))),
            _ => None,
        }
    }
}

impl Serialize for EssaySlot {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.key())
    }
}

impl<'de> Deserialize<'de> for EssaySlot {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let key = String::deserialize(d)?;
        EssaySlot::from_key(&key)
            .ok_or_else(|| serde::de::Error::custom(format!("unknown essay slot {key:?}")))
    }
}

/// The written essay: paragraphs by slot key, and notes whose item was
/// removed, as `(label, text)` with the label taken from the item's text.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EssayPlan {
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub paragraphs: IndexMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unplaced: Vec<(String, String)>,
    /// Unknown fields, kept so files roundtrip.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl EssayPlan {
    /// Nothing written and nothing unknown: the field is left out of files.
    pub fn is_empty(&self) -> bool {
        self.paragraphs.is_empty() && self.unplaced.is_empty() && self.extra.is_empty()
    }

    /// The paragraph written for `slot`, if any.
    pub fn text(&self, slot: EssaySlot) -> Option<&str> {
        self.paragraphs.get(&slot.key()).map(String::as_str)
    }
}

/// Shown for an unplaced note whose item had no text.
const EMPTY_LABEL: &str = "\u{2026}";

/// `text` on one line (runs of whitespace become one space), cut to
/// [`LABEL_CHARS`] characters; [`EMPTY_LABEL`] when nothing is left.
fn label_of(text: &str) -> String {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut label = flat[..capped_len(&flat, LABEL_CHARS)].to_owned();
    label.truncate(label.trim_end().len());
    if label.is_empty() {
        EMPTY_LABEL.to_owned()
    } else {
        label
    }
}

impl Document {
    /// Set the paragraph for `slot`, capped at [`MAX_ESSAY_CHARS`]; an empty
    /// text removes it. Unchanged text is a no-op (the modified time stays).
    /// A slot for a box that does not exist is refused; card slots are not
    /// writable (they only survive from files and are salvaged on removal).
    pub fn set_essay_text(&mut self, slot: EssaySlot, text: &str) -> Result<(), EditError> {
        match slot {
            EssaySlot::Item(Item::Card(_)) => return Err(EditError::NotAllowed),
            EssaySlot::Item(Item::Box(b)) if !self.boxes.contains_key(&b) => {
                return Err(EditError::NotFound);
            }
            _ => {}
        }
        let text = &text[..capped_len(text, MAX_ESSAY_CHARS)];
        let key = slot.key();
        let current = self.essay.paragraphs.get(&key).map_or("", String::as_str);
        if current == text {
            return Ok(());
        }
        if text.is_empty() {
            self.essay.paragraphs.shift_remove(&key);
        } else {
            self.essay.paragraphs.insert(key, text.to_owned());
        }
        self.finish_essay_edit();
        Ok(())
    }

    /// Delete unplaced note `index`.
    pub fn remove_unplaced(&mut self, index: usize) -> Result<(), EditError> {
        if index >= self.essay.unplaced.len() {
            return Err(EditError::NotFound);
        }
        self.essay.unplaced.remove(index);
        self.finish_essay_edit();
        Ok(())
    }

    fn finish_essay_edit(&mut self) {
        self.touch();
        #[cfg(debug_assertions)]
        self.check_invariants();
    }

    /// Move the paragraphs of `removed` to the unplaced notes. Call before
    /// the items leave the maps, so the labels can read their text: a box's
    /// own text, a card's first premise.
    pub(crate) fn salvage_essay_text(&mut self, removed: &[Item]) {
        if self.essay.paragraphs.is_empty() {
            return;
        }
        for &item in removed {
            let Some(text) = self
                .essay
                .paragraphs
                .shift_remove(&EssaySlot::Item(item).key())
            else {
                continue;
            };
            let label_box = match item {
                Item::Box(b) => Some(b),
                Item::Card(c) => self.cards.get(&c).and_then(|c| c.premises.first().copied()),
            };
            let node = label_box.and_then(|b| self.boxes.get(&b));
            let label = label_of(node.map_or("", |n| n.text.as_str()));
            self.essay.unplaced.push((label, text));
        }
    }

    /// Cut every paragraph and unplaced note to [`MAX_ESSAY_CHARS`] and
    /// every unplaced label to [`LABEL_CHARS`], as editing does, so an
    /// oversized text from a file never reaches the UI. Not an edit.
    pub(crate) fn cap_essay_texts(&mut self) {
        fn cap(text: &mut String, max: usize) {
            let end = capped_len(text, max);
            text.truncate(end);
        }
        let plan = &mut self.essay;
        for text in plan.paragraphs.values_mut() {
            cap(text, MAX_ESSAY_CHARS);
        }
        for (label, text) in &mut plan.unplaced {
            cap(label, LABEL_CHARS);
            cap(text, MAX_ESSAY_CHARS);
        }
    }
}

#[cfg(test)]
mod tests;
