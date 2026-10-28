//! The words the core itself writes into a document's text, in the
//! document language (en/nl): linking words ("because"/"omdat"), essay
//! headings and the Argdown heading of the parking area, which Argdown
//! import also reads back. They are core logic, used and tested without
//! the app.
//!
//! Every other word drawn on the map or written into exports, source-type
//! names included, lives in the app's `i18n/*.ftl` files only; core text
//! exports take those words from the caller (see
//! [`crate::outline_md::outline_markdown`]).
//!
//! The words themselves are in the language table, [`crate::lang`].

use crate::lang::pack;
use crate::outline::LinkLabel;

/// The words that link a premise to what it supports or opposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkingWords {
    pub because: &'static str,
    pub but: &'static str,
    pub however: &'static str,
}

/// The linking words for `lang`: Dutch for `nl` (any case or region),
/// English otherwise.
pub fn linking_words(lang: &str) -> LinkingWords {
    pack(lang).linking
}

impl LinkingWords {
    /// The word for a label; empty for [`LinkLabel::None`].
    pub fn word(&self, label: LinkLabel) -> &'static str {
        match label {
            LinkLabel::Because => self.because,
            LinkLabel::But => self.but,
            LinkLabel::However => self.however,
            LinkLabel::None => "",
        }
    }
}

/// The heading written above parked fragments (see
/// [`crate::lang::LangPack::not_connected`]).
pub fn not_connected_heading(lang: &str) -> &'static str {
    pack(lang).not_connected[0]
}

/// The headings of the essay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EssayHeading {
    Introduction,
    /// Followed by the section number.
    Section,
    Conclusion,
}

/// An essay heading word in the document language.
pub fn essay_heading(lang: &str, which: EssayHeading) -> &'static str {
    let p = pack(lang);
    match which {
        EssayHeading::Introduction => p.introduction,
        EssayHeading::Section => p.section,
        EssayHeading::Conclusion => p.conclusion,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_nl_and_fallback() {
        let nl = linking_words("nl-NL");
        assert_eq!(
            (nl.because, nl.but, nl.however),
            ("omdat", "maar", "echter")
        );
        assert_eq!(linking_words("NL"), nl);
        let en = linking_words("en");
        assert_eq!(
            (en.because, en.but, en.however),
            ("because", "but", "however")
        );
        assert_eq!(linking_words("de"), en);
        assert_eq!(linking_words(""), en);
        assert_eq!(en.word(LinkLabel::None), "");
    }

    #[test]
    fn essay_headings_nl_and_fallback() {
        let all = [
            EssayHeading::Introduction,
            EssayHeading::Section,
            EssayHeading::Conclusion,
        ];
        let nl = all.map(|h| essay_heading("nl-BE", h));
        assert_eq!(nl, ["Inleiding", "Deel", "Conclusie"]);
        let en = all.map(|h| essay_heading("de", h));
        assert_eq!(en, ["Introduction", "Section", "Conclusion"]);
    }
}
