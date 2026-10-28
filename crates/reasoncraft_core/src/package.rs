//! The in-memory form of a `.rcraft` file: the document, its image assets
//! and any zip entries this build does not understand.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use sha2::{Digest, Sha256};

use crate::model::Document;

/// Image bytes keyed by `"<sha256-hex>.<ext>"`. Cloning is cheap.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Assets {
    entries: BTreeMap<String, Arc<[u8]>>,
}

impl Assets {
    /// Store `bytes` and return their key. Equal bytes share one key.
    pub fn insert(&mut self, ext: &str, bytes: Vec<u8>) -> String {
        let key = asset_key(ext, &bytes);
        self.entries.insert(key.clone(), bytes.into());
        key
    }

    /// Store bytes read from a file under the key it was saved with.
    pub(crate) fn insert_raw(&mut self, key: String, bytes: Vec<u8>) {
        self.entries.insert(key, bytes.into());
    }

    pub fn get(&self, key: &str) -> Option<Arc<[u8]>> {
        self.entries.get(key).cloned()
    }

    /// Copy asset `key` from `other`, if it has it (a pasted image).
    ///
    /// Assets are never dropped while a document is open: undo and redo
    /// may bring back a box that shows one. Saving writes only the assets
    /// the document refers to (see [`Self::referenced`]).
    pub fn copy_from(&mut self, other: &Assets, key: &str) {
        if let Some(bytes) = other.entries.get(key) {
            self.entries.insert(key.to_owned(), bytes.clone());
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The assets `doc` refers to, in key order.
    pub(crate) fn referenced<'a>(
        &'a self,
        doc: &Document,
    ) -> impl Iterator<Item = (&'a str, &'a Arc<[u8]>)> {
        let used = referenced_keys(doc);
        self.entries
            .iter()
            .filter(move |(k, _)| used.contains(k.as_str()))
            .map(|(k, v)| (k.as_str(), v))
    }
}

fn referenced_keys(doc: &Document) -> BTreeSet<&str> {
    doc.boxes
        .values()
        .filter_map(|b| b.image.as_ref())
        .map(|i| i.asset.as_str())
        .collect()
}

/// `"<sha256-hex>.<ext>"` for `bytes`.
pub fn asset_key(ext: &str, bytes: &[u8]) -> String {
    use std::fmt::Write;
    let digest = Sha256::digest(bytes);
    let mut key = String::with_capacity(digest.len() * 2 + 1 + ext.len());
    for b in digest {
        let _ = write!(key, "{b:02x}");
    }
    key.push('.');
    key.push_str(ext);
    key
}

/// Everything in a `.rcraft` file.
///
/// `other_entries` holds every zip entry that is neither `document.json`
/// nor under `images/`, in original order, so files from newer builds keep
/// their extras across open and save. Their bytes are shared: cloning a
/// package (each autosave does) copies no entry.
///
/// `cut_on_open` is true when opening the file cut an over-long text or
/// left out data that was too large or unsafe (see [`crate::file::open`]);
/// `parked_on_open` is true when it moved branches nested deeper than
/// [`crate::MAX_DEPTH`] levels to the parking area. Saving ignores both.
#[derive(Debug, Clone, PartialEq)]
pub struct Package {
    pub doc: Document,
    pub assets: Assets,
    pub other_entries: Vec<(String, Arc<[u8]>)>,
    pub cut_on_open: bool,
    pub parked_on_open: bool,
}

impl Package {
    pub fn new(doc: Document) -> Self {
        Self {
            doc,
            assets: Assets::default(),
            other_entries: Vec::new(),
            cut_on_open: false,
            parked_on_open: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_key_is_content_hash() {
        let mut a = Assets::default();
        let k1 = a.insert("png", vec![1, 2, 3]);
        let k2 = a.insert("png", vec![1, 2, 3]);
        assert_eq!(k1, k2);
        assert!(k1.ends_with(".png"));
        assert_eq!(k1.len(), 64 + 4);
        assert_eq!(a.len(), 1);
        assert_ne!(k1, a.insert("png", vec![9]));
        assert_eq!(&*a.get(&k1).unwrap(), &[1, 2, 3]);
    }

    #[test]
    fn known_digest() {
        // SHA-256 of the empty input.
        assert_eq!(
            asset_key("png", b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855.png"
        );
    }
}
