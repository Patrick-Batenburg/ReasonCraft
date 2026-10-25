//! Inserting from the building panel. A click inserts at the selection,
//! exactly like the matching ribbon command; a drop inserts at the drop
//! target, by pointing the selection at it and running that same command.

use reasoncraft_core::{CardKind, Item, NewItem, NodeId, Parent, SourceType, can_attach};

use super::Action;
use crate::app::ReasonCraftApp;
use crate::canvas::comments::PRESETS;
use crate::doc_tab::DocTab;
use crate::i18n::t;

/// What the building panel offers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Insertable {
    Reason,
    Objection,
    CoPremise,
    Category,
    Note,
    Source(SourceType),
    /// A preset comment, by its index in [`PRESETS`].
    Comment(usize),
}

impl Insertable {
    /// Whether this goes on a node (`on`: co-premise, comment) rather than
    /// under a parent (`to`).
    pub fn goes_on_node(&self) -> bool {
        matches!(self, Insertable::CoPremise | Insertable::Comment(_))
    }

    /// The ribbon command that inserts this at the selection. Comments have
    /// none: they need the selected box (see [`apply`]).
    fn ribbon_action(&self) -> Option<Action> {
        Some(match self {
            Insertable::Reason => Action::AddReason,
            Insertable::Objection => Action::AddObjection,
            Insertable::CoPremise => Action::AddCoPremise,
            Insertable::Category => Action::AddCategory,
            Insertable::Note => Action::AddNote,
            Insertable::Source(t) => Action::AddSource(t.clone()),
            Insertable::Comment(_) => return None,
        })
    }

    /// What attaching this under a parent adds, for [`can_attach`].
    fn new_item(&self) -> Option<NewItem> {
        Some(match self {
            Insertable::Reason => NewItem::Card(CardKind::Reason),
            Insertable::Objection => NewItem::Card(CardKind::Objection),
            Insertable::Category => NewItem::Category,
            Insertable::Note => NewItem::Note,
            Insertable::Source(_) => NewItem::Source,
            Insertable::CoPremise | Insertable::Comment(_) => return None,
        })
    }
}

/// Whether inserting `what` at an explicit target is allowed: under `to`
/// (cards, categories, notes, sources) or on box `on` (a co-premise needs
/// a premise, whose card gets it; a comment any box). Mixing them up, or
/// giving neither, is refused.
pub fn target_allowed(
    doc: &reasoncraft_core::Document,
    what: &Insertable,
    to: Option<Parent>,
    on: Option<NodeId>,
) -> bool {
    match (what, to, on) {
        (Insertable::CoPremise, None, Some(n)) => {
            matches!(doc.owner_of(Item::Box(n)), Some(Parent::Card(_)))
        }
        (Insertable::Comment(i), None, Some(n)) => *i < PRESETS.len() && doc.boxes.contains_key(&n),
        (_, Some(parent), None) => what
            .new_item()
            .is_some_and(|new| can_attach(doc, parent, new)),
        _ => false,
    }
}

/// The box a comment at the selection goes on.
fn selected_box(tab: &DocTab) -> Option<NodeId> {
    match tab.selection? {
        Item::Box(b) => tab.doc.boxes.contains_key(&b).then_some(b),
        Item::Card(_) => None,
    }
}

/// Whether an insert applies now; `None` for other actions.
pub(super) fn is_enabled(tab: &DocTab, action: &Action) -> Option<bool> {
    let Action::Insert { what, to, on } = action else {
        return None;
    };
    Some(match (to, on) {
        (None, None) => match what.ribbon_action() {
            Some(a) => super::is_enabled(tab, &a),
            None => selected_box(tab).is_some(),
        },
        _ => target_allowed(&tab.doc, what, *to, *on),
    })
}

/// Carry out [`Action::Insert`]. A refused insert changes nothing (not
/// even the selection) and records no history.
pub(super) fn apply(app: &mut ReasonCraftApp, action: Action) {
    let Action::Insert { what, to, on } = action else {
        return;
    };
    let Some(tab) = app.tabs.get_mut(app.active) else {
        return;
    };
    if to.is_some() || on.is_some() {
        if !target_allowed(&tab.doc, &what, to, on) {
            return;
        }
        // The ribbon command then inserts at the target.
        tab.selection = match (to, on) {
            (_, Some(n)) | (Some(Parent::Box(n)), None) => Some(Item::Box(n)),
            (Some(Parent::Card(c)), None) => Some(Item::Card(c)),
            (Some(Parent::Parking), None) | (None, None) => None,
        };
    }
    let action = match what {
        Insertable::Comment(i) => {
            let (Some(node), Some(&(key, verdict))) = (selected_box(tab), PRESETS.get(i)) else {
                return;
            };
            Action::AddComment {
                node,
                text: t(key),
                verdict: Some(verdict),
            }
        }
        other => match other.ribbon_action() {
            Some(a) => a,
            None => return,
        },
    };
    super::apply(app, action);
}

#[cfg(test)]
mod tests;
