//! Edit operations for the document's metadata: title, author, language.

use crate::edit::EditError;
use crate::model::Document;

/// The languages checks know about.
pub const LANGUAGES: [&str; 2] = ["en", "nl"];

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
            self.finish_meta();
        }
        Ok(())
    }

    /// Set the title and author as given. Leaving both as they are changes
    /// nothing, not even the modified time.
    pub fn set_meta(&mut self, title: &str, author: &str) -> Result<(), EditError> {
        if self.meta.title != title || self.meta.author != author {
            title.clone_into(&mut self.meta.title);
            author.clone_into(&mut self.meta.author);
            self.finish_meta();
        }
        Ok(())
    }

    fn finish_meta(&mut self) {
        self.touch();
        #[cfg(debug_assertions)]
        self.check_invariants();
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
