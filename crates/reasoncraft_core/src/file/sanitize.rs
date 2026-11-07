//! What a file may hold besides a valid document: safe entry names,
//! well-formed asset keys and a bounded amount of unknown JSON.

use std::io;

use serde_json::{Map, Value};

use super::MAX_EXTRA_BYTES;
use crate::model::Document;

/// Whether zip entry `name` is a plain relative path that is safe to keep
/// and write back: forward slashes only, no root, drive or `..` part and
/// no control characters.
pub(super) fn safe_entry_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('/')
        && !name.contains(['\\', ':'])
        && !name.chars().any(char::is_control)
        && name.split('/').all(|part| part != "..")
}

/// The image types a key may name.
const IMAGE_EXTENSIONS: [&str; 4] = ["png", "jpg", "jpeg", "webp"];

/// Whether `key` reads `<64 lowercase hex digits>.<image extension>`.
pub(super) fn valid_asset_key(key: &str) -> bool {
    key.split_once('.').is_some_and(|(hash, ext)| {
        hash.len() == 64
            && hash.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
            && IMAGE_EXTENSIONS.contains(&ext)
    })
}

/// Counts the bytes written to it.
struct Count(usize);

impl io::Write for Count {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0 += buf.len();
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Every map of unknown fields in `doc`.
fn extras(doc: &mut Document) -> Vec<&mut Map<String, Value>> {
    let mut all = vec![&mut doc.extra, &mut doc.meta.extra, &mut doc.essay.extra];
    all.extend(doc.essay.unplaced.iter_mut().map(|u| &mut u.extra));
    all.extend(doc.ignored_hints.iter_mut().map(|h| &mut h.extra));
    for b in doc.boxes.values_mut() {
        all.push(&mut b.extra);
        all.extend(b.comments.iter_mut().map(|c| &mut c.extra));
        if let Some(image) = &mut b.image {
            all.push(&mut image.extra);
        }
    }
    all.extend(doc.cards.values_mut().map(|c| &mut c.extra));
    for s in &mut doc.snippets {
        all.push(&mut s.extra);
        if let Some(src) = &mut s.source {
            all.push(&mut src.extra);
        }
    }
    all
}

/// Leave out every unknown field when together they take more than
/// [`MAX_EXTRA_BYTES`] as JSON: all of them, so no half set from a newer
/// build survives. True when they were left out.
pub(super) fn drop_large_extras(doc: &mut Document) -> bool {
    let mut maps = extras(doc);
    let mut size = Count(0);
    for map in maps.iter().filter(|m| !m.is_empty()) {
        // Writing JSON to a counter cannot fail.
        let _ = serde_json::to_writer(&mut size, &**map);
        if size.0 > MAX_EXTRA_BYTES {
            maps.iter_mut().for_each(|m| m.clear());
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_names() {
        for ok in ["a.bin", "future/data.bin", "dir/", "a..b/c"] {
            assert!(safe_entry_name(ok), "{ok}");
        }
        for bad in [
            "",
            "../x",
            "a/../../x",
            "/abs",
            "C:\\x",
            "C:/x",
            "a\\b",
            "a\u{0}b",
            "..",
        ] {
            assert!(!safe_entry_name(bad), "{bad:?}");
        }
    }

    #[test]
    fn asset_keys() {
        let hash = "0123456789abcdef".repeat(4);
        assert!(valid_asset_key(&format!("{hash}.png")));
        assert!(valid_asset_key(&format!("{hash}.jpeg")));
        for bad in [
            format!("{hash}.png/../../x"),
            format!("{hash}.exe"),
            format!("{}.png", hash.to_uppercase()),
            format!("{}.png", &hash[1..]),
            "a.png".to_owned(),
            hash.clone(),
        ] {
            assert!(!valid_asset_key(&bad), "{bad}");
        }
    }
}
