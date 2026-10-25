//! Adding cards, premises, categories and notes, and deleting items.

use reasoncraft_core::{BoxKind, CardKind, Item, NewLoose, Parent, RemoveMode};

use super::{select_new, selected_card};
use crate::doc_tab::DocTab;

/// Selected box → card under it; selected card → card targeting the whole
/// card; nothing selected → loose card in the parking area.
pub(super) fn add_card(tab: &mut DocTab, kind: CardKind) {
    let premise = match tab.selection {
        Some(Item::Box(b)) => tab
            .edit(|d| d.add_card(Parent::Box(b), kind, ""))
            .map(|(_, p)| p),
        Some(Item::Card(c)) => tab
            .edit(|d| d.add_card(Parent::Card(c), kind, ""))
            .map(|(_, p)| p),
        None => tab.edit(|d| {
            let item = d.add_loose(NewLoose::Card(kind), "");
            Ok(match item {
                Item::Card(c) => d.cards[&c].premises[0],
                Item::Box(b) => b,
            })
        }),
    };
    select_new(tab, premise.map(Item::Box));
}

/// A premise for the selected card, or for the card that owns the selected premise.
pub(super) fn add_co_premise(tab: &mut DocTab) {
    if let Some(c) = selected_card(tab) {
        let p = tab.edit(|d| d.add_premise(c, None, ""));
        select_new(tab, p.map(Item::Box));
    }
}

/// A category or note under the selected box, or loose when nothing is selected.
pub(super) fn add_box(tab: &mut DocTab, kind: BoxKind, loose: NewLoose) {
    let item = match tab.selection {
        Some(Item::Box(b)) => tab
            .edit(|d| d.add_box(Parent::Box(b), kind, ""))
            .map(Item::Box),
        Some(Item::Card(_)) => None,
        None => tab.edit(|d| Ok(d.add_loose(loose, ""))),
    };
    select_new(tab, item);
}

/// Remove the selection, then select what held it (or the root).
pub(super) fn delete(tab: &mut DocTab, mode: RemoveMode) {
    let Some(item) = tab.selection else { return };
    let owner = tab.doc.owner_of(item);
    if tab.edit(|d| d.remove_item(item, mode)).is_none() {
        return;
    }
    tab.selection = match owner {
        Some(Parent::Box(b)) => Some(Item::Box(b)),
        Some(Parent::Card(c)) => Some(Item::Card(c)),
        Some(Parent::Parking) | None => None,
    };
    tab.editing = None;
    tab.fix_selection();
}
