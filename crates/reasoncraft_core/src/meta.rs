//! Edit operations for the document's metadata: title, author, language.

use crate::caps::{MAX_META_CHARS, capped};
use crate::edit::EditError;
use crate::model::Document;

/// The languages checks know about.
pub const LANGUAGES: [&str; 2] = ["en", "nl"];

/// The language part of a language tag: `nl` of `nl`, `nl-NL`, `nl_BE`,
/// `nl_BE.UTF-8` or `nl@euro`. The one rule for reading tags, in the core
/// and the app; compare it ignoring case.
pub fn primary_language(tag: &str) -> &str {
    tag.split(['-', '_', '.', '@']).next().unwrap_or_default()
}

/// The language of [`LANGUAGES`] that `tag` names: `"nl"` for Dutch in any
/// case or region (`nl-NL`, `NL_be`), `"en"` for anything else.
pub fn language_code(tag: &str) -> &'static str {
    if primary_language(tag).eq_ignore_ascii_case("nl") {
        "nl"
    } else {
        "en"
    }
}

impl Document {
    /// Set the document language: `"en"` or `"nl"`, else
    /// [`EditError::NotAllowed`]. Setting the current language changes
    /// nothing, not even the modified time.
    pub fn set_language(&mut self, lang: &str) -> Result<(), EditError> {
        if !LANGUAGES.contains(&lang) {
            return Err(EditError::NotAllowed);
        }
        if self.meta.language != lang {
            lang.clone_into(&mut self.meta.language);
            self.finish_edit();
        }
        Ok(())
    }

    /// Set the title and author as given, each cut to [`MAX_META_CHARS`]
    /// characters. Leaving both as they are changes nothing, not even the
    /// modified time.
    pub fn set_meta(&mut self, title: &str, author: &str) -> Result<(), EditError> {
        let (title, author) = (
            capped(title, MAX_META_CHARS),
            capped(author, MAX_META_CHARS),
        );
        if self.meta.title != title || self.meta.author != author {
            title.clone_into(&mut self.meta.title);
            author.clone_into(&mut self.meta.author);
            self.finish_edit();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_language_validates() {
        let mut d = Document::new("c");
        assert_eq!(d.set_language("nl"), Ok(()));
        assert_eq!(d.meta.language, "nl");
        for bad in ["", "NL", "nl-NL", "fr"] {
            assert_eq!(d.set_language(bad), Err(EditError::NotAllowed), "{bad}");
        }
        assert_eq!(d.meta.language, "nl", "a refusal changes nothing");
    }

    #[test]
    fn unchanged_values_do_not_touch() {
        let mut d = Document::new("c");
        d.meta.modified = "old".to_owned();
        d.set_language("en").unwrap();
        d.set_meta("", "").unwrap();
        assert_eq!(d.meta.modified, "old");
        d.set_meta("T", "A").unwrap();
        assert_eq!((d.meta.title.as_str(), d.meta.author.as_str()), ("T", "A"));
        assert_ne!(d.meta.modified, "old");
    }
}
