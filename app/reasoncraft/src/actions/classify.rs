//! What each action leaves alone: the status note, "Move to…", a
//! running edit session and an inline edit. One exhaustive match, so a new
//! action cannot be added without deciding.

use super::Action;
use crate::doc_tab::EditSession;

/// The kinds of edit session (see [`EditSession`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SessionKind {
    Box,
    Snippet,
    Meta,
    Essay,
}

impl SessionKind {
    fn of(session: &EditSession) -> Self {
        match session {
            EditSession::Box { .. } => Self::Box,
            EditSession::Snippet { .. } => Self::Snippet,
            EditSession::Meta => Self::Meta,
            EditSession::Essay { .. } => Self::Essay,
        }
    }
}

/// How an action treats what is going on around it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Traits {
    /// Leaves the status note and export notice alone: it only looks
    /// around (selection, view, panels), changes a personal setting, or
    /// answers a message (which may have left a note behind). Document
    /// edits and file, export and image commands clear them, and so does
    /// switching the UI language (a note is in the language it was
    /// written in).
    pub keeps_notes: bool,
    /// Leaves "Move to…" running: picking a target (selecting,
    /// navigating, revealing, zooming), starting it again, view
    /// preferences and personal settings. The inline editor's own "stop
    /// editing" (sent when it loses focus, e.g. to the ribbon's "Move
    /// to…") keeps it too: `apply` committed the edit when the mode
    /// started, and Esc arrives as [`Action::CancelConnect`].
    pub keeps_connect: bool,
    /// A shortcut-table command that also works (ending the edit) while a
    /// box is edited inline (see `crate::canvas::input::key_to_action`).
    pub works_while_editing: bool,
    /// The kind of edit session this action is live text for, which it
    /// must not end (box text, snippet fields, title and author, essay
    /// paragraph).
    pub live_for: Option<SessionKind>,
}

/// Changes something: clears the notes and ends "Move to…".
const CHANGES: Traits = Traits {
    keeps_notes: false,
    keeps_connect: false,
    works_while_editing: false,
    live_for: None,
};
/// Only looks around or sets a preference: keeps both.
const LOOKS: Traits = Traits {
    keeps_notes: true,
    keeps_connect: true,
    ..CHANGES
};
/// Keeps the notes but ends "Move to…".
const KEEPS_NOTES: Traits = Traits {
    keeps_notes: true,
    ..CHANGES
};
/// Keeps "Move to…" but clears the notes.
const KEEPS_CONNECT: Traits = Traits {
    keeps_connect: true,
    ..CHANGES
};

const fn editing(t: Traits) -> Traits {
    Traits {
        works_while_editing: true,
        ..t
    }
}

const fn live(kind: SessionKind) -> Traits {
    Traits {
        live_for: Some(kind),
        ..CHANGES
    }
}

