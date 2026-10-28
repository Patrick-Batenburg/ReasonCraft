//! One open document: its content, history, file path and view state.

use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use eframe::egui::{Id, Rect};
use reasoncraft_core::checks::{CheckSet, Issue};
use reasoncraft_core::essay::EssaySlot;
use reasoncraft_core::meta::LANGUAGES;
use reasoncraft_core::outline::{OutlineLine, OutlineOptions};
use reasoncraft_core::{Assets, Document, EditError, History, Item, NodeId, Package, Uuid};

use crate::canvas::Camera;
use crate::i18n::t;
use crate::jobs::JobId;
use crate::memo::Memo;

mod derived;
mod essay_edit;
mod find_state;
mod meta_edit;
mod row_heights;
mod session;
mod snippet_edit;
pub use derived::ISSUE_SETTLE;
pub use essay_edit::EssayView;
pub use find_state::FindState;
pub use row_heights::{CheckRow, EssayRow, RowHeights};
pub use session::EditSession;
pub use snippet_edit::{cap_snippet, tidy_source};

/// The next [`DocTab::ui_key`].
static NEXT_UI_KEY: AtomicU64 = AtomicU64::new(0);

/// The longest file name (in characters, without extension) a Save or
/// Export suggests from a document title.
pub const FILE_STEM_CHARS: usize = 100;

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
    /// written back unchanged on save. The bytes are shared, so autosave
    /// copies none.
    pub other_entries: Vec<(String, Arc<[u8]>)>,
    /// Where the document was last saved or opened from.
    pub path: Option<PathBuf>,
    pub selection: Option<Item>,
    /// The running edit session (box text, snippet, title/author or essay
    /// paragraph) with the document as it was when it began, recorded as
    /// one history step when it ends (see [`Self::commit_session`]).
    session: Option<(EditSession, Document)>,
    /// The inline box editor's text (mirrors the edited box's text).
    pub edit_text: String,
    /// The inline editor should take keyboard focus on its next frame.
    pub edit_focus: bool,
    pub camera: Camera,
    /// The screen rect the canvas showed this tab in last frame.
    pub viewport: Option<Rect>,
    /// Fit the map to the view on the next canvas frame (set for new
    /// documents and by Zoom Fit, so the fit uses the fresh layout).
    pub fit_pending: bool,
    /// Fit the selection to the view on the next canvas frame (Fit
    /// selection), with that frame's layout.
    pub fit_selection_pending: bool,
    /// The overview minimap's shapes, kept until the document, theme or
    /// layout settings change.
    pub minimap: crate::canvas::minimap::Cache,
    /// Pan the selection into view on the next canvas frame (set by
    /// keyboard navigation and adds, so it uses the fresh layout).
    pub reveal_pending: bool,
    /// The box whose comment popover is showing.
    pub comments_open: Option<NodeId>,
    /// The parked item waiting for a target ("Connect to…").
    pub connect_mode: Option<Item>,
    /// The snippet whose text field should take keyboard focus next frame.
    pub snippet_focus: Option<Uuid>,
    /// The snippet whose last text was cut at the length limit (a notice
    /// shows under it).
    pub snippet_capped: Option<Uuid>,
    /// Counts changes to [`Self::doc`], so text derived from it (outline,
    /// checks) can be cached until the next change. Starts at 0.
    revision: u64,
    /// The outline last built, with the revision and options it was built
    /// for (see [`Self::outline`]).
    pub outline_cache: Memo<(u64, OutlineOptions), Vec<OutlineLine>>,
    /// The issues last found, with the revision and checks they were found
    /// for (see [`Self::issues`]).
    pub issue_cache: Memo<(u64, CheckSet), Vec<Issue>>,
    /// The revision [`Self::settled_issues`] last saw, and when it first
    /// saw it (in seconds of UI time).
    seen_revision: (u64, f64),
    /// The Essay tab's view, with the revision it was built for (see
    /// [`Self::essay`]).
    pub essay_cache: Memo<u64, EssayView>,
    /// The heights of the Essay tab's rows, so it lays out only those in
    /// view; [`RowHeights::laid_out`] counts the rows of the last frame.
    pub essay_heights: RowHeights<EssayRow>,
    /// The same for the Check tab.
    pub check_heights: RowHeights<CheckRow>,
    /// The same for the Snippets tab, by snippet.
    pub snippet_heights: RowHeights<Uuid>,
    /// The boxes whose inserted image is being prepared, with the job
    /// preparing it; a newer insert for the same box replaces the entry,
    /// so the older result is dropped when it arrives.
    pub pending_images: HashMap<NodeId, JobId>,
    /// The find bar: its query, options, current hit and cached hits.
    pub find: FindState,
    /// The spelling pass (F7), while its panel is open.
    pub spelling: Option<crate::spell::SpellPass>,
    /// Names this tab's copy in the recovery folder.
    pub recovery_key: Uuid,
    /// The [`Self::revision`] last autosaved to the recovery folder;
    /// `None` when the folder holds no copy of this tab.
    pub autosaved_revision: Option<u64>,
    /// The modification time of [`Self::path`] when the tab last opened or
    /// saved it (autosave records it, so a restore can tell whether the
    /// file changed since).
    pub disk_mtime: Option<std::time::SystemTime>,
    /// Where this tab was restored from after a crash, until it is saved.
    pub recovered_from: Option<crate::recovery::RecoveryMeta>,
    /// Unique to this tab, so its text fields' widget state (cursor, undo
    /// history) is never shared with another tab (see [`Self::widget_id`]).
    ui_key: u64,
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

    /// Like [`Self::new`], in language `lang` (`"en"` when it is not a
    /// language the checks know).
    pub fn with_language(lang: &str) -> Self {
        let mut doc = Document::new("");
        doc.meta.language = if LANGUAGES.contains(&lang) {
            lang
        } else {
            "en"
        }
        .to_owned();
        Self::from_document(doc)
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
            cut_on_open: _,
        } = pkg;
        let selection = Some(Item::Box(doc.root));
        Self {
            doc,
            history: History::new(HISTORY_LIMIT),
            assets,
            other_entries,
            path: None,
            selection,
            session: None,
            edit_text: String::new(),
            edit_focus: false,
            camera: Camera::default(),
            viewport: None,
            fit_pending: true,
            fit_selection_pending: false,
            minimap: Default::default(),
            reveal_pending: false,
            comments_open: None,
            connect_mode: None,
            snippet_focus: None,
            snippet_capped: None,
            revision: 0,
            outline_cache: Memo::default(),
            issue_cache: Memo::default(),
            seen_revision: (0, 0.0),
            essay_cache: Memo::default(),
            essay_heights: RowHeights::default(),
            check_heights: RowHeights::default(),
            snippet_heights: RowHeights::default(),
            pending_images: HashMap::new(),
            find: FindState::default(),
            spelling: None,
            recovery_key: Uuid::new_v4(),
            autosaved_revision: None,
            disk_mtime: None,
            recovered_from: None,
            ui_key: NEXT_UI_KEY.fetch_add(1, Ordering::Relaxed),
        }
    }

    /// The widget id for `key` in this tab. Text fields use it: egui keeps
    /// a field's cursor and undo history per id, and the same field in
    /// another tab (or another document in this tab) must not inherit them.
    pub fn widget_id(&self, key: impl Hash + Debug) -> Id {
        Id::unique(("doc_tab", self.ui_key, key))
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

    /// Note that a file was loaded into this tab, in place of a tab at
    /// revision `replaced` (0 for none). The new revision is past both, so
    /// nothing cached for the replaced document is reused.
    pub fn mark_loaded(&mut self, replaced: u64) {
        self.revision = self.revision.max(replaced) + 1;
    }

    /// Unsaved changes, counting edit-session text not yet committed.
    pub fn is_dirty(&self) -> bool {
        self.history.is_dirty() || self.has_pending_session()
    }

    /// Work a close would lose: unsaved changes, or an image still being
    /// prepared for one of its boxes.
    pub fn has_unsaved_work(&self) -> bool {
        self.is_dirty() || !self.pending_images.is_empty()
    }

    /// The file stem; without a file, the document's title (an imported
    /// map, an example), else "Untitled".
    pub fn name(&self) -> String {
        if let Some(stem) = self.path.as_deref().and_then(|p| p.file_stem()) {
            return stem.to_string_lossy().into_owned();
        }
        match self.doc.meta.title.trim() {
            "" => t("untitled"),
            title => title.to_owned(),
        }
    }

    /// [`Self::name`] as the file name a Save or Export suggests: without
    /// the characters file names cannot hold, at most [`FILE_STEM_CHARS`]
    /// long ("Untitled" when nothing is left).
    pub fn file_stem(&self) -> String {
        let name = self.name();
        let kept = name
            .chars()
            .map(|c| match c {
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => ' ',
                c if c.is_control() => ' ',
                c => c,
            })
            .collect::<String>();
        let words: Vec<&str> = kept.split_whitespace().collect();
        let stem: String = words.join(" ").chars().take(FILE_STEM_CHARS).collect();
        match stem.trim_end_matches(['.', ' ']) {
            "" => t("untitled"),
            stem => stem.to_owned(),
        }
    }

    /// [`Self::name`], marked when restored after a crash (until saved),
    /// followed by " •" when there are unsaved changes.
    pub fn title(&self) -> String {
        let mut name = self.name();
        if self.recovered_from.is_some() {
            let mut args = crate::i18n::FluentArgs::new();
            args.set("name", name);
            name = crate::i18n::t_args("recovery.tab_title", &args);
        }
        if self.is_dirty() {
            format!("{name} •")
        } else {
            name
        }
    }

    /// Run `edit` as one undoable step. A refused edit changes nothing and
    /// leaves no history entry; in debug builds an error no command should
    /// cause stops the app.
    pub fn edit<T>(
        &mut self,
        edit: impl FnOnce(&mut Document) -> Result<T, EditError>,
    ) -> Option<T> {
        let mut draft = self.doc.clone();
        let out = match edit(&mut draft) {
            Ok(out) => out,
            // A rule refusing (or nothing to change) and a target that is
            // gone (an id from an earlier frame) are expected. A cycle, or
            // a claim losing its last premise, means a command skipped the
            // check that should have greyed it out.
            Err(e) => {
                debug_assert!(
                    matches!(e, EditError::NotAllowed | EditError::NotFound),
                    "unexpected edit error: {e:?}"
                );
                return None;
            }
        };
        // The document before the edit becomes the snapshot: one copy per
        // edit (the draft), none for the history.
        let before = std::mem::replace(&mut self.doc, draft);
        self.history.record(before);
        self.bump_revision();
        Some(out)
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
        self.drop_session_if(|doc, s| match s {
            EditSession::Box { node, .. } => !doc.boxes.contains_key(node),
            EditSession::Essay {
                slot: EssaySlot::Item(Item::Box(b)),
            } => !doc.boxes.contains_key(b),
            _ => false,
        });
        if self
            .comments_open
            .is_some_and(|b| !self.doc.boxes.contains_key(&b))
        {
            self.comments_open = None;
        }
        if self.connect_mode.is_some_and(|i| !self.contains(i)) {
            self.connect_mode = None;
        }
        self.fix_snippet_state();
    }
}

#[cfg(test)]
mod tests;
