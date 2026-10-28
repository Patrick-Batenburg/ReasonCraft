//! The document languages: one [`LangPack`] per language holds every word
//! and tag the core (and the app's Word export) needs for it. [`LANGUAGES`]
//! and every per-language lookup derive from [`PACKS`]; a tag that names no
//! pack reads as English in one place, [`pack`].
//!
//! Adding a language means adding a pack here (a test checks it is
//! complete), its dictionary in `reasoncraft_spell` and its `.ftl` file in
//! the app (an app test checks the three lists agree).

use crate::checks::stopwords;
use crate::meta::primary_language;
use crate::words::LinkingWords;

/// Everything language-specific the core writes or reads in a document's
/// language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LangPack {
    /// The language code stored in documents (`"en"`, `"nl"`).
    pub code: &'static str,
    /// The words that link a premise to what it supports or opposes.
    pub linking: LinkingWords,
    /// The essay headings: introduction, section (followed by its number),
    /// conclusion.
    pub introduction: &'static str,
    pub section: &'static str,
    pub conclusion: &'static str,
    /// Function words the key-term check skips: lowercase, folded, sorted.
    pub stopwords: &'static [&'static str],
    /// The "and" and "or" the Compound check looks for.
    pub conjunctions: [&'static str; 2],
    /// The heading above parked fragments in Argdown, Markdown and Word
    /// exports. The first form is written; Argdown import accepts every
    /// form of every pack, so a form that is renamed must stay listed
    /// after the new one for older files.
    ///
    /// This is a file-format word, read back by the Argdown importer
    /// whatever the UI language, so it lives here and not in the app's
    /// `.ftl` files. The map's own label for the area (`parking-title`)
    /// and the check name (`check-kind-not_connected`) are translations
    /// the owner may reword (ruling S6-T4-2); an app test keeps
    /// `parking-title` equal to the written form so exports and the map
    /// agree, and a rewording updates both.
    pub not_connected: &'static [&'static str],
    /// Word's language tag for the document text.
    pub docx_tag: &'static str,
}

/// English: the first pack, and the fallback for any other tag.
const EN: LangPack = LangPack {
    code: "en",
    linking: LinkingWords {
        because: "because",
        but: "but",
        however: "however",
    },
    introduction: "Introduction",
    section: "Section",
    conclusion: "Conclusion",
    stopwords: stopwords::EN,
    conjunctions: ["and", "or"],
    not_connected: &["Not connected"],
    docx_tag: "en-GB",
};

const NL: LangPack = LangPack {
    code: "nl",
    linking: LinkingWords {
        because: "omdat",
        but: "maar",
        however: "echter",
    },
    introduction: "Inleiding",
    section: "Deel",
    conclusion: "Conclusie",
    stopwords: stopwords::NL,
    conjunctions: ["en", "of"],
    not_connected: &["Niet verbonden"],
    docx_tag: "nl-NL",
};

/// Every document language; the first is the fallback.
pub const PACKS: [LangPack; 2] = [EN, NL];

/// The document language codes, in [`PACKS`] order.
pub const LANGUAGES: [&str; PACKS.len()] = {
    let mut codes = [""; PACKS.len()];
    let mut i = 0;
    while i < PACKS.len() {
        codes[i] = PACKS[i].code;
        i += 1;
    }
    codes
};

/// The pack for language tag `tag` (`nl`, `NL`, `nl-NL`, `nl_BE.UTF-8`, ...,
/// matched on its language part ignoring case); English for a tag that
/// names no pack. The only place that falls back.
pub fn pack(tag: &str) -> &'static LangPack {
    let primary = primary_language(tag);
    PACKS
        .iter()
        .find(|p| p.code.eq_ignore_ascii_case(primary))
        .unwrap_or(&PACKS[0])
}

/// Every Argdown heading for the parking area, in all languages.
pub fn not_connected_forms() -> impl Iterator<Item = &'static str> {
    PACKS.iter().flat_map(|p| p.not_connected.iter().copied())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_language_has_a_complete_pack() {
        assert_eq!(LANGUAGES, ["en", "nl"]);
        for (code, p) in LANGUAGES.iter().zip(&PACKS) {
            assert_eq!(pack(code), p, "{code}");
            let l = p.linking;
            let words = [l.because, l.but, l.however, p.introduction, p.section];
            for w in words.into_iter().chain([p.conclusion]) {
                assert!(!w.trim().is_empty(), "{code}: empty word");
            }
            assert!(p.conjunctions.iter().all(|w| !w.is_empty()), "{code}");
            assert!(!p.not_connected.is_empty(), "{code}: no parking heading");
            assert!(p.not_connected.iter().all(|w| !w.trim().is_empty()));
            assert!(p.stopwords.len() >= 100, "{code}: stopwords");
            assert!(p.stopwords.windows(2).all(|w| w[0] < w[1]), "{code}");
            let tag = p.docx_tag;
            assert!(primary_language(tag).eq_ignore_ascii_case(code), "{tag}");
        }
        let unique = |v: Vec<&str>| v.iter().all(|a| v.iter().filter(|b| *b == a).count() == 1);
        assert!(unique(LANGUAGES.to_vec()), "a language is listed twice");
        assert!(unique(not_connected_forms().collect()), "shared heading");
    }

    #[test]
    fn tags_find_their_pack_and_others_read_english() {
        for tag in ["nl", "NL", "nl-NL", "nl_be", "Nl-nl", "nl_BE.UTF-8"] {
            assert_eq!(pack(tag).code, "nl", "{tag}");
        }
        for tag in ["", "en", "en-US", "nld", "fr-NL", "n", "de"] {
            assert_eq!(pack(tag).code, "en", "{tag}");
        }
    }
}
