//! The tab's one running edit session. Typing in a box, a snippet, the
//! title and author fields or an essay paragraph shows live in the
//! document, and the whole session becomes one history step when it ends.
//! Starting a session ends the one before.

use reasoncraft_core::caps::cap_chars;
use reasoncraft_core::essay::EssaySlot;
use reasoncraft_core::{Document, Item, MAX_BOX_CHARS, NodeId, Uuid};

use super::{DocTab, essay_edit, meta_edit, snippet_edit};

/// What the running edit session edits.
#[derive(Clone, Debug, PartialEq)]
pub enum EditSession {
    /// Box `node`'s text, in the inline editor. `fresh`: the box was just
    /// added, so its first text belongs to the add's history step.
    Box { node: NodeId, fresh: bool },
    /// A snippet's text, citation or link.
    Snippet { id: Uuid },
    /// The document's title and author.
    Meta,
    /// One paragraph of the essay.
    Essay { slot: EssaySlot },
}

/// Box `b`'s text in `doc`, if the box exists.
fn box_text(doc: &Document, b: NodeId) -> Option<&String> {
    doc.boxes.get(&b).map(|n| &n.text)
}

impl DocTab {
    /// The running edit session, if any.
    pub fn session(&self) -> Option<&EditSession> {
        self.session.as_ref().map(|(s, _)| s)
    }

    /// The box being edited inline, if any.
    pub fn editing(&self) -> Option<NodeId> {
        match self.session() {
            Some(EditSession::Box { node, .. }) => Some(*node),
            _ => None,
        }
    }

    /// Start session `s`, ending any running session first. The document
    /// as it is then becomes the session's snapshot.
    pub fn begin_session(&mut self, s: EditSession) {
        self.commit_session();
        self.session = Some((s, self.doc.clone()));
    }

    /// Like [`Self::begin_session`], with `before` as the snapshot: for a
    /// session that starts with an add, so the add and the first text are
    /// one history step.
    pub(crate) fn begin_session_with(&mut self, s: EditSession, before: Document) {
        self.commit_session();
        self.session = Some((s, before));
    }

    /// Forget the session without recording it when `gone` says its
    /// target no longer exists (after undo, redo or removal). A dropped
    /// inline edit no longer asks for the editor's focus.
    pub(super) fn drop_session_if(&mut self, gone: impl FnOnce(&Document, &EditSession) -> bool) {
        if self
            .session
            .as_ref()
            .is_some_and(|(s, _)| gone(&self.doc, s))
        {
            if matches!(self.session, Some((EditSession::Box { .. }, _))) {
                self.edit_focus = false;
            }
            self.session = None;
        }
    }

    /// True when ending the session now would record a history step.
    pub fn has_pending_session(&self) -> bool {
        let Some((s, before)) = &self.session else {
            return false;
        };
        match s {
            EditSession::Box { node, fresh } => {
                !fresh && box_text(before, *node) != box_text(&self.doc, *node)
            }
            EditSession::Snippet { id } => snippet_edit::differs(before, &self.doc, *id),
            EditSession::Meta => meta_edit::pending(before, &self.doc),
            EditSession::Essay { slot } => essay_edit::pending(before, &self.doc, *slot),
        }
    }

    /// End the running session, as before a save, a close or any command:
    /// tidy what it edited (a snippet's source; title and author trimmed)
    /// and record the snapshot as exactly one history step if the document
    /// changed. Returns whether it recorded a step.
    pub fn commit_session(&mut self) -> bool {
        let Some((s, before)) = self.session.take() else {
            return false;
        };
        match s {
            EditSession::Box { node, fresh } => {
                self.edit_focus = false;
                let changed = !fresh && box_text(&before, node) != box_text(&self.doc, node);
                self.record_if(changed, before)
            }
            EditSession::Snippet { id } => self.finish_snippet(id, before),
            EditSession::Meta => self.finish_meta(before),
            EditSession::Essay { slot } => {
                let changed = essay_edit::pending(&before, &self.doc, slot);
                self.record_if(changed, before)
            }
        }
    }

    /// Record `before` as a history step and note the change, if `changed`.
    fn record_if(&mut self, changed: bool, before: Document) -> bool {
        if changed {
            self.history.record(before);
            self.bump_revision();
        }
        changed
    }

    /// Start an inline edit of box `b` and select it. `fresh`: `b` was just
    /// added, so its first text belongs to the add's history step.
    pub fn begin_edit(&mut self, b: NodeId, fresh: bool) {
        self.commit_session();
        let Some(node) = self.doc.boxes.get(&b) else {
            return;
        };
        self.edit_text = node.text.clone();
        self.begin_session(EditSession::Box { node: b, fresh });
        self.selection = Some(Item::Box(b));
        self.edit_focus = true;
    }

    /// Show the editor's text for box `b` live in the document, without a
    /// history step, cut to [`MAX_BOX_CHARS`] characters. Text for a box
    /// that is no longer being edited (a late update after the edit ended)
    /// is dropped. Returns whether the text was cut.
    pub fn set_edit_text(&mut self, b: NodeId, mut text: String) -> bool {
        if self.editing() != Some(b) {
            return false;
        }
        let cut = cap_chars(&mut text, MAX_BOX_CHARS);
        let changed = self.doc.boxes.get(&b).is_some_and(|n| n.text != text);
        if self.doc.set_text(b, &text).is_ok() {
            self.edit_text = text;
            if changed {
                self.bump_revision();
            }
        }
        cut
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc_tab::DocTab;

    #[test]
    fn only_one_session_at_a_time() {
        let mut tab = DocTab::new();
        let root = tab.doc.root;
        let id = tab.doc.add_snippet("s");

        tab.begin_edit(root, false);
        tab.set_edit_text(root, "claim".into());
        assert_eq!(tab.editing(), Some(root));
        assert!(tab.history.undo.is_empty(), "typing is not a step yet");

        // A snippet edit ends the box edit: exactly one step for it.
        tab.set_snippet_text(id, "snippet");
        assert_eq!(tab.session(), Some(&EditSession::Snippet { id }));
        assert_eq!(tab.editing(), None);
        assert_eq!(tab.history.undo.len(), 1);

        // A title edit ends the snippet edit: one more step.
        tab.edit_meta_live("Title", "");
        assert_eq!(tab.session(), Some(&EditSession::Meta));
        assert_eq!(tab.history.undo.len(), 2);

        // Starting a box session ends the title edit: one more step.
        tab.begin_session(EditSession::Box {
            node: root,
            fresh: false,
        });
        assert_eq!(tab.editing(), Some(root));
        assert_eq!(tab.history.undo.len(), 3);

        // A changed session records one step when it ends…
        tab.set_edit_text(root, "claim 2".into());
        assert!(tab.has_pending_session());
        assert!(tab.commit_session());
        assert_eq!(tab.history.undo.len(), 4);
        // …and an unchanged one, or none, records nothing.
        tab.begin_session(EditSession::Meta);
        assert!(!tab.commit_session());
        assert!(!tab.commit_session());
        assert_eq!(tab.session(), None);
        assert_eq!(tab.history.undo.len(), 4);
        assert_eq!(tab.doc.meta.title, "Title");
        assert_eq!(tab.doc.boxes[&root].text, "claim 2");
    }
}
