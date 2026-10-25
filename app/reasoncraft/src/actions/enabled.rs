//! Whether a command would do something right now.

use reasoncraft_core::{CardKind, Item, NewItem, Parent, can_attach};

use super::{
    Action, box_group, comments, connect, evaluate, insert, selected_card, sources, structure,
};
use crate::canvas::input::parking_jump;
use crate::doc_tab::DocTab;

/// Whether `action` would do something for `tab` right now. Ribbon, "+"
/// menu and keyboard all ask this, so refused commands are greyed out
/// instead of silently doing nothing.
pub fn is_enabled(tab: &DocTab, action: &Action) -> bool {
    if let Some(enabled) = evaluate::is_enabled(tab, action)
        .or_else(|| sources::is_enabled(tab, action))
        .or_else(|| comments::is_enabled(tab, action))
        .or_else(|| insert::is_enabled(tab, action))
        .or_else(|| structure::is_enabled(tab, action))
        .or_else(|| connect::is_enabled(tab, action))
        .or_else(|| box_group::is_enabled(tab, action))
    {
        return enabled;
    }
    let sel = tab.selection;
    let attach = |new: NewItem| match sel {
        Some(Item::Box(b)) => can_attach(&tab.doc, Parent::Box(b), new),
        Some(Item::Card(c)) => can_attach(&tab.doc, Parent::Card(c), new),
        None => true,
    };
    match action {
        Action::AddReason => attach(NewItem::Card(CardKind::Reason)),
        Action::AddObjection => attach(NewItem::Card(CardKind::Objection)),
        Action::AddCategory => attach(NewItem::Category),
        Action::AddNote => attach(NewItem::Note),
        Action::AddCoPremise => selected_card(tab).is_some(),
        Action::TargetWholeCard(kind) => selected_card(tab)
            .is_some_and(|c| can_attach(&tab.doc, Parent::Card(c), NewItem::Card(kind.clone()))),
        Action::Delete | Action::DeleteBranch => sel.is_some_and(|s| s != Item::Box(tab.doc.root)),
        Action::Edit => matches!(sel, Some(Item::Box(_))),
        Action::JumpParking => parking_jump(&tab.doc, sel).is_some(),
        Action::Undo => !tab.history.undo.is_empty() || tab.has_pending_edit(),
        Action::Redo => !tab.history.redo.is_empty(),
        _ => true,
    }
}
