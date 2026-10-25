//! Stopword lists for the key-term check: common function words
//! (articles, pronouns, prepositions, auxiliaries, conjunctions and
//! quantifiers) that say nothing about the subject of a statement. The
//! words are lowercase and folded (no diacritics) and sorted, so lookups
//! use a binary search.

/// English function words, plus the stems left of negative contractions
/// once the apostrophe suffix is dropped ("shouldn't" -> "shouldn").
#[rustfmt::skip]
pub(super) const EN: &[&str] = &[
    "a", "about", "above", "across", "after", "again", "against", "all", "already", "also",
    "although", "always", "am", "among", "an", "and", "another", "any", "anything", "are", "aren",
    "around", "as", "at", "be", "because", "been", "before", "being", "below", "between", "both",
    "but", "by", "can", "cannot", "could", "couldn", "did", "didn", "do", "does", "doesn", "doing",
    "down", "during", "each", "either", "else", "enough", "even", "ever", "every", "everything",
    "few", "for", "from", "further", "had", "hadn", "has", "hasn", "have", "haven", "having", "he",
    "her", "here", "hers", "herself", "him", "himself", "his", "how", "however", "i", "if", "in",
    "into", "is", "it", "its", "itself", "just", "less", "many", "may", "me", "might", "mightn",
    "more", "most", "much", "must", "mustn", "my", "myself", "needn", "neither", "never", "no",
    "nor", "not", "nothing", "now", "of", "off", "often", "on", "once", "only", "or", "other",
    "others", "ought", "our", "ours", "ourselves", "out", "over", "own", "same", "shall", "she",
    "should", "shouldn", "since", "so", "some", "someone", "something", "such", "than", "that",
    "the", "their", "theirs", "them", "themselves", "then", "there", "therefore", "these", "they",
    "this", "those", "though", "through", "thus", "to", "too", "towards", "under", "unless",
    "until", "up", "upon", "us", "very", "was", "wasn", "we", "were", "weren", "what", "whatever",
    "when", "where", "whether", "which", "while", "who", "whom", "whose", "why", "will", "with",
    "within", "without", "would", "wouldn", "yet", "you", "your", "yours", "yourself",
    "yourselves",
];

/// Dutch function words.
#[rustfmt::skip]
pub(super) const NL: &[&str] = &[
    "aan", "al", "alle", "alleen", "alles", "als", "alsof", "altijd", "andere", "anderen",
    "anders", "beide", "ben", "bent", "bij", "binnen", "boven", "buiten", "daar", "daarbij",
    "daardoor", "daarna", "daarom", "dan", "dat", "de", "deed", "deze", "die", "dit", "doch",
    "doen", "doet", "door", "doordat", "dus", "echter", "een", "eens", "eigen", "elk", "elke",
    "en", "enige", "enkele", "er", "ervan", "gaan", "gaat", "geen", "haar", "had", "hadden", "heb",
    "hebben", "heeft", "heel", "hem", "hen", "het", "hier", "hij", "hoe", "hoewel", "hun", "ieder",
    "iedere", "iemand", "iets", "ik", "in", "indien", "is", "ja", "je", "jij", "jou", "jouw",
    "jullie", "kan", "kon", "konden", "kunnen", "kunt", "maar", "me", "meer", "meest", "meeste",
    "men", "met", "mij", "mijn", "minder", "moest", "moet", "moeten", "na", "naar", "naast",
    "niemand", "niet", "niets", "nog", "nooit", "nu", "of", "om", "omdat", "onder", "ons", "onze",
    "ook", "op", "over", "reeds", "sinds", "te", "tegen", "terwijl", "toch", "toen", "tot",
    "tussen", "u", "uit", "uw", "van", "veel", "voor", "vooral", "waar", "waardoor", "waarom",
    "wanneer", "want", "waren", "was", "wat", "we", "weinig", "wel", "welk", "welke", "werd",
    "werden", "wie", "wij", "wil", "willen", "word", "worden", "wordt", "zal", "ze", "zeer",
    "zelf", "zich", "zij", "zijn", "zo", "zoals", "zodat", "zonder", "zou", "zouden", "zowel",
    "zulke", "zullen",
];

/// Whether `lang` names Dutch: `"nl"` in any case, alone or with a region
/// (`"nl-NL"`, `"nl_BE"`). Anything else is treated as English.
pub(crate) fn is_dutch(lang: &str) -> bool {
    let code = lang.split(['-', '_']).next().unwrap_or_default();
    code.eq_ignore_ascii_case("nl")
}

/// The stopword list for `lang`: Dutch when [`is_dutch`], otherwise English.
pub(super) fn list(lang: &str) -> &'static [&'static str] {
    if is_dutch(lang) { NL } else { EN }
}

/// Whether the folded, lowercase `word` is a stopword in `lang`.
pub(super) fn is_stopword(word: &str, lang: &str) -> bool {
    list(lang).binary_search(&word).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_are_sorted_unique_and_folded() {
        for list in [EN, NL] {
            assert!(
                list.windows(2).all(|w| w[0] < w[1]),
                "not sorted or not unique"
            );
            assert!(
                list.iter()
                    .all(|w| w.chars().all(|c| c.is_ascii_lowercase()))
            );
            assert!((140..=200).contains(&list.len()), "{}", list.len());
        }
    }

    #[test]
    fn course_example_words() {
        for w in ["voor", "meer", "de", "een", "zijn"] {
            assert!(is_stopword(w, "nl"), "{w}");
        }
        for w in ["robots", "zorgen", "banen", "verhogen"] {
            assert!(!is_stopword(w, "nl"), "{w}");
        }
        assert!(is_stopword("more", "en"));
        assert!(!is_stopword("meer", "en"));
        assert!(is_stopword("more", "de"));
        assert!(is_stopword("meer", "nl-NL"));
        assert!(is_stopword("meer", "NL"));
    }

    #[test]
    fn dutch_language_codes() {
        for lang in ["nl", "NL", "nl-NL", "nl_be", "Nl-nl"] {
            assert!(is_dutch(lang), "{lang}");
        }
        for lang in ["", "en", "nld", "fr-NL", "n"] {
            assert!(!is_dutch(lang), "{lang}");
        }
    }
}
