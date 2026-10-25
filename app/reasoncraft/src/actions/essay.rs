//! The essay: live paragraph text and removing unplaced notes.

use super::Action;
use crate::doc_tab::DocTab;

/// Carry out an essay action. Typing a paragraph shows live and becomes
/// one history step when its session ends; removing an unplaced note is a
/// step of its own.
pub(super) fn apply(tab: &mut DocTab, action: Action) {
    match action {
        Action::EditEssayText { slot, text } => tab.set_essay_text_live(slot, &text),
        Action::RemoveUnplaced(index) => {
            tab.edit(|d| d.remove_unplaced(index));
        }
        _ => unreachable!("not an essay action"),
    }
}
