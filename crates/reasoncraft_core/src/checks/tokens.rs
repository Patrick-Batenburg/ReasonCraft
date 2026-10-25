//! Turning box text into comparable terms for the key-term check.
//!
//! Text is lowercased, diacritics are folded ("coördinatie" → "coordinatie"),
//! apostrophe suffixes are dropped ("auto's" → "auto") and words are split
//! on anything that is not a letter or digit. Hyphenated words are split
//! into parts, but their joined form is kept aside so "e-mail" and "email"
//! still match.

use super::stopwords;

/// Shortest term that counts.
const MIN_TERM_CHARS: usize = 4;

/// One word of the text: its parts (more than one when hyphenated), each
/// lowercased and folded, before stopwords and short parts are dropped.
#[derive(Debug, Default)]
pub(super) struct Word {
    pub(super) parts: Vec<String>,
}

impl Word {
    /// The parts written together ("e-mail" → "email"), for hyphenated words.
    pub(super) fn joined(&self) -> Option<String> {
        (self.parts.len() > 1).then(|| self.parts.concat())
    }
}

/// The comparable terms of `text`: lowercased, folded, apostrophe suffixes
/// dropped, split on non-alphanumerics (hyphenated words into parts), with
/// stopwords of `lang` (Dutch for `"nl"`, `"nl-NL"`, ...; anything else is
/// English) and terms shorter than four characters removed.
pub fn tokenise(text: &str, lang: &str) -> Vec<String> {
    words(text)
        .into_iter()
        .flat_map(|w| w.parts)
        .filter(|t| is_term(t, lang))
        .collect()
}

/// Whether a folded part is long enough and not a stopword.
pub(super) fn is_term(token: &str, lang: &str) -> bool {
    token.chars().count() >= MIN_TERM_CHARS && !stopwords::is_stopword(token, lang)
}

/// Whether two terms count as the same word: equal, or sharing a prefix of
/// at least `max(4, shorter - 2)` characters ("automatisering" and
/// "automatiseren", "robot" and "robots").
pub(super) fn same_term(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    let shorter = a.chars().count().min(b.chars().count());
    let shared = a.chars().zip(b.chars()).take_while(|(x, y)| x == y).count();
    shared >= MIN_TERM_CHARS.max(shorter.saturating_sub(2))
}

/// Split `text` into words of lowercased, folded parts. Nothing is dropped
/// here except apostrophe suffixes.
pub(super) fn words(text: &str) -> Vec<Word> {
    let mut out = Vec::new();
    let mut word = Word::default();
    let mut part = String::new();
    // Inside an apostrophe suffix ("'s" of "auto's"): letters are skipped.
    let mut in_suffix = false;
    for c in text.chars().flat_map(char::to_lowercase) {
        if is_combining_mark(c) {
            continue;
        }
        if c.is_alphanumeric() {
            if !in_suffix {
                match fold(c) {
                    Some(s) => part.push_str(s),
                    None => part.push(c),
                }
            }
        } else if is_apostrophe(c) && !part.is_empty() {
            in_suffix = true;
        } else if is_hyphen(c) && !part.is_empty() {
            word.parts.push(std::mem::take(&mut part));
            in_suffix = false;
        } else {
            if !part.is_empty() {
                word.parts.push(std::mem::take(&mut part));
            }
            if !word.parts.is_empty() {
                out.push(std::mem::take(&mut word));
            }
            in_suffix = false;
        }
    }
    if !part.is_empty() {
        word.parts.push(part);
    }
    if !word.parts.is_empty() {
        out.push(word);
    }
    out
}

fn is_apostrophe(c: char) -> bool {
    matches!(
        c,
        '\'' | '\u{2019}' | '\u{2018}' | '\u{02BC}' | '`' | '\u{00B4}'
    )
}

fn is_hyphen(c: char) -> bool {
    matches!(c, '-' | '\u{2010}' | '\u{2011}')
}

/// Combining diacritical marks, as in decomposed text ("e" + U+0301).
fn is_combining_mark(c: char) -> bool {
    matches!(c, '\u{0300}'..='\u{036F}')
}

