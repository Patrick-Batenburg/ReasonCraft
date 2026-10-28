//! "Connect to…": pick where a parked item goes by clicking a box or card,
//! or by selecting one with the arrow keys and pressing Enter.

use reasoncraft_core::{Item, Parent};

use super::Action;
use crate::doc_tab::DocTab;

/// The parked item "Connect to…" would move for the selection: a top-level
/// parking item, or the card of a selected premise when that card is a
/// top-level parking item. Items nested deeper inside a parked fragment are
/// not offered (drag them instead).
pub fn connect_item(tab: &DocTab) -> Option<Item> {
    let sel = tab.selection?;
    let item = match (sel, tab.doc.owner_of(sel)?) {
        (_, Parent::Parking) => sel,
        (Item::Box(_), Parent::Card(c)) => Item::Card(c),
        _ => return None,
    };
    (tab.doc.owner_of(item) == Some(Parent::Parking)).then_some(item)
}

/// Start "Connect to…" for the selected parked item: the arrow keys then
/// pick the target in the main tree. (`apply` ends it for every action
/// that does not keep it, Esc's [`super::Action::CancelConnect`] too.)
pub(super) fn start(tab: &mut DocTab) {
    if let Some(item) = connect_item(tab) {
        tab.connect_mode = Some(item);
        tab.selection = Some(Item::Box(tab.doc.root));
        tab.reveal_pending = true;
    }
}

/// `Some` for the actions this module handles.
pub(super) fn is_enabled(tab: &DocTab, action: &Action) -> Option<bool> {
    match action {
        Action::StartConnect => Some(connect_item(tab).is_some()),
        Action::CancelConnect => Some(tab.connect_mode.is_some()),
        _ => None,
    }
}