impl Action {
    /// How this action treats the notes, "Move to…" and edit sessions.
    pub(crate) fn traits(&self) -> Traits {
        use SessionKind as S;
        match self {
            Action::Select(_)
            | Action::RevealSelection
            | Action::Navigate(_)
            | Action::JumpParking
            | Action::ZoomIn
            | Action::ZoomOut
            | Action::ZoomFit
            | Action::ZoomFitSelection
            | Action::Zoom100
            | Action::SetZoom(_)
            | Action::TogglePanel(_)
            | Action::ShowPanel(_)
            | Action::SetTextTab(_)
            | Action::SetTheme(_)
            | Action::SetUiScale(_)
            | Action::SetConnectorStyle(_)
            | Action::ToggleMarkers
            | Action::SetMinimap(_)
            | Action::ToggleNumbers
            | Action::ToggleShading
            | Action::SetOutlineOption(..)
            | Action::SetCheckEnabled(..)
            | Action::SetDefaultDocLanguage(_)
            | Action::SetSpellcheck(_)
            | Action::SetAutosaveSecs(_)
            | Action::SetAuthor(_)
            | Action::OpenHelp(_)
            | Action::HelpBack => LOOKS,
            // A finished image job is not the user's doing (`apply`
            // handles it before looking at these).
            Action::ImagePrepared { .. } => LOOKS,
            Action::NextPane | Action::PrevPane | Action::ContextHelp => editing(LOOKS),
            Action::NextTab | Action::PrevTab => editing(KEEPS_NOTES),
            Action::ActivateTab(_)
            | Action::ResetSettings
            | Action::ConfirmResetSettings
            | Action::OpenComments(_)
            | Action::CloseComments
            | Action::OpenLink(_)
            | Action::OpenSelectedLink
            | Action::OpenExportFolder
            | Action::DismissModal
            | Action::CloseFind
            | Action::SetFindQuery(_)
            | Action::SetReplaceText(_)
            | Action::SetFindOptions(_)
            | Action::CloseSpelling
            | Action::SpellIgnore
            | Action::SpellIgnoreAll(_)
            | Action::ShowAbout => KEEPS_NOTES,
            Action::OpenCommandSearch
            | Action::OpenFind { .. }
            | Action::FindNext
            | Action::FindPrev
            | Action::OpenSpelling => editing(KEEPS_NOTES),
            Action::StartConnect | Action::StopEditing | Action::SetUiLanguage(_) => KEEPS_CONNECT,
            Action::EditText { .. } | Action::TypeText(_) => live(S::Box),
            Action::EditSnippetText { .. }
            | Action::EditSnippetCitation { .. }
            | Action::EditSnippetLink { .. } => live(S::Snippet),
            Action::SetDocMeta { .. } => live(S::Meta),
            Action::EditEssayText { .. } => live(S::Essay),
            Action::New
            | Action::Open
            | Action::Save
            | Action::SaveAs
            | Action::CloseActiveTab
            | Action::TargetWholeCard(_)
            | Action::AddSource(_) => editing(CHANGES),
            Action::ImportArgdown
            | Action::CloseTab(_)
            | Action::CloseWindow
            | Action::ResolveUnsaved(_)
            | Action::ResolveReplace(_)
            | Action::Undo
            | Action::Redo
            | Action::AddReason
            | Action::AddObjection
            | Action::AddCoPremise
            | Action::AddCategory
            | Action::AddNote
            | Action::AddLoose(_)
            | Action::SetSourceType(_)
            | Action::SetCardKind(_)
            | Action::EditLink
            | Action::SetLink(_)
            | Action::InsertImage
            | Action::RemoveImage
            | Action::Delete
            | Action::DeleteBranch
            | Action::Edit
            | Action::AddSnippet(_)
            | Action::SetSnippetSource { .. }
            | Action::RemoveSnippet(_)
            | Action::RemoveUnplaced(_)
            | Action::MoveSnippet { .. }
            | Action::Move { .. }
            | Action::MoveToParking
            | Action::Reorder(_)
            | Action::CancelConnect
            | Action::SetLayoutDirection(_)
            | Action::Rate(_)
            | Action::SetAcceptance(_)
            | Action::SetStrength(_)
            | Action::SetSourceRating(_)
            | Action::ClearEvaluation(_)
            | Action::AddComment { .. }
            | Action::RemoveComment { .. }
            | Action::CommentSelected
            | Action::Insert { .. }
            | Action::IgnoreHint(_)
            | Action::RestoreHints
            | Action::SetDocLanguage(_)
            | Action::OpenExport(_)
            | Action::RunExport(_)
            | Action::CloseExportDialog
            | Action::OpenPageSetup
            | Action::SetPageSetup(_)
            | Action::CopyAsImage
            | Action::Copy
            | Action::Cut
            | Action::Paste(_)
            | Action::Print
            | Action::ResolveExportReplace(_)
            | Action::ResolveClosingExports(_)
            | Action::ReplaceCurrent
            | Action::ReplaceAll
            | Action::SpellChange { .. }
            | Action::SpellChangeAll { .. }
            | Action::SpellAddWord(_)
            | Action::ResolveRecovery(_)
            | Action::OpenExample(_)
            | Action::OpenTemplate(_)
            | Action::OpenRecent(_) => CHANGES,
            #[cfg(debug_assertions)]
            Action::LoadSampleMap | Action::ToggleSourceIcons => CHANGES,
        }
    }

    /// Whether this action is live text for `session`, so it must not end it.
    pub(super) fn continues(&self, session: Option<&EditSession>) -> bool {
        session.is_some_and(|s| self.traits().live_for == Some(SessionKind::of(s)))
    }
}

#[cfg(test)]
mod tests;