/// The plain form of a lowercase Latin-1 or Latin Extended-A letter;
/// `None` when the letter has no diacritic to strip.
fn fold(c: char) -> Option<&'static str> {
    Some(match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' => "a",
        'æ' => "ae",
        'ç' | 'ć' | 'ĉ' | 'ċ' | 'č' => "c",
        'ď' | 'đ' | 'ð' => "d",
        'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ĕ' | 'ė' | 'ę' | 'ě' => "e",
        'ĝ' | 'ğ' | 'ġ' | 'ģ' => "g",
        'ĥ' | 'ħ' => "h",
        'ì' | 'í' | 'î' | 'ï' | 'ĩ' | 'ī' | 'ĭ' | 'į' | 'ı' => "i",
        'ĳ' => "ij",
        'ĵ' => "j",
        'ķ' => "k",
        'ĺ' | 'ļ' | 'ľ' | 'ŀ' | 'ł' => "l",
        'ñ' | 'ń' | 'ņ' | 'ň' | 'ŉ' => "n",
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ŏ' | 'ő' => "o",
        'œ' => "oe",
        'ŕ' | 'ŗ' | 'ř' => "r",
        'ś' | 'ŝ' | 'ş' | 'š' => "s",
        'ß' => "ss",
        'ţ' | 'ť' | 'ŧ' => "t",
        'þ' => "th",
        'ù' | 'ú' | 'û' | 'ü' | 'ũ' | 'ū' | 'ŭ' | 'ů' | 'ű' | 'ų' => "u",
        'ŵ' => "w",
        'ý' | 'ÿ' | 'ŷ' => "y",
        'ź' | 'ż' | 'ž' => "z",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folds_case_diacritics_and_apostrophes() {
        assert_eq!(
            tokenise("Coördinatie van AUTO'S", "nl"),
            ["coordinatie", "auto"]
        );
        assert_eq!(tokenise("Één ÇAFÉ Straße", "en"), ["cafe", "strasse"]);
        assert_eq!(tokenise("Ĳzer, Łódź", "nl"), ["ijzer", "lodz"]);
        // Decomposed "é" (e + combining acute) folds the same way.
        assert_eq!(tokenise("caf\u{0065}\u{0301}s", "en"), ["cafes"]);
        assert_eq!(tokenise("’s Avonds; robot’s", "nl"), ["avonds", "robot"]);
        assert_eq!(
            tokenise("children's 'quoted' words", "en"),
            ["children", "quoted", "words"]
        );
    }

    #[test]
    fn splits_hyphens_and_punctuation() {
        assert_eq!(tokenise("E-mail-adres", "nl"), ["mail", "adres"]);
        assert_eq!(tokenise("robots - banen", "nl"), ["robots", "banen"]);
        assert_eq!(tokenise("x/y,data.2024!", "en"), ["data", "2024"]);
        let w = words("e-mail post");
        assert_eq!(w[0].joined().as_deref(), Some("email"));
        assert_eq!(w[1].joined(), None);
    }

    #[test]
    fn drops_stopwords_per_language() {
        assert_eq!(
            tokenise("Robots zorgen voor meer banen", "nl"),
            ["robots", "zorgen", "banen"]
        );
        // "voor" is no English stopword; "more" is.
        assert_eq!(tokenise("voor more jobs", "en"), ["voor", "jobs"]);
        assert_eq!(tokenise("these robots", "xx"), ["robots"]);
        assert_eq!(
            tokenise("Robots shouldn't, mustn’t or cannot", "en"),
            ["robots"]
        );
        assert!(tokenise("", "en").is_empty());
        assert!(tokenise("  ?! - ' ", "en").is_empty());
    }

    #[test]
    fn same_term_rule() {
        assert!(same_term("robot", "robots"));
        assert!(same_term("automatisering", "automatiseren"));
        assert!(same_term("banen", "banen"));
        assert!(!same_term("baan", "banen"));
        assert!(!same_term("mail", "email"));
        assert!(!same_term("banen", "bananen"));
        // A short shared start is not enough for longer words.
        assert!(!same_term("democratie", "demografie"));
    }
}
