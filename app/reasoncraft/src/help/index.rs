//! The embedded help pages, indexed by `build.rs`: every
//! `help/<lang>/<topic>.md` of the source tree.

/// Each page as (language, topic id, Markdown), sorted.
pub(super) const FILES: &[(&str, &str, &str)] =
    include!(concat!(env!("OUT_DIR"), "/help_index.rs"));

/// The page of `topic` in `lang`, if there is one.
pub(super) fn page(lang: &str, topic: &str) -> Option<&'static str> {
    FILES
        .iter()
        .find(|(l, t, _)| *l == lang && *t == topic)
        .map(|(.., text)| *text)
}
