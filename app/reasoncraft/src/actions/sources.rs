//! Adding typed sources and changing a source's type.

use reasoncraft_core::{BoxKind, Item, NewItem, NewLoose, NodeId, Parent, can_attach};

use super::{Action, select_new};
use crate::doc_tab::DocTab;

/// Where a new source goes: under the selected box if it takes sources,
/// the parking area when nothing is selected, nowhere otherwise.
fn source_target(tab: &DocTab) -> Option<Parent> {
    match tab.selection {
        None => Some(Parent::Parking),
        Some(Item::Box(b)) => {
            can_attach(&tab.doc, Parent::Box(b), NewItem::Source).then_some(Parent::Box(b))
        }
        Some(Item::Card(_)) => None,
    }
}

/// The selected source box, if a source is selected.
fn selected_source(tab: &DocTab) -> Option<NodeId> {
    match tab.selection? {
        Item::Box(b) => matches!(tab.doc.boxes.get(&b)?.kind, BoxKind::Source { .. }).then_some(b),
        Item::Card(_) => None,
    }
}

/// Whether a source action applies now; `None` for other actions.
pub(super) fn is_enabled(tab: &DocTab, action: &Action) -> Option<bool> {
    Some(match action {
        Action::AddSource(_) => source_target(tab).is_some(),
        Action::SetSourceType(_) => selected_source(tab).is_some(),
        _ => return None,
    })
}

/// Carry out a source action. A refused add, or a type change to the type
/// the source already has, changes nothing and records no history.
pub(super) fn apply(tab: &mut DocTab, action: Action) {
    match action {
        Action::AddSource(source_type) => {
            let item = match source_target(tab) {
                Some(Parent::Parking) => {
                    tab.edit(|d| Ok(d.add_loose(NewLoose::Source(source_type), "")))
                }
                Some(parent) => {
                    let kind = BoxKind::Source { source_type };
                    tab.edit(|d| d.add_box(parent, kind, "")).map(Item::Box)
                }
                None => None,
            };
            select_new(tab, item);
        }
        Action::SetSourceType(t) => {
            let Some(b) = selected_source(tab) else {
                return;
            };
            let current = BoxKind::Source {
                source_type: t.clone(),
            };
            if tab.doc.boxes[&b].kind != current {
                tab.edit(|d| d.set_source_type(b, t));
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests;
