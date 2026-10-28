//! Title and author editing sessions: typing in the Document page shows
//! live in the document and becomes one history step when the session ends.

use reasoncraft_core::Document;

use super::{DocTab, EditSession};

/// Whether title or author differ between the two documents.
fn differs(a: &Document, b: &Document) -> bool {
    a.meta.title != b.meta.title || a.meta.author != b.meta.author
}

/// True when ending a title/author session that started at `before` would
/// record a step (the committed values are trimmed).
pub(super) fn pending(before: &Document, now: &Document) -> bool {
    before.meta.title != now.meta.title.trim() || before.meta.author != now.meta.author.trim()
}

impl DocTab {
    /// End a title/author session that started at `before`: store both
    /// trimmed, and record `before` as one history step if they changed.
    pub(super) fn finish_meta(&mut self, before: Document) -> bool {
        let (title, author) = (self.doc.meta.title.trim(), self.doc.meta.author.trim());
        let (title, author) = (title.to_owned(), author.to_owned());
        if self.doc.set_meta(&title, &author).is_ok() {
            self.bump_revision();
        }
        let changed = differs(&before, &self.doc);
        if changed {
            self.history.record(before);
        }
        changed
    }

    /// Show a new title and author live, without a history step. Values
    /// that leave them as they are do nothing; the first real change starts
    /// the session, ending any other edit.
    pub fn edit_meta_live(&mut self, title: &str, author: &str) {
        if self.doc.meta.title == title && self.doc.meta.author == author {
            return;
        }
        if self.session() != Some(&EditSession::Meta) {
            self.begin_session(EditSession::Meta);
        }
        if self.doc.set_meta(title, author).is_ok() {
            self.bump_revision();
        }
    }
}
