//! Length caps on the texts a document holds. Editing cuts longer text,
//! and opening a file cuts what a hostile or damaged file holds, so no
//! text the UI lays out, paints or exports is unbounded.

use crate::model::{BoxKind, Document, MAX_BOX_CHARS, SourceType};

/// The most characters a comment's text keeps.
pub const MAX_COMMENT_CHARS: usize = 50_000;
/// The most characters a title, an author's name or a date keeps.
pub const MAX_META_CHARS: usize = 1_000;
/// The longest link kept, in characters. A longer link is refused, not
/// cut: a cut link would lead somewhere else.
pub const MAX_LINK_CHARS: usize = 8_192;

/// The byte length of `text` cut to at most `max` characters (always on
/// a char boundary).
pub fn capped_len(text: &str, max: usize) -> usize {
    match text.char_indices().nth(max) {
        Some((end, _)) => end,
        None => text.len(),
    }
}

/// `text` cut to at most `max` characters.
pub fn capped(text: &str, max: usize) -> &str {
    &text[..capped_len(text, max)]
}

/// Cut `text` to at most `max` characters; true when it was longer.
pub fn cap_chars(text: &mut String, max: usize) -> bool {
    let end = capped_len(text, max);
    let cut = end < text.len();
    text.truncate(end);
    cut
}

/// Whether `link` has at most [`MAX_LINK_CHARS`] characters.
pub(crate) fn link_fits(link: &str) -> bool {
    capped_len(link, MAX_LINK_CHARS) == link.len()
}

/// Cut the name of a source type from a newer build to
/// [`MAX_META_CHARS`]; true when it was longer.
fn cap_source_type(source_type: &mut SourceType) -> bool {
    match source_type {
        SourceType::Other(name) => cap_chars(name, MAX_META_CHARS),
        _ => false,
    }
}

impl Document {
    /// Cut every text past its cap, as editing does, and drop links that
    /// are too long, so an oversized text from a file never reaches the
    /// UI. Not an edit: the modified time is left alone. True when
    /// anything changed.
    pub(crate) fn cap_texts(&mut self) -> bool {
        let mut cut = false;
        let meta = &mut self.meta;
        for text in [
            &mut meta.title,
            &mut meta.author,
            &mut meta.created,
            &mut meta.modified,
            &mut meta.language,
        ] {
            cut |= cap_chars(text, MAX_META_CHARS);
        }
        for node in self.boxes.values_mut() {
            cut |= cap_chars(&mut node.text, MAX_BOX_CHARS);
            if let BoxKind::Source { source_type } = &mut node.kind {
                cut |= cap_source_type(source_type);
            }
            if node.link.as_deref().is_some_and(|l| !link_fits(l)) {
                node.link = None;
                cut = true;
            }
            for c in &mut node.comments {
                cut |= cap_chars(&mut c.text, MAX_COMMENT_CHARS);
                cut |= cap_chars(&mut c.author, MAX_META_CHARS);
                cut |= cap_chars(&mut c.created, MAX_META_CHARS);
            }
        }
        for source in self.snippets.iter_mut().filter_map(|s| s.source.as_mut()) {
            cut |= cap_chars(&mut source.citation, MAX_BOX_CHARS);
            cut |= cap_source_type(&mut source.source_type);
            if source.link.as_deref().is_some_and(|l| !link_fits(l)) {
                source.link = None;
                cut = true;
            }
        }
        cut |= self.cap_snippet_texts();
        cut |= self.cap_essay_texts();
        cut
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::Parent;
    use crate::model::{BoxKind, CardKind};

    fn chars(s: &str) -> usize {
        s.chars().count()
    }

    #[test]
    fn cap_chars_cuts_on_char_boundaries() {
        let mut s = "é日a".repeat(3);
        assert!(!cap_chars(&mut s, 9));
        assert!(cap_chars(&mut s, 4));
        assert_eq!(s, "é日aé");
        assert_eq!(capped("日本", 1), "日");
        assert_eq!(capped("", 0), "");
    }

    #[test]
    fn set_text_caps_and_says_so() {
        let mut d = Document::new("c");
        let root = d.root;
        assert_eq!(d.set_text(root, "short"), Ok(false));
        let long = "é".repeat(MAX_BOX_CHARS + 5);
        assert_eq!(d.set_text(root, &long), Ok(true));
        assert_eq!(chars(&d.boxes[&root].text), MAX_BOX_CHARS);
    }

    #[test]
    fn new_boxes_are_capped() {
        let long = "x".repeat(MAX_BOX_CHARS + 1);
        let mut d = Document::new(&long);
        assert_eq!(chars(&d.boxes[&d.root].text), MAX_BOX_CHARS);
        let (_, p) = d
            .add_card(Parent::Box(d.root), CardKind::Reason, &long)
            .unwrap();
        assert_eq!(chars(&d.boxes[&p].text), MAX_BOX_CHARS);
        let n = d.add_box(Parent::Box(p), BoxKind::Note, &long).unwrap();
        assert_eq!(chars(&d.boxes[&n].text), MAX_BOX_CHARS);
    }

    #[test]
    fn comments_meta_and_links_are_capped() {
        let mut d = Document::new("c");
        let root = d.root;
        let long = "a".repeat(MAX_COMMENT_CHARS + 1);
        d.add_comment(root, &long, &long, None).unwrap();
        let c = &d.boxes[&root].comments[0];
        assert_eq!(chars(&c.text), MAX_COMMENT_CHARS);
        assert_eq!(chars(&c.author), MAX_META_CHARS);
        d.set_meta(&long, &long).unwrap();
        assert_eq!(chars(&d.meta.title), MAX_META_CHARS);
        assert_eq!(chars(&d.meta.author), MAX_META_CHARS);
        let link = format!("https://example.org/{}", "a".repeat(MAX_LINK_CHARS));
        assert_eq!(
            d.set_link(root, Some(link)),
            Err(crate::edit::EditError::NotAllowed)
        );
        let fits = format!("https://e.org/{}", "a".repeat(MAX_LINK_CHARS - 14));
        d.set_link(root, Some(fits.clone())).unwrap();
        assert_eq!(d.boxes[&root].link, Some(fits));
    }
}
