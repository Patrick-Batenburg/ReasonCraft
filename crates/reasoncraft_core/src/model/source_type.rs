//! The kinds of source boxes.

use serde::{Deserialize, Serialize};

/// The kind of a source box. A type name this build does not know (from a
/// newer file) is kept as [`SourceType::Other`], written back unchanged and
/// shown as the generic source.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SourceType {
    Source,
    MyBasis,
    Assertion,
    ByDefinition,
    CaseStudy,
    CommonBelief,
    Data,
    Event,
    Example,
    ExpertOpinion,
    Law,
    Media,
    PersonalExperience,
    Publication,
    Quote,
    Statistic,
    Web,
    /// An unknown type name, kept verbatim.
    Other(String),
}

impl SourceType {
    /// Every selectable type, in menu order (without `Other`).
    pub const ALL: [SourceType; 17] = [
        SourceType::Source,
        SourceType::MyBasis,
        SourceType::Assertion,
        SourceType::ByDefinition,
        SourceType::CaseStudy,
        SourceType::CommonBelief,
        SourceType::Data,
        SourceType::Event,
        SourceType::Example,
        SourceType::ExpertOpinion,
        SourceType::Law,
        SourceType::Media,
        SourceType::PersonalExperience,
        SourceType::Publication,
        SourceType::Quote,
        SourceType::Statistic,
        SourceType::Web,
    ];

    /// The stable snake_case name; `Other` shows as the generic `"source"`.
    pub fn key(&self) -> &'static str {
        match self {
            SourceType::Source | SourceType::Other(_) => "source",
            SourceType::MyBasis => "my_basis",
            SourceType::Assertion => "assertion",
            SourceType::ByDefinition => "by_definition",
            SourceType::CaseStudy => "case_study",
            SourceType::CommonBelief => "common_belief",
            SourceType::Data => "data",
            SourceType::Event => "event",
            SourceType::Example => "example",
            SourceType::ExpertOpinion => "expert_opinion",
            SourceType::Law => "law",
            SourceType::Media => "media",
            SourceType::PersonalExperience => "personal_experience",
            SourceType::Publication => "publication",
            SourceType::Quote => "quote",
            SourceType::Statistic => "statistic",
            SourceType::Web => "web",
        }
    }

    /// The type named `name` in a file; unknown names become `Other`.
    pub fn from_name(name: &str) -> SourceType {
        SourceType::ALL
            .into_iter()
            .find(|t| t.key() == name)
            .unwrap_or_else(|| SourceType::Other(name.to_owned()))
    }

    /// The name written to a file: the key, or an unknown name as it was read.
    pub fn name(&self) -> &str {
        match self {
            SourceType::Other(name) => name,
            known => known.key(),
        }
    }
}

impl Serialize for SourceType {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.name())
    }
}

impl<'de> Deserialize<'de> for SourceType {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let name = String::deserialize(d)?;
        Ok(SourceType::from_name(&name))
    }
}
