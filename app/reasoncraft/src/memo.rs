//! A value worked out for a key and kept until it is asked for with
//! another key: the caches of a tab's outline, issues, essay view and find
//! hits, and of the minimap's picture.

/// The value last made, with the key it was made for.
#[derive(Clone, Debug)]
pub struct Memo<K, V> {
    slot: Option<(K, V)>,
}

impl<K, V> Default for Memo<K, V> {
    fn default() -> Self {
        Self { slot: None }
    }
}

impl<K: PartialEq, V> Memo<K, V> {
    /// The value for `key`: the kept one when it was made for `key`, else
    /// a new one from `make`, kept in its place.
    pub fn get(&mut self, key: K, make: impl FnOnce() -> V) -> &V {
        if self.slot.as_ref().is_none_or(|(k, _)| *k != key) {
            self.slot = Some((key, make()));
        }
        &self.slot.as_ref().expect("just made").1
    }

    /// The key the kept value was made for.
    pub fn key(&self) -> Option<&K> {
        self.slot.as_ref().map(|(k, _)| k)
    }

    /// The value last made, whatever it was made for.
    pub fn last(&self) -> Option<&V> {
        self.slot.as_ref().map(|(_, v)| v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn makes_again_only_for_a_new_key() {
        let mut memo = Memo::default();
        let mut made = 0;
        let mut get = |memo: &mut Memo<u8, u8>, key| {
            *memo.get(key, || {
                made += 1;
                key * 2
            })
        };
        assert_eq!(memo.last(), None);
        assert_eq!(get(&mut memo, 1), 2);
        assert_eq!(get(&mut memo, 1), 2);
        assert_eq!(get(&mut memo, 3), 6);
        assert_eq!(memo.last(), Some(&6));
        assert_eq!(memo.key(), Some(&3));
        drop(get);
        assert_eq!(made, 2);
    }
}
