//! Which actions leave the status note alone and which belong to a running
//! edit session.

use super::Action;
use crate::doc_tab::EditSession;

/// Whether `action` leaves the status note and export notice alone: it
/// only looks around (selection, view, panels) or answers a message (which
/// may have left a note behind). "Open folder" acts on the notice itself.
/// Document edits and file, export and image commands clear them.
pub(super) fn keeps_notes(action: &Action) -> bool {
    matches!(
        action,
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
            | Action::ActivateTab(_)
            | Action::TogglePanel(_)
            | Action::ShowPanel(_)
            | Action::SetTextTab(_)
            | Action::SetTheme(_)
            | Action::SetConnectorStyle(_)
            | Action::ToggleMarkers
            | Action::SetMinimap(_)
            | Action::ToggleNumbers
            | Action::ToggleShading
            | Action::SetOutlineOption(..)
            | Action::OpenComments(_)
            | Action::CloseComments
            | Action::OpenLink(_)
            | Action::OpenExportFolder
            | Action::DismissModal
            | Action::OpenCommandSearch
            | Action::OpenFind { .. }
            | Action::CloseFind
            | Action::FindNext
            | Action::FindPrev
            | Action::SetFindQuery(_)
            | Action::SetReplaceText(_)
            | Action::SetFindOptions(_)
            | Action::OpenSpelling
            | Action::CloseSpelling
            | Action::SpellIgnore
            | Action::SpellIgnoreAll(_)
    )
}

/// Whether `action` is live text for the kind of session that is running
/// (box text for a box session, snippet fields for a snippet session,
/// title and author for a title/author session, paragraph text for an
/// essay session), so it must not end it.
pub(super) fn continues_session(action: &Action, session: Option<&EditSession>) -> bool {
    matches!(
        (action, session),
        (
            Action::EditText { .. } | Action::TypeText(_),
            Some(EditSession::Box { .. })
        ) | (
            Action::EditSnippetText { .. }
                | Action::EditSnippetCitation { .. }
                | Action::EditSnippetLink { .. },
            Some(EditSession::Snippet { .. })
        ) | (Action::SetDocMeta { .. }, Some(EditSession::Meta))
            | (
                Action::EditEssayText { .. },
                Some(EditSession::Essay { .. })
            )
    )
}
