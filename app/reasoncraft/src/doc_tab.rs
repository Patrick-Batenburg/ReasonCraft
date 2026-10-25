//! One open document: its content, history, file path and view state.

use std::path::PathBuf;

use eframe::egui::Rect;
use reasoncraft_core::outline::{OutlineLine, OutlineOptions, build_outline};
use reasoncraft_core::{Assets, Document, EditError, History, Item, NodeId, Package};

use crate::canvas::Camera;
use crate::i18n::t;

/// How many undo steps a tab keeps.
pub const HISTORY_LIMIT: usize = 500;

pub struct DocTab {
    pub doc: Document,
    pub history: History,
    /// The image bytes the document's boxes refer to, kept outside
    /// [`History`] so undo snapshots never hold bytes. Add-only while the
    /// tab is open: an undone image insert leaves its bytes unused, and a
    /// save writes only the referenced ones.
    pub assets: Assets,
    /// Zip entries of the opened file that this build does not understand,
    /// written back unchanged on save.
    pub other_entries: Vec<(String, Vec<u8>)>,
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
    /// The parked item waiting for a target ("Connect to…").
    pub connect_mode: Option<Item>,
    /// Counts changes to [`Self::doc`], so text derived from it (outline,
    /// checks) can be cached until the next change. Starts at 0.
    revision: u64,
    /// The outline last built, with the revision and options it was built
    /// for (see [`Self::outline`]).
    pub outline_cache: Option<(u64, OutlineOptions, Vec<OutlineLine>)>,
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
        Self::from_package(Package::new(doc))
    }

    /// A clean tab showing an opened file's document, with its assets and
    /// unknown entries.
    pub fn from_package(pkg: Package) -> Self {
        let Package {
            doc,
            assets,
            other_entries,
        } = pkg;
        let selection = Some(Item::Box(doc.root));
        Self {
            doc,
            history: History::new(HISTORY_LIMIT),
            assets,
            other_entries,
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
            connect_mode: None,
            revision: 0,
            outline_cache: None,
        }
    }

    /// The document's change count: it differs from an earlier value
    /// whenever the document may have changed since.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Note a change to the document (edits, undo and redo, inline text,
    /// a file loaded into the tab). Selection and view changes do not count.
    pub fn bump_revision(&mut self) {
        self.revision += 1;
    }

    /// The document's outline for `opts`, rebuilt only when the document or
    /// the options changed since the last call.
    pub fn outline(&mut self, opts: OutlineOptions) -> &[OutlineLine] {
        let fresh = self
            .outline_cache
            .as_ref()
            .is_some_and(|(rev, o, _)| *rev == self.revision && *o == opts);
        if !fresh {
            let lines = build_outline(&self.doc, &opts);
            self.outline_cache = Some((self.revision, opts, lines));
        }
        self.outline_cache.as_ref().map_or(&[], |(_, _, l)| l)
    }

    /// Note that a file was loaded into this tab, in place of a tab at
    /// revision `replaced` (0 for none). The new revision is past both, so
    /// nothing cached for the replaced document is reused.
    pub fn mark_loaded(&mut self, replaced: u64) {
        self.revision = self.revision.max(replaced) + 1;
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
        self.bump_revision();
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
        if self.editing != Some(b) {
            return;
        }
        let changed = self.doc.boxes.get(&b).is_some_and(|n| n.text != text);
        if self.doc.set_text(b, &text).is_ok() {
            self.edit_text = text;
            if changed {
                self.bump_revision();
            }
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
            self.bump_revision();
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

    /// Undo one step, if there is one (a new revision), and fix the selection.
    pub fn undo(&mut self) {
        if self.history.undo(&mut self.doc) {
            self.bump_revision();
        }
        self.fix_selection();
    }

    /// Redo one step, if there is one (a new revision), and fix the selection.
    pub fn redo(&mut self) {
        if self.history.redo(&mut self.doc) {
            self.bump_revision();
        }
        self.fix_selection();
    }

    /// Drop a selection, edit target, comment popover or connect mode whose item no
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
        if self.connect_mode.is_some_and(|i| !self.contains(i)) {
            self.connect_mode = None;
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
    fn outline_is_rebuilt_only_for_new_revision_or_options() {
        let mut tab = DocTab::new();
        let opts = OutlineOptions::default();
        assert_eq!(tab.outline(opts).len(), 1);
        // A change made behind the cache's back is not seen…
        let root = tab.doc.root;
        tab.doc
            .add_card(Parent::Box(root), CardKind::Reason, "")
            .unwrap();
        assert_eq!(tab.outline(opts).len(), 1);
        // …until the revision moves on, or the options change.
        tab.bump_revision();
        assert_eq!(tab.outline(opts).len(), 2);
        tab.doc
            .add_card(Parent::Box(root), CardKind::Reason, "")
            .unwrap();
        let bare = OutlineOptions {
            numbering: false,
            ..opts
        };
        assert_eq!(tab.outline(bare).len(), 3);
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
