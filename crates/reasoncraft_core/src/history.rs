//! Snapshot-based undo/redo.

use crate::model::Document;

/// Undo/redo stacks of whole-document snapshots.
///
/// Snapshots cannot drift out of sync with the editing rules, and documents
/// are small enough for this to be cheap.
#[derive(Debug, Clone)]
pub struct History {
    pub undo: Vec<Document>,
    pub redo: Vec<Document>,
    pub limit: usize,
    /// Undo-stack depth at the last save, or `None` if that state is no longer reachable.
    pub saved_marker: Option<usize>,
}

impl History {
    /// A fresh history is clean (saved at depth 0).
    pub fn new(limit: usize) -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            limit,
            saved_marker: Some(0),
        }
    }

    /// Call before applying an edit; clears redo.
    pub fn record(&mut self, before: &Document) {
        // The saved position lives in the redo branch we are about to discard.
        if self.saved_marker.is_some_and(|m| m > self.undo.len()) {
            self.saved_marker = None;
        }
        self.redo.clear();
        self.undo.push(before.clone());
        if self.undo.len() > self.limit {
            self.undo.remove(0);
            // Everything shifts down one; a marker at 0 is no longer reachable.
            self.saved_marker = self.saved_marker.and_then(|m| m.checked_sub(1));
        }
    }

    /// Step back. Returns false when there is nothing to undo.
    pub fn undo(&mut self, current: &mut Document) -> bool {
        let Some(prev) = self.undo.pop() else {
            return false;
        };
        self.redo.push(std::mem::replace(current, prev));
        true
    }

    /// Step forward. Returns false when there is nothing to redo.
    pub fn redo(&mut self, current: &mut Document) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        self.undo.push(std::mem::replace(current, next));
        true
    }

    pub fn mark_saved(&mut self) {
        self.saved_marker = Some(self.undo.len());
    }

    /// True when the current position differs from the saved one.
    pub fn is_dirty(&self) -> bool {
        self.saved_marker != Some(self.undo.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edit::{NewLoose, RemoveMode};
    use crate::ids::{Item, Parent};
    use crate::model::CardKind;

    #[test]
    fn undo_restores_parking() {
        let mut d = Document::new("C");
        let (card, _) = d
            .add_card(Parent::Box(d.root), CardKind::Reason, "P")
            .unwrap();
        d.add_loose(NewLoose::Note, "loose");
        let mut h = History::new(500);
        let before = d.clone();
        h.record(&d);
        d.remove_item(Item::Card(card), RemoveMode::KeepChildren)
            .unwrap();
        let after = d.clone();
        assert!(h.undo(&mut d));
        assert_eq!(d, before);
        assert_eq!(d.parking, before.parking);
        assert!(h.redo(&mut d));
        assert_eq!(d, after);
        assert!(!h.redo(&mut d));
    }

    #[test]
    fn new_edit_clears_redo() {
        let mut d = Document::new("C");
        let mut h = History::new(500);
        h.record(&d);
        d.set_text(d.root, "A").unwrap();
        assert!(h.undo(&mut d));
        assert_eq!(h.redo.len(), 1);
        h.record(&d);
        d.set_text(d.root, "B").unwrap();
        assert!(h.redo.is_empty());
        assert!(!h.redo(&mut d));
    }

    #[test]
    fn undo_on_empty_returns_false() {
        let mut d = Document::new("C");
        let mut h = History::new(5);
        assert!(!h.undo(&mut d));
    }

    #[test]
    fn limit_drops_oldest() {
        let mut d = Document::new("C");
        let mut h = History::new(3);
        for i in 0..5 {
            h.record(&d);
            d.set_text(d.root, &format!("t{i}")).unwrap();
        }
        assert_eq!(h.undo.len(), 3);
        while h.undo(&mut d) {}
        // Oldest two snapshots ("C", "t0") were dropped.
        assert_eq!(d.boxes[&d.root].text, "t1");
    }

    #[test]
    fn dirty_tracks_saved_position() {
        let mut d = Document::new("C");
        let mut h = History::new(500);
        assert!(!h.is_dirty());
        h.record(&d);
        d.set_text(d.root, "C2").unwrap();
        assert!(h.is_dirty());
        h.mark_saved();
        assert!(!h.is_dirty());
        h.undo(&mut d);
        assert!(h.is_dirty());
        h.redo(&mut d);
        assert!(!h.is_dirty());
    }

    #[test]
    fn dirty_after_saved_state_pushed_out_by_limit() {
        let mut d = Document::new("C");
        let mut h = History::new(2);
        h.mark_saved(); // depth 0
        for i in 0..3 {
            h.record(&d);
            d.set_text(d.root, &format!("t{i}")).unwrap();
        }
        assert!(h.is_dirty());
        while h.undo(&mut d) {}
        assert!(h.is_dirty());
    }

    #[test]
    fn marker_shifts_when_limit_drops_oldest() {
        let mut d = Document::new("C");
        let mut h = History::new(3);
        for i in 0..2 {
            h.record(&d);
            d.set_text(d.root, &format!("t{i}")).unwrap();
        }
        h.mark_saved(); // depth 2
        h.record(&d);
        d.set_text(d.root, "t2").unwrap(); // depth 3
        h.record(&d); // drops oldest; marker shifts 2 -> 1, still reachable
        d.set_text(d.root, "t3").unwrap();
        assert!(h.is_dirty());
        assert!(h.undo(&mut d));
        assert!(h.undo(&mut d));
        assert!(!h.is_dirty());
        assert_eq!(d.boxes[&d.root].text, "t1");
    }

    #[test]
    fn record_in_redo_branch_discards_saved_marker() {
        let mut d = Document::new("C");
        let mut h = History::new(500);
        h.record(&d);
        d.set_text(d.root, "A").unwrap();
        h.mark_saved(); // depth 1
        h.undo(&mut d);
        h.record(&d);
        d.set_text(d.root, "B").unwrap(); // depth 1 again, different content
        assert!(h.is_dirty());
    }
}
