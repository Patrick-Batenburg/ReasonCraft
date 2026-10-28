//! Whether a command would do something right now.

use reasoncraft_core::{CardKind, Item, NewItem, Parent, can_attach};

use super::{
    Action, box_group, comments, connect, evaluate, find, images, insert, selected_card, sources,
    structure,
};
use crate::app::ReasonCraftApp;
use crate::doc_tab::DocTab;
use reasoncraft_core::nav::parking_jump;

/// Whether `action` may run in `app` right now: enabled for the active
/// tab, and the spelling pass (F7) only with spellcheck on. The ribbon,
/// Search commands and the keyboard all ask this, so they agree.
pub fn can_run(app: &ReasonCraftApp, action: &Action) -> bool {
    app.tabs
        .get(app.active)
        .is_some_and(|tab| is_enabled(tab, action))
        && (*action != Action::OpenSpelling || app.settings.spellcheck)
}

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
        .or_else(|| images::is_enabled(tab, action))
        .or_else(|| find::is_enabled(tab, action))
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
        Action::ZoomFitSelection => sel.is_some(),
        Action::Undo => !tab.history.undo.is_empty() || tab.has_pending_session(),
        // Redo first ends the edit session, and one with text to record
        // clears the redo steps.
        Action::Redo => !tab.history.redo.is_empty() && !tab.has_pending_session(),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::apply;

    /// Redo first ends the running edit session, and a session with text
    /// to record clears the redo steps: Redo would do nothing, so it is
    /// greyed out.
    #[test]
    fn redo_off_while_an_edit_would_clear_it() {
        let mut a = ReasonCraftApp::new_for_test();
        apply(&mut a, Action::AddReason);
        apply(&mut a, Action::Undo);
        let root = a.tabs[0].doc.root;
        assert!(is_enabled(&a.tabs[0], &Action::Redo));
        apply(&mut a, Action::Edit);
        apply(
            &mut a,
            Action::EditText {
                node: root,
                text: "Changed".into(),
            },
        );
        assert!(a.tabs[0].has_pending_session());
        assert!(!is_enabled(&a.tabs[0], &Action::Redo));
        apply(&mut a, Action::Redo);
        assert!(a.tabs[0].history.redo.is_empty(), "the edit cleared redo");
    }
}
