//! "Move to…" (the command once called "Connect to…"): pick where the
//! selected item goes by clicking a box or card, or by selecting one with
//! the arrow keys and pressing Enter; and "Move to parking".

use reasoncraft_core::{Item, Parent};

use super::Action;
use crate::doc_tab::DocTab;

/// The item "Move to…" would move for the selection: the selected card or
/// box, or the card of a selected premise (a premise moves with its
/// card), wherever it is: in the map or the parking area, at the top or
/// deeper. Never the root claim.
pub fn connect_item(tab: &DocTab) -> Option<Item> {
    let sel = tab.selection?;
    match (sel, tab.doc.owner_of(sel)?) {
        (Item::Box(_), Parent::Card(c)) => Some(Item::Card(c)),
        _ => Some(sel),
    }
}

/// Start "Move to…" for the selected item: the arrow keys then pick the
/// target, from the root on. (`apply` ends it for every action that does
/// not keep it, Esc's [`super::Action::CancelConnect`] too.) The rules of
/// a drag apply: a target that refuses the item, or one inside its own
/// branch, is greyed out.
pub(super) fn start(tab: &mut DocTab) {
    if let Some(item) = connect_item(tab) {
        tab.connect_mode = Some(item);
        tab.selection = Some(Item::Box(tab.doc.root));
        tab.reveal_pending = true;
    }
}

/// The move "Move to parking" makes for the selection: its item (see
/// [`connect_item`]) to the end of the parking area, unless it is at the
/// top of the parking area already.
pub(super) fn park_move(tab: &DocTab) -> Option<Action> {
    let item = connect_item(tab)?;
    (tab.doc.owner_of(item)? != Parent::Parking).then_some(Action::Move {
        item,
        to: Parent::Parking,
        index: None,
    })
}

/// `Some` for the actions this module handles.
pub(super) fn is_enabled(tab: &DocTab, action: &Action) -> Option<bool> {
    match action {
        Action::StartConnect => Some(connect_item(tab).is_some()),
        Action::CancelConnect => Some(tab.connect_mode.is_some()),
        Action::MoveToParking => Some(park_move(tab).is_some()),
        _ => None,
    }
}
