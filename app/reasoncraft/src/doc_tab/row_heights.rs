//! The heights of a virtualised list's rows (the Essay, Check and Snippets
//! tabs), so
//! a frame lays out only the rows near the view and places the others
//! from their heights.

use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use reasoncraft_core::Item;
use reasoncraft_core::checks::CheckKind;
use reasoncraft_core::essay::EssaySlot;

/// A row of the Essay tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EssayRow {
    /// The "How to use" note.
    Help,
    /// A statement with its paragraph field: the introduction, the
    /// conclusion or an element of a section.
    Slot(EssaySlot),
    /// "No sections yet".
    NoSections,
    /// The heading of the section for this item.
    Section(Item),
    /// The heading of the paragraphs not in the essay, and one of them.
    InactiveHeading,
    Inactive(Item),
    /// The heading of the notes of deleted items, and one of them.
    UnplacedHeading,
    Unplaced(usize),
}

/// A row of the Check tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CheckRow {
    /// A check's group heading.
    Heading(CheckKind),
    /// The `n`th issue of a check.
    Issue(CheckKind, usize),
    /// "Show ignored hints".
    Footer,
}

/// Row heights by row key, measured for one document revision and panel
/// width. Heights from an earlier revision or width stay as estimates
/// until their rows are laid out again, so the list does not jump while
/// the user types.
#[derive(Debug)]
pub struct RowHeights<K> {
    /// The revision and width [`Self::heights`] were measured at.
    measured_for: (u64, f32),
    heights: HashMap<K, f32>,
    estimates: HashMap<K, f32>,
    /// Rows laid out in the current (or last) frame.
    laid_out: usize,
    /// The row holding the keyboard focus in the last frame, and the
    /// focused widget.
    pub(crate) focus: Option<(K, eframe::egui::Id)>,
    /// The scroll offset the list takes on its next frame, after rows
    /// above the view were measured at new heights.
    pub(crate) scroll_to: Option<f32>,
    /// A row to bring into view (a new snippet): the list lays it out and
    /// scrolls to it each frame until the rows around it are measured and
    /// it is in view.
    pub reveal: Option<K>,
}

impl<K> Default for RowHeights<K> {
    fn default() -> Self {
        Self {
            measured_for: (0, 0.0),
            heights: HashMap::new(),
            estimates: HashMap::new(),
            laid_out: 0,
            focus: None,
            scroll_to: None,
            reveal: None,
        }
    }
}

impl<K: Eq + Hash> RowHeights<K> {
    /// Starts a frame showing the rows `keys` of `revision` at `width`.
    /// When the revision or width changed, the heights become estimates,
    /// and those of rows no longer in `keys` are forgotten.
    pub fn start_frame(&mut self, revision: u64, width: f32, keys: &[K]) {
        self.laid_out = 0;
        if self.measured_for != (revision, width) {
            self.measured_for = (revision, width);
            self.estimates.extend(self.heights.drain());
            let live: HashSet<&K> = keys.iter().collect();
            self.estimates.retain(|k, _| live.contains(k));
        }
    }

    /// The height of row `key`: as measured for this revision and width,
    /// else as last measured.
    pub fn get(&self, key: &K) -> Option<f32> {
        self.heights
            .get(key)
            .or_else(|| self.estimates.get(key))
            .copied()
    }

    /// Notes that row `key` was laid out this frame, `height` tall.
    pub fn measured(&mut self, key: K, height: f32) {
        self.laid_out += 1;
        self.heights.insert(key, height);
    }

    /// How many rows the current (or last) frame laid out.
    pub fn laid_out(&self) -> usize {
        self.laid_out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heights_outlive_a_change_as_estimates() {
        let mut h = RowHeights::default();
        let keys = ["a", "b"];
        h.start_frame(1, 300.0, &keys);
        h.measured("a", 20.0);
        h.measured("b", 30.0);
        assert_eq!(h.laid_out(), 2);
        h.start_frame(2, 300.0, &keys);
        assert_eq!(h.laid_out(), 0);
        assert_eq!(h.get(&"a"), Some(20.0), "kept as an estimate");
        h.measured("a", 25.0);
        assert_eq!(h.get(&"a"), Some(25.0));
        h.start_frame(2, 250.0, &keys);
        assert_eq!(h.get(&"a"), Some(25.0));
        assert_eq!(h.get(&"b"), Some(30.0));
        assert_eq!(h.get(&"c"), None);
    }

    #[test]
    fn heights_of_removed_rows_are_forgotten() {
        let mut h = RowHeights::default();
        h.start_frame(1, 300.0, &["a", "b"]);
        h.measured("a", 20.0);
        h.measured("b", 30.0);
        // Row "b" was deleted: the next revision forgets its height.
        h.start_frame(2, 300.0, &["a"]);
        assert_eq!(h.get(&"a"), Some(20.0));
        assert_eq!(h.get(&"b"), None);
    }
}
