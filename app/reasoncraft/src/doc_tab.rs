//! One open document: its content, history, file path and view state.

use std::path::PathBuf;

use reasoncraft_core::{Document, EditError, History, Item, NodeId};

use crate::canvas::Camera;
use crate::i18n::t;

/// How many undo steps a tab keeps.
pub const HISTORY_LIMIT: usize = 200;

pub struct DocTab {
    pub doc: Document,
    pub history: History,
    /// Where the document was last saved or opened from.
    pub path: Option<PathBuf>,
    pub selection: Option<Item>,
    /// The box whose text is being edited inline.
    pub editing: Option<NodeId>,
    pub camera: Camera,
}

impl Default for DocTab {
    fn default() -> Self {
        Self::new()
    }
}

impl DocTab {
    /// A new, clean document with an empty claim, which is selected.
    pub fn new() -> Self {
        let doc = Document::new("");
        let selection = Some(Item::Box(doc.root));
        Self {
            doc,
            history: History::new(HISTORY_LIMIT),
            path: None,
            selection,
            editing: None,
            camera: Camera::default(),
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.history.is_dirty()
    }

    /// The file stem, or "Untitled"; followed by " •" when there are unsaved changes.
    pub fn title(&self) -> String {
        let name = self
            .path
            .as_deref()
            .and_then(|p| p.file_stem())
            .map_or_else(|| t("untitled"), |s| s.to_string_lossy().into_owned());
        if self.is_dirty() {
            format!("{name} •")
        } else {
            name
        }
    }

    /// Run `edit` as one undoable step. A refused edit changes nothing and
    /// leaves no history entry.
    pub fn edit<T>(
        &mut self,
        edit: impl FnOnce(&mut Document) -> Result<T, EditError>,
    ) -> Option<T> {
        let mut draft = self.doc.clone();
        let out = edit(&mut draft).ok()?;
        self.history.record(&self.doc);
        self.doc = draft;
        Some(out)
    }

    pub fn contains(&self, item: Item) -> bool {
        match item {
            Item::Box(b) => self.doc.boxes.contains_key(&b),
            Item::Card(c) => self.doc.cards.contains_key(&c),
        }
    }

    /// Drop a selection or edit target that no longer exists (after undo,
    /// redo or delete), falling back to the root.
    pub fn fix_selection(&mut self) {
        if self.selection.is_some_and(|s| !self.contains(s)) {
            self.selection = Some(Item::Box(self.doc.root));
        }
        if self
            .editing
            .is_some_and(|b| !self.doc.boxes.contains_key(&b))
        {
            self.editing = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reasoncraft_core::{CardKind, Parent};

    #[test]
    fn title_marks_dirty_and_uses_file_stem() {
        let mut tab = DocTab::new();
        assert_eq!(tab.title(), "Untitled");
        let root = tab.doc.root;
        tab.edit(|d| d.add_card(Parent::Box(root), CardKind::Reason, ""))
            .unwrap();
        assert_eq!(tab.title(), "Untitled •");
        tab.path = Some(PathBuf::from("maps/robots.rcraft"));
        tab.history.mark_saved();
        assert_eq!(tab.title(), "robots");
    }

    #[test]
    fn refused_edit_records_no_history() {
        let mut tab = DocTab::new();
        let out =
            tab.edit(|d| d.remove_item(Item::Box(d.root), reasoncraft_core::RemoveMode::Branch));
        assert!(out.is_none());
        assert!(tab.history.undo.is_empty());
        assert!(!tab.is_dirty());
    }
}
