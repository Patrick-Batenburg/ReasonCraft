//! Inline editing of a box's text: starting, live text, typing into the
//! selected box. [`super::apply`] commits a running edit first, so
//! stopping needs nothing more. Text past
//! [`MAX_BOX_CHARS`](reasoncraft_core::MAX_BOX_CHARS) is cut.

use reasoncraft_core::Item;

use crate::doc_tab::DocTab;

/// Start editing the selected box's text.
pub(super) fn start(tab: &mut DocTab) {
    if let Some(Item::Box(b)) = tab.selection {
        tab.begin_edit(b, false);
    }
}

/// Text typed while a box is selected: start editing it (if needed) and
/// append; true when the text was cut.
pub(super) fn type_text(tab: &mut DocTab, typed: &str) -> bool {
    if tab.editing().is_none() {
        start(tab);
    }
    let Some(b) = tab.editing() else {
        return false;
    };
    let text = format!("{}{typed}", tab.edit_text);
    tab.set_edit_text(b, text)
}
