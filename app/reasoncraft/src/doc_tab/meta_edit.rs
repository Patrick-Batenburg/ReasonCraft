//! Title and author editing sessions: typing in the Document page shows
//! live in the document and becomes one history step when the session ends.

use reasoncraft_core::Document;

use super::DocTab;

/// Whether title or author differ between the two documents.
fn differs(a: &Document, b: &Document) -> bool {
    a.meta.title != b.meta.title || a.meta.author != b.meta.author
}

impl DocTab {
    /// True when committing the title/author session now would record a
    /// step (the committed values are trimmed).
    pub fn has_pending_meta_edit(&self) -> bool {
        self.meta_edit.as_ref().is_some_and(|before| {
            before.meta.title != self.doc.meta.title.trim()
                || before.meta.author != self.doc.meta.author.trim()
        })
    }

    /// End the title/author session: store both trimmed, and record the
    /// document as it was before as one history step if they changed.
    pub fn commit_meta_edit(&mut self) {
        let Some(before) = self.meta_edit.take() else {
            return;
        };
        let (title, author) = (self.doc.meta.title.trim(), self.doc.meta.author.trim());
        let (title, author) = (title.to_owned(), author.to_owned());
        if self.doc.set_meta(&title, &author).is_ok() {
            self.bump_revision();
        }
        if differs(&before, &self.doc) {
            self.history.record(&before);
        }
    }

    /// Show a new title and author live, without a history step. Values
    /// that leave them as they are do nothing; the first real change starts
    /// the session, ending any other edit.
    pub fn edit_meta_live(&mut self, title: &str, author: &str) {
        if self.doc.meta.title == title && self.doc.meta.author == author {
            return;
        }
        if self.meta_edit.is_none() {
            self.commit_edits();
            self.meta_edit = Some(self.doc.clone());
        }
        if self.doc.set_meta(title, author).is_ok() {
            self.bump_revision();
        }
    }
}
