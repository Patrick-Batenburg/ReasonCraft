//! Snippet text editing sessions: like inline box editing, the typing
//! shows live and becomes one history step when the session ends.

use reasoncraft_core::{Document, MAX_SNIPPET_CHARS, Uuid};

use super::DocTab;

/// `text` cut to at most [`MAX_SNIPPET_CHARS`] characters, and whether it
/// was cut.
pub fn cap_snippet(text: &str) -> (&str, bool) {
    match text.char_indices().nth(MAX_SNIPPET_CHARS) {
        Some((end, _)) => (&text[..end], true),
        None => (text, false),
    }
}

fn snippet_text(doc: &Document, id: Uuid) -> Option<&str> {
    doc.snippets
        .iter()
        .find(|s| s.id == id)
        .map(|s| s.text.as_str())
}

impl DocTab {
    /// True when committing the snippet session now would record a step
    /// (for a snippet added by the session, always).
    pub fn has_pending_snippet_edit(&self) -> bool {
        self.snippet_edit
            .as_ref()
            .is_some_and(|(id, before)| snippet_text(before, *id) != snippet_text(&self.doc, *id))
    }

    /// End the snippet session, recording the document as it was before
    /// as one history step if the snippet changed.
    pub fn commit_snippet_edit(&mut self) {
        if self.has_pending_snippet_edit()
            && let Some((_, before)) = self.snippet_edit.as_ref()
        {
            self.history.record(before);
            self.bump_revision();
        }
        self.snippet_edit = None;
    }

    /// Show snippet `id`'s new text live, without a history step, cut at
    /// [`MAX_SNIPPET_CHARS`] (noted in [`Self::snippet_capped`]). The first
    /// change to a snippet starts its session, ending any other edit.
    /// Unchanged text changes nothing.
    pub fn set_snippet_text(&mut self, id: Uuid, text: &str) {
        let Some(current) = snippet_text(&self.doc, id) else {
            return;
        };
        let (text, capped) = cap_snippet(text);
        if capped {
            self.snippet_capped = Some(id);
        } else if self.snippet_capped == Some(id) {
            self.snippet_capped = None;
        }
        if current == text {
            return;
        }
        if self.snippet_edit.as_ref().map(|(s, _)| *s) != Some(id) {
            self.commit_edit();
            self.commit_snippet_edit();
            self.snippet_edit = Some((id, self.doc.clone()));
        }
        if self.doc.update_snippet_text(id, text).is_ok() {
            self.bump_revision();
        }
    }

    /// Forget snippet session, focus and notice state for a snippet that no
    /// longer exists (after undo, redo or removal).
    pub(super) fn fix_snippet_state(&mut self) {
        let gone = |doc: &Document, id: &Uuid| !doc.snippets.iter().any(|s| s.id == *id);
        if self
            .snippet_edit
            .as_ref()
            .is_some_and(|(id, _)| gone(&self.doc, id))
        {
            self.snippet_edit = None;
        }
        if self.snippet_focus.is_some_and(|id| gone(&self.doc, &id)) {
            self.snippet_focus = None;
        }
        if self.snippet_capped.is_some_and(|id| gone(&self.doc, &id)) {
            self.snippet_capped = None;
        }
    }
}
