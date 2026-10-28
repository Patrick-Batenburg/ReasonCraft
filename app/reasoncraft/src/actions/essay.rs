//! The essay: live paragraph text and removing unplaced notes.

use crate::doc_tab::DocTab;

/// Delete the unplaced essay note at `index` (a step of its own; typing
/// a paragraph goes through [`DocTab::set_essay_text_live`]).
pub(super) fn remove_unplaced(tab: &mut DocTab, index: usize) {
    tab.edit(|d| d.remove_unplaced(index));
}
