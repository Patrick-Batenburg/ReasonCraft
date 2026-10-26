//! The commands the user can give (see [`super::apply`]).

use reasoncraft_core::checks::CheckKind;
use reasoncraft_core::essay::EssaySlot;
use reasoncraft_core::{
    Acceptance, CardKind, CardStrength, ClearScope, ConnectorStyle, FindOptions, Item,
    LayoutDirection, NewLoose, NodeId, Parent, SnippetSource, SourceRating, SourceType, Uuid,
    Verdict,
};

use super::{Insertable, Rate};
use crate::canvas::images::ImageResult;
use crate::export::{ExportFormat, ExportRequest, PageSetup};
use crate::jobs::JobId;
use crate::settings::{MinimapMode, OutlineToggle, TextTab};
use crate::theme::ThemeChoice;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Panel {
    Left,
    Right,
}

/// The answer to "save changes before closing?".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnsavedChoice {
    Save,
    Discard,
    Cancel,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    New,
    Open,
    Save,
    SaveAs,
    /// Close the tab at this index, asking first if it has unsaved changes.
    CloseTab(usize),
    /// Close the window, asking first about each tab with unsaved changes.
    CloseWindow,
    ResolveUnsaved(UnsavedChoice),
    /// The answer to "replace the existing file?" (true: replace).
    ResolveReplace(bool),
    /// Close the error message.
    DismissModal,
    Undo,
    Redo,
    AddReason,
    AddObjection,
    AddCoPremise,
    AddCategory,
    AddNote,
    AddLoose(NewLoose),
    /// Add a source of this type under the selected claim or premise, or
    /// loose when nothing is selected.
    AddSource(SourceType),
    /// Change the type of the selected source.
    SetSourceType(SourceType),
    /// Make the selected card (or the card of the selected premise) a
    /// reason or an objection; its strength is kept.
    SetCardKind(CardKind),
    /// Open the link editor for the selected box.
    EditLink,
    /// Set (`None`: remove) the link of the box being edited, or of the
    /// selected box, and close the link editor.
    SetLink(Option<String>),
    /// Open box `node`'s link in the browser (a click on its link glyph).
    OpenLink(NodeId),
    /// Pick an image file and put it in the selected box (replacing its image).
    InsertImage,
    /// The image prepared by `job` for box `node` (pushed by the frame
    /// loop when the job ends): put it in the box as one history step if
    /// the box can still hold it. Ignored when a newer insert replaced the
    /// job, or its tab was closed.
    ImagePrepared {
        job: JobId,
        node: NodeId,
        result: ImageResult,
    },
    RemoveImage,
    /// Add a card that targets the whole selected card (or the card of the selected premise).
    TargetWholeCard(CardKind),
    Delete,
    DeleteBranch,
    /// Start editing the selected box's text.
    Edit,
    /// Commit the inline edit (if any).
    StopEditing,
    /// The inline editor of box `node` changed its text: show it live,
    /// without a history step. Ignored unless `node` is still being edited.
    EditText {
        node: NodeId,
        text: String,
    },
    /// Text typed while a box is selected: start editing it (if needed) and append.
    TypeText(String),
    /// Add a snippet (an empty one takes focus); text is cut at the limit.
    AddSnippet(String),
    /// Snippet `id`'s text, citation or link field changed: show it live,
    /// without a history step until the snippet's editing session ends
    /// (like [`Action::EditText`]). A typed link that is not valid is not stored.
    EditSnippetText {
        id: Uuid,
        text: String,
    },
    EditSnippetCitation {
        id: Uuid,
        citation: String,
    },
    EditSnippetLink {
        id: Uuid,
        link: String,
    },
    /// Tag snippet `id` with a source (`None` clears). An invalid link is refused.
    SetSnippetSource {
        id: Uuid,
        source: Option<SnippetSource>,
    },
    RemoveSnippet(Uuid),
    /// The paragraph field for `slot` in the Essay tab changed: show it
    /// live, without a history step until the paragraph's editing session
    /// ends (like [`Action::EditText`]). Card slots are refused.
    EditEssayText {
        slot: EssaySlot,
        text: String,
    },
    /// Delete the unplaced essay note at this index.
    RemoveUnplaced(usize),
    /// Move snippet `id` to `index` in the list (with it taken out).
    MoveSnippet {
        id: Uuid,
        index: usize,
    },
    /// Connect, reattach, detach or reorder `item` (drag and drop, "Connect
    /// to…"). `index` counts `to`'s children with `item` taken out; `None`
    /// appends.
    Move {
        item: Item,
        to: Parent,
        index: Option<usize>,
    },
    /// Move the selection (a premise within its card) one place among its
    /// siblings (Alt+arrow). `Dir` is the arrow's on-screen direction, mapped
    /// like [`Action::Navigate`]: Left/Right top-down and bottom-up, Up/Down
    /// left-to-right, where siblings stack vertically.
    Reorder(Dir),
    /// Pick a target for the selected parking item ("Connect to…").
    StartConnect,
    CancelConnect,
    Select(Option<Item>),
    /// Pan the selection into view on the next canvas frame (after a
    /// selection made outside the canvas, e.g. in the outline).
    RevealSelection,
    /// Move the selection with an arrow key: `Dir` is the key's on-screen
    /// direction, mapped through the document's layout direction.
    Navigate(Dir),
    /// Jump between the main tree's root and the first parked item.
    JumpParking,
    ZoomIn,
    ZoomOut,
    ZoomFit,
    /// Fit the selected item (a card with everything under it) to the view.
    ZoomFitSelection,
    Zoom100,
    /// Set the zoom factor directly (status-bar slider), clamped to the zoom range.
    SetZoom(f32),
    SetTheme(ThemeChoice),
    /// Draw connectors in this style (a personal setting: no history).
    SetConnectorStyle(ConnectorStyle),
    /// Lay the document out in this direction (one history step; none
    /// when it already is).
    SetLayoutDirection(LayoutDirection),
    /// Rate the selection, whatever its kind (Alt+1/2/3/0).
    Rate(Rate),
    /// Accept, doubt or reject the selected claim or premise (`None` clears).
    SetAcceptance(Option<Acceptance>),
    /// Rate the selected card's strength (`None` clears).
    SetStrength(Option<CardStrength>),
    /// Rate the selected source (`None` clears).
    SetSourceRating(Option<SourceRating>),
    /// Clear one item's rating or every rating in the map.
    ClearEvaluation(ClearScope),
    /// Show or hide rating markers.
    ToggleMarkers,
    /// When the overview minimap shows (a personal setting).
    SetMinimap(MinimapMode),
    /// Show or hide the outline numbers on the map.
    ToggleNumbers,
    /// Turn evaluation shading on or off.
    ToggleShading,
    /// Comment on box `node` as the author in the settings. Blank text is refused.
    AddComment {
        node: NodeId,
        text: String,
        verdict: Option<Verdict>,
    },
    RemoveComment {
        node: NodeId,
        comment: Uuid,
    },
    /// Show the comment popover of a box (and select it).
    OpenComments(NodeId),
    /// Show the comment popover of the selected box (ribbon, Ctrl+Shift+M).
    CommentSelected,
    CloseComments,
    /// Set the name put on new comments (trimmed).
    SetAuthor(String),
    /// Insert from the building panel: under `to` (cards, categories, notes,
    /// sources), on box `on` (a co-premise joins its card; a comment goes
    /// on it), or at the selection like the ribbon when both are `None`.
    Insert {
        what: Insertable,
        to: Option<Parent>,
        on: Option<NodeId>,
    },
    TogglePanel(Panel),
    /// Open a side panel (it stays open if it already is).
    ShowPanel(Panel),
    /// Show this tab in the Text panel.
    SetTextTab(TextTab),
    /// Turn one of the Outline tab's switches on or off.
    SetOutlineOption(OutlineToggle, bool),
    /// Turn one course-rule check on or off.
    SetCheckEnabled(CheckKind, bool),
    /// The Document page's title or author field changed: show it live,
    /// without a history step until the session ends (like
    /// [`Action::EditText`]).
    SetDocMeta {
        title: String,
        author: String,
    },
    /// Set the document language (`"en"` or `"nl"`); anything else is refused.
    SetDocLanguage(String),
    /// Set the language new documents start with (`"en"` or `"nl"`).
    SetDefaultDocLanguage(String),
    /// Switch to the tab at this index.
    ActivateTab(usize),
    /// Open the export dialog for an image format; a text format asks
    /// for a file right away.
    OpenExport(ExportFormat),
    /// Remember the request's options, ask where to save and start the
    /// export on a worker thread.
    RunExport(ExportRequest),
    /// Close the export or page setup dialog without exporting.
    CloseExportDialog,
    /// Open the page setup dialog.
    OpenPageSetup,
    /// Remember the page setup for PDF export and printing.
    SetPageSetup(PageSetup),
    /// Put the map on the clipboard as an image, in the export scope and
    /// theme like the image exports (2x; the whole map when the scope is
    /// the selection and nothing is selected).
    CopyAsImage,
    /// Make a PDF with the saved page setup and open it to print.
    Print,
    /// Show the folder of the last exported file.
    OpenExportFolder,
    /// The answer to "replace the existing file?" for an export (true:
    /// replace and export).
    ResolveExportReplace(bool),
    /// The answer to "finishing exports" while the window closes: true
    /// quits now (the running exports write nothing more), false keeps
    /// the window open.
    ResolveClosingExports(bool),
    /// Put the keyboard focus in the title bar's Search commands box (Alt+Q).
    OpenCommandSearch,
    /// Show the find bar (with the replacement field when `replace`) and
    /// put the keyboard focus in its query field.
    OpenFind {
        replace: bool,
    },
    /// Hide the find bar; the map has the keys again.
    CloseFind,
    /// Select the next (previous) hit's box and reveal it, wrapping
    /// around. With the bar closed it opens first.
    FindNext,
    FindPrev,
    /// Replace the current hit (one history step) and go to the next;
    /// with no current hit, go to the first.
    ReplaceCurrent,
    /// Replace every hit as one history step, with a status note.
    ReplaceAll,
    /// The query changed: go to its first hit.
    SetFindQuery(String),
    SetReplaceText(String),
    /// Match case and whole word changed: go to the first hit.
    SetFindOptions(FindOptions),
    /// Open the built-in sample map in a new tab (debug builds only).
    #[cfg(debug_assertions)]
    LoadSampleMap,
    /// Show or hide the source icon preview (debug builds only).
    #[cfg(debug_assertions)]
    ToggleSourceIcons,
}
