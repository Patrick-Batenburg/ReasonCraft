//! Dropping text (a snippet selection, or a whole snippet) onto the map.
//!
//! On a claim or premise the text becomes the premise of a new reason
//! card under it; on a card, a reason against the whole card; on empty
//! canvas, a loose reason card. A source tagged on the text goes under
//! the new premise, in the same history step. On a category the text
//! becomes a new category under it, without the source: categories take
//! no sources. Blank text, notes and sources are refused.

use reasoncraft_core::{
    BoxKind, CardKind, Document, Item, NewItem, NewLoose, NodeId, Parent, SnippetSource, can_attach,
};

use crate::doc_tab::DocTab;

/// What text dropped on `to` becomes, for [`can_attach`].
fn new_item(doc: &Document, to: Parent) -> Option<NewItem> {
    Some(match to {
        Parent::Box(b) => match doc.boxes.get(&b)?.kind {
            BoxKind::Claim | BoxKind::Premise => NewItem::Card(CardKind::Reason),
            BoxKind::Category => NewItem::Category,
            BoxKind::Source { .. } | BoxKind::Note => return None,
        },
        Parent::Card(_) | Parent::Parking => NewItem::Card(CardKind::Reason),
    })
}

/// Whether `text` may be dropped on `to`.
pub(super) fn allowed(doc: &Document, text: &str, to: Parent) -> bool {
    !text.trim().is_empty() && new_item(doc, to).is_some_and(|new| can_attach(doc, to, new))
}

/// Add `text` (trimmed) at `to`, with `source` under the new premise, as
/// one history step. Returns the new box (the premise, or the category);
/// `None` when refused, with nothing changed.
pub(super) fn insert(
    tab: &mut DocTab,
    text: &str,
    source: Option<SnippetSource>,
    to: Parent,
) -> Option<NodeId> {
    if !allowed(&tab.doc, text, to) {
        return None;
    }
    let text = text.trim();
    let category = matches!(new_item(&tab.doc, to), Some(NewItem::Category));
    tab.edit(|d| {
        if category {
            return d.add_box(to, BoxKind::Category, text);
        }
        let premise = match to {
            Parent::Parking => match d.add_loose(NewLoose::Card(CardKind::Reason), text) {
                Item::Card(c) => d.cards[&c].premises[0],
                Item::Box(b) => b,
            },
            _ => d.add_card(to, CardKind::Reason, text)?.1,
        };
        if let Some(src) = source {
            let kind = BoxKind::Source {
                source_type: src.source_type,
            };
            let s = d.add_box(Parent::Box(premise), kind, &src.citation)?;
            d.set_link(s, src.link)?;
        }
        Ok(premise)
    })
}

#[cfg(test)]
mod tests;
