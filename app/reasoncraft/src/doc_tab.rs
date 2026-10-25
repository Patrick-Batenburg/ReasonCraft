//! One open document: its content, history, file path and view state.

use std::path::PathBuf;

use eframe::egui::Rect;
use reasoncraft_core::{Document, EditError, History, Item, NodeId};

use crate::canvas::Camera;
use crate::i18n::t;

/// How many undo steps a tab keeps.
pub const HISTORY_LIMIT: usize = 500;

pub struct DocTab {
    pub doc: Document,
    pub history: History,
    /// Where the document was last saved or opened from.
    pub path: Option<PathBuf>,
    pub selection: Option<Item>,
    /// The box whose text is being edited inline.
    pub editing: Option<NodeId>,
    /// The inline editor's text (mirrors the edited box's text).
    pub edit_text: String,
    /// The document as it was when the inline edit began, recorded as one
    /// history step on commit if the text changed. `None` for a box that was
    /// just added: the add is already the history step.
    pub edit_snapshot: Option<Document>,
    /// The inline editor should take keyboard focus on its next frame.
    pub edit_focus: bool,
    pub camera: Camera,
    /// The screen rect the canvas showed this tab in last frame.
    pub viewport: Option<Rect>,
    /// Fit the map to the view on the next canvas frame (set for new
    /// documents and by Zoom Fit, so the fit uses the fresh layout).
    pub fit_pending: bool,
    /// Pan the selection into view on the next canvas frame (set by
    /// keyboard navigation and adds, so it uses the fresh layout).
    pub reveal_pending: bool,
    /// The box whose comment popover is showing.
    pub comments_open: Option<NodeId>,
}

impl Default for DocTab {
    fn default() -> Self {
        Self::new()
    }
}

impl DocTab {
    /// A new, clean document with an empty claim, which is selected.
    pub fn new() -> Self {
        Self::from_document(Document::new(""))
    }

    /// A clean tab showing `doc`, with its claim selected.
    pub fn from_document(doc: Document) -> Self {
        let selection = Some(Item::Box(doc.root));
        Self {
            doc,
            history: History::new(HISTORY_LIMIT),
            path: None,
            selection,
            editing: None,
            edit_text: String::new(),
            edit_snapshot: None,
            edit_focus: false,
            camera: Camera::default(),
            viewport: None,
            fit_pending: true,
            reveal_pending: false,
            comments_open: None,
        }
    }

    /// Unsaved changes, counting inline-edit text not yet committed.
    pub fn is_dirty(&self) -> bool {
        self.history.is_dirty() || self.has_pending_edit()
    }

    /// The file stem, or "Untitled".
    pub fn name(&self) -> String {
        self.path
            .as_deref()
            .and_then(|p| p.file_stem())
            .map_or_else(|| t("untitled"), |s| s.to_string_lossy().into_owned())
    }

    /// [`Self::name`], followed by " •" when there are unsaved changes.
    pub fn title(&self) -> String {
        let name = self.name();
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

    /// Start an inline edit of box `b` and select it. `fresh`: `b` was just
    /// added, so its first text belongs to the add's history step.
    pub fn begin_edit(&mut self, b: NodeId, fresh: bool) {
        self.commit_edit();
        let Some(node) = self.doc.boxes.get(&b) else {
            return;
        };
        self.edit_text = node.text.clone();
        self.edit_snapshot = (!fresh).then(|| self.doc.clone());
        self.editing = Some(b);
        self.selection = Some(Item::Box(b));
        self.edit_focus = true;
    }

    /// Show the editor's text for box `b` live in the document, without a
    /// history step. Text for a box that is no longer being edited (a late
    /// update after the edit ended) is dropped.
    pub fn set_edit_text(&mut self, b: NodeId, text: String) {
        if self.editing == Some(b) && self.doc.set_text(b, &text).is_ok() {
            self.edit_text = text;
        }
    }

    /// True when committing now would record a history step.
    pub fn has_pending_edit(&self) -> bool {
        match (self.editing, &self.edit_snapshot) {
            (Some(b), Some(before)) => {
                before.boxes.get(&b).map(|n| &n.text) != self.doc.boxes.get(&b).map(|n| &n.text)
            }
            _ => false,
        }
    }

    /// End the inline edit, recording the pre-edit document as one history
    /// step if the text changed.
    pub fn commit_edit(&mut self) {
        if self.has_pending_edit()
            && let Some(before) = self.edit_snapshot.as_ref()
        {
            self.history.record(before);
        }
        self.editing = None;
        self.edit_snapshot = None;
        self.edit_focus = false;
    }

    pub fn contains(&self, item: Item) -> bool {
        match item {
            Item::Box(b) => self.doc.boxes.contains_key(&b),
            Item::Card(c) => self.doc.cards.contains_key(&c),
        }
    }

    /// Drop a selection, edit target or comment popover whose item no
    /// longer exists (after undo, redo or delete); the selection falls back
    /// to the root.
    pub fn fix_selection(&mut self) {
        if self.selection.is_some_and(|s| !self.contains(s)) {
            self.selection = Some(Item::Box(self.doc.root));
        }
        if self
            .editing
            .is_some_and(|b| !self.doc.boxes.contains_key(&b))
        {
            self.editing = None;
            self.edit_snapshot = None;
        }
        if self
            .comments_open
            .is_some_and(|b| !self.doc.boxes.contains_key(&b))
        {
            self.comments_open = None;
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
