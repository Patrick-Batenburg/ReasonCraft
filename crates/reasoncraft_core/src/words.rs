//! Linking words and source-type names in the document language (en/nl).
//! These are document content, not UI text.

use crate::checks::stopwords::is_dutch;
use crate::model::SourceType;
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
    if is_dutch(lang) {
        LinkingWords {
            because: "omdat",
            but: "maar",
            however: "echter",
        }
    } else {
        LinkingWords {
            because: "because",
            but: "but",
            however: "however",
        }
    }
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

/// The name of a source type in the document language. `Other` gives the
/// generic name.
pub fn source_word(t: &SourceType, lang: &str) -> &'static str {
    let nl = is_dutch(lang);
    let pick = |en: &'static str, nl_word: &'static str| if nl { nl_word } else { en };
    match t {
        SourceType::Source | SourceType::Other(_) => pick("Source", "Bron"),
        SourceType::MyBasis => pick("My own basis", "Eigen basis"),
        SourceType::Assertion => pick("Assertion", "Bewering"),
        SourceType::ByDefinition => pick("By definition", "Per definitie"),
        SourceType::CaseStudy => pick("Case study", "Casus"),
        SourceType::CommonBelief => pick("Common belief", "Algemene opvatting"),
        SourceType::Data => pick("Data", "Gegevens"),
        SourceType::Event => pick("Event", "Gebeurtenis"),
        SourceType::Example => pick("Example", "Voorbeeld"),
        SourceType::ExpertOpinion => pick("Expert opinion", "Deskundigenoordeel"),
        SourceType::Law => pick("Law", "Wet"),
        SourceType::Media => "Media",
        SourceType::PersonalExperience => pick("Personal experience", "Persoonlijke ervaring"),
        SourceType::Publication => pick("Publication", "Publicatie"),
        SourceType::Quote => pick("Quote", "Citaat"),
        SourceType::Statistic => pick("Statistic", "Statistiek"),
        SourceType::Web => "Web",
    }
}

/// The heading above parked fragments.
pub fn not_connected_heading(lang: &str) -> &'static str {
    if is_dutch(lang) {
        "Niet verbonden"
    } else {
        "Not connected"
    }
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
    let nl = is_dutch(lang);
    match which {
        EssayHeading::Introduction if nl => "Inleiding",
        EssayHeading::Introduction => "Introduction",
        EssayHeading::Section if nl => "Deel",
        EssayHeading::Section => "Section",
        EssayHeading::Conclusion if nl => "Conclusie",
        EssayHeading::Conclusion => "Conclusion",
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
    fn source_words_cover_every_type() {
        for t in SourceType::ALL {
            assert!(!source_word(&t, "en").is_empty());
            assert!(!source_word(&t, "nl").is_empty());
        }
        assert_eq!(source_word(&SourceType::Law, "nl"), "Wet");
        assert_eq!(source_word(&SourceType::Other("x".into()), "nl"), "Bron");
        assert_eq!(source_word(&SourceType::MyBasis, "fr"), "My own basis");
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
