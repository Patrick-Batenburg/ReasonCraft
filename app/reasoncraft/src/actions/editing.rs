//! Inline editing of a box's text: starting, live text, typing into the
//! selected box. [`super::apply`] commits a running edit first, so
//! stopping needs nothing more.

use reasoncraft_core::Item;

use super::Action;
use crate::doc_tab::DocTab;

pub(super) fn apply(tab: &mut DocTab, action: Action) {
    match action {
        Action::Edit => {
            if let Some(Item::Box(b)) = tab.selection {
                tab.begin_edit(b, false);
            }
        }
        // `apply` has already committed the edit.
        Action::StopEditing => {}
        Action::EditText { node, text } => tab.set_edit_text(node, text),
        Action::TypeText(typed) => {
            if tab.editing.is_none()
                && let Some(Item::Box(b)) = tab.selection
            {
                tab.begin_edit(b, false);
            }
            if let Some(b) = tab.editing {
                let text = format!("{}{typed}", tab.edit_text);
                tab.set_edit_text(b, text);
            }
        }
        _ => unreachable!("not an editing action"),
    }
}
