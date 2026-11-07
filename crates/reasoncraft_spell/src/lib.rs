//! Offline English and Dutch spell checking on bundled Hunspell dictionaries.

#[cfg(test)]
mod tests;
mod tokens;

use std::ops::Range;

pub use tokens::words;

/// `word` as a personal dictionary keeps it (trimmed), or `None` for a
/// word no dictionary can hold: a blank one, or one holding `/` (the
/// dictionary would read what follows as affix flags).
pub fn personal_word(word: &str) -> Option<&str> {
    let word = word.trim();
    (!word.is_empty() && !word.contains('/')).then_some(word)
}

/// The longest word that gets suggestions, in characters (Hunspell's
/// MAXWORDLEN): finding them takes time superlinear in the length, and a
/// worker cannot be stopped half way.
pub const MAX_SUGGEST_CHARS: usize = 100;

/// Supported dictionary languages.
pub const LANGUAGES: [&str; 2] = ["en", "nl"];

#[derive(Debug, thiserror::Error)]
pub enum SpellError {
    #[error("unsupported language {0}")]
    Unsupported(String),
    #[error("dictionary failed to load: {0}")]
    Parse(String),
}

/// A loaded dictionary plus the user's personal words.
pub struct Speller {
    dict: spellbook::Dictionary,
}

impl Speller {
    pub fn load(lang: &str) -> Result<Speller, SpellError> {
        let (aff, dic) = match lang {
            "en" => (
                include_str!("../dict/en/index.aff"),
                include_str!("../dict/en/index.dic"),
            ),
            "nl" => (
                include_str!("../dict/nl/index.aff"),
                include_str!("../dict/nl/index.dic"),
            ),
            other => return Err(SpellError::Unsupported(other.to_string())),
        };
        let dict =
            spellbook::Dictionary::new(aff, dic).map_err(|e| SpellError::Parse(e.to_string()))?;
        Ok(Speller { dict })
    }

    /// Whether a single token is spelled correctly. Hyphenated tokens pass when the
    /// whole token checks or every part does.
    pub fn check_word(&self, word: &str) -> bool {
        let word = word.replace('’', "'");
        if self.dict.check(&word) {
            return true;
        }
        word.contains('-')
            && word
                .split('-')
                .all(|part| part.is_empty() || self.dict.check(part))
    }

    /// Byte ranges of misspelled words in `text`.
    pub fn misspelled(&self, text: &str) -> Vec<Range<usize>> {
        words(text)
            .into_iter()
            .filter(|r| !self.check_word(&text[r.clone()]))
            .collect()
    }

    /// Up to `max` corrections for `word`; none for a word longer than
    /// [`MAX_SUGGEST_CHARS`].
    pub fn suggest(&self, word: &str, max: usize) -> Vec<String> {
        let mut out = Vec::new();
        if word.chars().nth(MAX_SUGGEST_CHARS).is_some() {
            return out;
        }
        self.dict.suggest(&word.replace('’', "'"), &mut out);
        out.truncate(max);
        out
    }

    /// Adds a word to the personal dictionary for this session. Returns
    /// false, adding nothing, for a word [`personal_word`] refuses.
    pub fn add_word(&mut self, word: &str) -> bool {
        let Some(word) = personal_word(word) else {
            return false;
        };
        self.dict.add(&word.replace('’', "'")).is_ok()
    }
}
