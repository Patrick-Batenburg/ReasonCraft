//! Snippet editing sessions: like inline box editing, typing in a
//! snippet's text, citation or link shows live in the document and becomes
//! one history step when the session ends.

use reasoncraft_core::{Document, MAX_SNIPPET_CHARS, Snippet, SnippetSource, SourceType, Uuid};

use super::{DocTab, EditSession};

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
pub(super) fn differs(before: &Document, now: &Document, id: Uuid) -> bool {
    match (snippet(before, id), snippet(now, id)) {
        (Some(a), Some(b)) => a.text != b.text || a.source != tidy_source(b.source.clone()),
        (a, b) => a.is_some() != b.is_some(),
    }
}

impl DocTab {
    /// End a session on snippet `id` that started at `before`: store its
    /// source tidied, and record `before` as one history step if the
    /// snippet changed.
    pub(super) fn finish_snippet(&mut self, id: Uuid, before: Document) -> bool {
        if let Some(s) = snippet(&self.doc, id) {
            let tidy = tidy_source(s.source.clone());
            if s.source != tidy && self.doc.set_snippet_source(id, tidy).is_ok() {
                self.bump_revision();
            }
        }
        let changed = differs(&before, &self.doc, id);
        if changed {
            self.history.record(before);
        }
        changed
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
        let this = EditSession::Snippet { id };
        let before = match &self.session {
            Some((s, before)) if *s == this => before,
            _ => &self.doc,
        };
        change(&mut new, before);
        if &new == current {
            return;
        }
        if self.session() != Some(&this) {
            self.begin_session(this);
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
        self.drop_session_if(|doc, s| matches!(s, EditSession::Snippet { id } if gone(doc, id)));
        if self.snippet_focus.is_some_and(|id| gone(&self.doc, &id)) {
            self.snippet_focus = None;
        }
        if self.snippet_capped.is_some_and(|id| gone(&self.doc, &id)) {
            self.snippet_capped = None;
        }
    }
}
