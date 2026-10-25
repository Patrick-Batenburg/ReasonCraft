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

pub(super) fn apply(tab: &mut DocTab, action: Action) {
    match action {
        Action::StartConnect => {
            if let Some(item) = connect_item(tab) {
                tab.connect_mode = Some(item);
                // The arrow keys pick the target in the main tree.
                tab.selection = Some(Item::Box(tab.doc.root));
                tab.reveal_pending = true;
            }
        }
        // `end_unless_kept` has already ended the mode.
        Action::CancelConnect => {}
        _ => unreachable!("not a connect action"),
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

/// End "Connect to…" for every action but picking a target (selecting,
/// navigating, revealing, zooming), starting it again and view preferences
/// (theme, rating display, panels, the Text panel tab and outline switches).
/// A connecting move ends it too. The inline editor's own "stop editing"
/// (sent when it loses focus, e.g. to the ribbon's "Connect to…") keeps it:
/// `apply` committed the edit when the mode started, and Esc reaches here as
/// [`Action::CancelConnect`].
pub(super) fn end_unless_kept(tab: &mut DocTab, action: &Action) {
    let keeps = matches!(
        action,
        Action::Select(_)
            | Action::RevealSelection
            | Action::Navigate(_)
            | Action::JumpParking
            | Action::ZoomIn
            | Action::ZoomOut
            | Action::ZoomFit
            | Action::Zoom100
            | Action::SetZoom(_)
            | Action::StartConnect
            | Action::StopEditing
            | Action::SetTheme(_)
            | Action::TogglePanel(_)
            | Action::ToggleMarkers
            | Action::ToggleShading
            | Action::SetTextTab(_)
            | Action::SetOutlineOption(..)
    );
    if !keeps {
        tab.connect_mode = None;
    }
}
