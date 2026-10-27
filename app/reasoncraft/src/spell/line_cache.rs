//! The misspellings found per (language, line hash), kept for the
//! [`LINE_CACHE`] or so lines used most recently. Two generations: lines
//! go into the new one, a line found in the old one moves up, and once
//! the new one is full the old one is dropped whole. Every step is O(1).

use std::collections::HashMap;

use super::Ranges;

/// How many lines keep their misspellings cached (at least half of it,
/// the lines used last, are always kept).
pub(super) const LINE_CACHE: usize = 4096;

type Key = (String, u64);

#[derive(Default)]
pub(super) struct LineCache {
    new: HashMap<Key, Ranges>,
    old: HashMap<Key, Ranges>,
}

impl LineCache {
    /// The misspellings kept for `key`, if any.
    pub(super) fn get(&mut self, key: &Key) -> Option<Ranges> {
        if let Some(ranges) = self.new.get(key) {
            return Some(ranges.clone());
        }
        let ranges = self.old.remove(key)?;
        self.insert(key.clone(), ranges.clone());
        Some(ranges)
    }

    pub(super) fn insert(&mut self, key: Key, ranges: Ranges) {
        if self.new.len() >= LINE_CACHE / 2 {
            self.old = std::mem::take(&mut self.new);
        }
        self.new.insert(key, ranges);
    }

    pub(super) fn clear(&mut self) {
        self.new.clear();
        self.old.clear();
    }

    /// How many lines are kept.
    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.new.len() + self.old.len()
    }
}
