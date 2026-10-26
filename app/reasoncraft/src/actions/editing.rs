//! Inline editing of a box's text: starting, live text, typing into the
//! selected box. [`super::apply`] commits a running edit first, so
//! stopping needs nothing more. Text past
//! [`MAX_BOX_CHARS`](crate::canvas::editor::MAX_BOX_CHARS) is cut.

use reasoncraft_core::Item;

use super::Action;
use crate::doc_tab::DocTab;

/// Carry out an editing action; true when the text was cut.
pub(super) fn apply(tab: &mut DocTab, action: Action) -> bool {
    match action {
        Action::Edit => {
            if let Some(Item::Box(b)) = tab.selection {
                tab.begin_edit(b, false);
            }
            false
        }
        // `apply` has already committed the edit.
        Action::StopEditing => false,
        Action::EditText { node, text } => tab.set_edit_text(node, text),
        Action::TypeText(typed) => {
            if tab.editing().is_none()
                && let Some(Item::Box(b)) = tab.selection
            {
                tab.begin_edit(b, false);
            }
            let Some(b) = tab.editing() else {
                return false;
            };
            let text = format!("{}{typed}", tab.edit_text);
            tab.set_edit_text(b, text)
        }
        _ => unreachable!("not an editing action"),
    }
}
