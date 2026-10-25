//! Snippet editing sessions: like inline box editing, typing in a
//! snippet's text, citation or link shows live in the document and becomes
//! one history step when the session ends.

use reasoncraft_core::{Document, MAX_SNIPPET_CHARS, Snippet, SnippetSource, SourceType, Uuid};

use super::DocTab;

/// `text` cut to at most [`MAX_SNIPPET_CHARS`] characters, and whether it
/// was cut.
pub fn cap_snippet(text: &str) -> (&str, bool) {
    match text.char_indices().nth(MAX_SNIPPET_CHARS) {
        Some((end, _)) => (&text[..end], true),
        None => (text, false),
    }
}

/// A source as stored: citation trimmed, and `None` when nothing is left
/// (generic type, no citation, no link).
pub fn tidy_source(source: Option<SnippetSource>) -> Option<SnippetSource> {
    let mut s = source?;
    s.citation = s.citation.trim().to_owned();
    let empty = s.citation.is_empty() && s.link.is_none() && s.source_type == SourceType::Source;
    (!empty).then_some(s)
}

fn snippet(doc: &Document, id: Uuid) -> Option<&Snippet> {
    doc.snippets.iter().find(|s| s.id == id)
}

/// Whether snippet `id` differs between `before` and `now`, counting its
/// source as it will be stored.
fn differs(before: &Document, now: &Document, id: Uuid) -> bool {
    match (snippet(before, id), snippet(now, id)) {
        (Some(a), Some(b)) => a.text != b.text || a.source != tidy_source(b.source.clone()),
        (a, b) => a.is_some() != b.is_some(),
    }
}

impl DocTab {
    /// True when committing the snippet session now would record a step
    /// (for a snippet added by the session, always).
    pub fn has_pending_snippet_edit(&self) -> bool {
        self.snippet_edit
            .as_ref()
            .is_some_and(|(id, before)| differs(before, &self.doc, *id))
    }

    /// End the snippet session: store its source tidied, and record the
    /// document as it was before as one history step if the snippet changed.
    pub fn commit_snippet_edit(&mut self) {
        let Some((id, before)) = self.snippet_edit.take() else {
            return;
        };
        if let Some(s) = snippet(&self.doc, id) {
            let tidy = tidy_source(s.source.clone());
            if s.source != tidy && self.doc.set_snippet_source(id, tidy).is_ok() {
                self.bump_revision();
            }
        }
        if differs(&before, &self.doc, id) {
            self.history.record(&before);
        }
    }

    /// End every running edit (inline box text, snippet and title/author sessions), as
    /// before a save, a close or any command.
    pub fn commit_edits(&mut self) {
        self.commit_edit();
        self.commit_snippet_edit();
        self.commit_meta_edit();
    }

    /// Change snippet `id` live, without a history step: `change` gets a
    /// copy of the snippet and the document as it was before the session.
    /// A change that leaves the snippet as it is does nothing; the first
    /// real change starts the snippet's session, ending any other edit.
    pub fn edit_snippet_live(&mut self, id: Uuid, change: impl FnOnce(&mut Snippet, &Document)) {
        let Some(current) = snippet(&self.doc, id) else {
            return;
        };
        let mut new = current.clone();
        let before = match &self.snippet_edit {
            Some((s, before)) if *s == id => before,
            _ => &self.doc,
        };
        change(&mut new, before);
        if &new == current {
            return;
        }
        if self.snippet_edit.as_ref().map(|(s, _)| *s) != Some(id) {
            self.commit_edits();
            self.snippet_edit = Some((id, self.doc.clone()));
        }
        let text_ok = self.doc.update_snippet_text(id, &new.text).is_ok();
        if text_ok && self.doc.set_snippet_source(id, new.source).is_ok() {
            self.bump_revision();
        }
    }

    /// Show snippet `id`'s new text live, cut at [`MAX_SNIPPET_CHARS`]
    /// (noted in [`Self::snippet_capped`]).
    pub fn set_snippet_text(&mut self, id: Uuid, text: &str) {
        if snippet(&self.doc, id).is_none() {
            return;
        }
        let (text, capped) = cap_snippet(text);
        if capped {
            self.snippet_capped = Some(id);
        } else if self.snippet_capped == Some(id) {
            self.snippet_capped = None;
        }
        self.edit_snippet_live(id, |s, _| text.clone_into(&mut s.text));
    }

    /// Forget snippet session, focus and notice state for a snippet that no
    /// longer exists (after undo, redo or removal).
    pub(super) fn fix_snippet_state(&mut self) {
        let gone = |doc: &Document, id: &Uuid| snippet(doc, *id).is_none();
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
