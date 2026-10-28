//! Essay paragraph sessions and the Essay tab's cached view of the map.
//!
//! Like snippet editing, typing a paragraph shows live in the document and
//! becomes one history step when the session ends. Only the paragraph of
//! the session's slot counts: the document's modified time moving on is
//! not a change to record.

use std::collections::{HashMap, HashSet};

use reasoncraft_core::essay::{EssaySection, EssaySlot, essay_order};
use reasoncraft_core::{CardStrength, Document, Item, NodeId};

use super::{DocTab, EditSession};

/// What the Essay tab shows, rebuilt once per document revision.
#[derive(Clone, Debug, PartialEq)]
pub struct EssayView {
    /// The sections in essay order (see [`essay_order`]).
    pub sections: Vec<EssaySection>,
    /// Items that have a paragraph but are not in the essay right now
    /// (parked, or no longer a premise or source, e.g. turned into a note),
    /// with the item's text as a label. Their paragraphs are kept and count
    /// again when the item returns to the essay.
    pub inactive: Vec<(Item, String)>,
    /// The strength of the card each premise belongs to, for its colour.
    pub premise_strength: HashMap<NodeId, Option<CardStrength>>,
}

/// Whether the paragraph for `slot` differs between `before` and `now`.
pub(super) fn pending(before: &Document, now: &Document, slot: EssaySlot) -> bool {
    before.essay.text(slot) != now.essay.text(slot)
}

/// The text an item is known by: a box's own text, a card's first premise.
fn item_label(doc: &Document, item: Item) -> Option<String> {
    let b = match item {
        Item::Box(b) => b,
        Item::Card(c) => doc.cards.get(&c)?.premises.first().copied()?,
    };
    doc.boxes.get(&b).map(|n| n.text.clone())
}

/// The view of `doc` for the Essay tab.
fn build_view(doc: &Document) -> EssayView {
    let sections = essay_order(doc);
    let active: HashSet<EssaySlot> = sections
        .iter()
        .flat_map(|s| s.elements.iter().map(|e| e.slot))
        .collect();
    let exists = |item: Item| match item {
        Item::Box(b) => doc.boxes.contains_key(&b),
        Item::Card(c) => doc.cards.contains_key(&c),
    };
    // Keys this build cannot read, and keys of items that do not exist
    // (only from hand-edited files), stay in the file but are not shown.
    let inactive = doc
        .essay
        .paragraphs
        .keys()
        .filter_map(|key| match EssaySlot::from_key(key)? {
            slot @ EssaySlot::Item(item) if !active.contains(&slot) && exists(item) => {
                Some((item, item_label(doc, item).unwrap_or_default()))
            }
            _ => None,
        })
        .collect();
    let premise_strength = doc
        .cards
        .values()
        .flat_map(|c| c.premises.iter().map(|p| (*p, c.strength)))
        .collect();
    EssayView {
        sections,
        inactive,
        premise_strength,
    }
}

impl DocTab {
    /// The Essay tab's view of the document, rebuilt only when the
    /// document changed since the last call.
    pub fn essay(&mut self) -> &EssayView {
        let doc = &self.doc;
        self.essay_cache.get(self.revision, || build_view(doc))
    }

    /// Show `text` live as the paragraph for `slot`, without a history
    /// step; the session for `slot` starts if it is not running. Card
    /// slots and slots of boxes that do not exist are not writable and
    /// start nothing.
    pub fn set_essay_text_live(&mut self, slot: EssaySlot, text: &str) {
        let writable = match slot {
            EssaySlot::Item(Item::Card(_)) => false,
            EssaySlot::Item(Item::Box(b)) => self.doc.boxes.contains_key(&b),
            EssaySlot::Intro | EssaySlot::Conclusion => true,
        };
        if !writable {
            return;
        }
        let this = EditSession::Essay { slot };
        if self.session() != Some(&this) {
            self.begin_session(this);
        }
        let old = self.doc.essay.text(slot).map(str::to_owned);
        if self.doc.set_essay_text(slot, text).is_ok()
            && self.doc.essay.text(slot) != old.as_deref()
        {
            self.bump_revision();
        }
    }
}
