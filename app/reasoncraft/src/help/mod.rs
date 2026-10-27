//! Offline help: one page per topic in every UI language, written in a
//! small Markdown subset (see [`render`]) and embedded in the binary. The
//! Text panel's Help tab shows it, in the UI language.

mod index;
pub mod keyboard;
pub mod render;

#[cfg(test)]
mod tests;

/// Every topic, by id (the page's file stem), in the order the contents
/// list them.
pub const TOPICS: [&str; 17] = [
    "getting-started",
    "claims-and-premises",
    "reasons-and-objections",
    "co-premises",
    "rebuttals",
    "whole-card-vs-premise",
    "sources",
    "evaluation",
    "checks",
    "grouping-maps",
    "outline-and-essay",
    "snippets",
    "exports-and-print",
    "argdown",
    "keyboard",
    "files-and-recovery",
    "languages",
];

/// The topic F1 opens when nothing more specific applies.
pub const START: &str = "getting-started";

/// What the Help tab shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HelpPage {
    /// The list of all topics.
    Contents,
    /// One topic, by its id (one of [`TOPICS`]).
    Topic(&'static str),
}

/// The page the Help tab shows, and the pages shown before it (for Back).
/// View state: it is not saved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HelpState {
    pub page: HelpPage,
    back: Vec<HelpPage>,
}

impl Default for HelpState {
    fn default() -> Self {
        Self {
            page: HelpPage::Topic(START),
            back: Vec::new(),
        }
    }
}

/// How many pages Back remembers.
const BACK_MAX: usize = 50;

impl HelpState {
    /// Show `page`; the page shown now is remembered for Back.
    pub fn open(&mut self, page: HelpPage) {
        if page != self.page {
            if self.back.len() == BACK_MAX {
                self.back.remove(0);
            }
            self.back.push(self.page);
            self.page = page;
        }
    }

    /// Show the page shown before this one, if any.
    pub fn back(&mut self) {
        if let Some(page) = self.back.pop() {
            self.page = page;
        }
    }

    pub fn can_go_back(&self) -> bool {
        !self.back.is_empty()
    }
}

/// The topic with this id, as the `'static` id of [`TOPICS`].
pub fn topic(id: &str) -> Option<&'static str> {
    TOPICS.into_iter().find(|t| *t == id)
}

/// The page of `topic` in `lang` (a UI language), or in English if that
/// language lacks it; empty for an unknown topic.
pub fn source(lang: &str, topic: &str) -> &'static str {
    index::page(lang, topic)
        .or_else(|| index::page(crate::i18n::UI_LANGUAGES[0], topic))
        .unwrap_or_default()
}

/// The title of `topic` in `lang`: its page's first heading.
pub fn title(lang: &str, topic: &str) -> String {
    source(lang, topic)
        .lines()
        .find_map(|l| l.strip_prefix("# "))
        .unwrap_or(topic)
        .trim()
        .to_owned()
}
