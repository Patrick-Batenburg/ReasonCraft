//! User-visible strings, looked up in the embedded `i18n/*.ftl` files.
//!
//! Keys are dotted (`ribbon.tab.home`); see [`keys`] for how they map to
//! Fluent message ids. The interface reads in the UI language
//! ([`set_ui_language`], [`t`]); text drawn on the map or written into
//! exports names its language ([`t_lang`]). A message a language lacks is
//! taken from English.
//!
//! The UI language is one setting for the whole process. Tests run in
//! parallel threads of one process, so a test that switches it would
//! change the text other tests see: such tests live in their own test
//! binary (`tests/ui_language.rs`) and take turns there. Tests elsewhere
//! leave it English and use [`t_lang`] for other languages.

use std::sync::{PoisonError, RwLock};

pub use fluent_bundle::FluentArgs;

mod bundles;
mod keys;

/// The languages the interface can be shown in; the first is the default
/// and the fallback for messages another language lacks.
pub const UI_LANGUAGES: [&str; 2] = ["en", "nl"];

static ACTIVE: RwLock<&str> = RwLock::new(UI_LANGUAGES[0]);

/// The UI language a language tag such as `nl`, `nl-NL` or `nl_BE.UTF-8`
/// names (its language part, in any case), else English. A document's
/// text is drawn and exported in `language_of(&doc.meta.language)`.
pub fn language_of(tag: &str) -> &'static str {
    let primary = tag.split(['-', '_', '.', '@']).next().unwrap_or_default();
    UI_LANGUAGES
        .into_iter()
        .find(|l| primary.eq_ignore_ascii_case(l))
        .unwrap_or(UI_LANGUAGES[0])
}

/// Show the interface in `lang` from the next lookup on; a language that
/// is not one of [`UI_LANGUAGES`] means English.
pub fn set_ui_language(lang: &str) {
    *ACTIVE.write().unwrap_or_else(PoisonError::into_inner) = language_of(lang);
}

/// The language the interface is shown in.
pub fn ui_language() -> &'static str {
    *ACTIVE.read().unwrap_or_else(PoisonError::into_inner)
}

/// The UI language for an OS `locale` such as `nl-NL` or `nl_BE.UTF-8`:
/// the UI language its language part names, else English.
pub fn os_ui_language(locale: Option<&str>) -> &'static str {
    language_of(locale.unwrap_or_default())
}

/// The text for `key` in the UI language. A missing key returns the key itself.
pub fn t(key: &str) -> String {
    lookup(ui_language(), key, None)
}

/// Like [`t`], with arguments for placeables such as `{ $count }`.
pub fn t_args(key: &str, args: &FluentArgs) -> String {
    lookup(ui_language(), key, Some(args))
}

/// The text for `key` in `lang`, a language tag such as `nl` or `nl-NL`
/// (English when it names no UI language).
pub fn t_lang(lang: &str, key: &str) -> String {
    lookup(lang, key, None)
}

/// Like [`t_lang`], with arguments for placeables.
pub fn t_lang_args(lang: &str, key: &str, args: &FluentArgs) -> String {
    lookup(lang, key, Some(args))
}

fn lookup(lang: &str, key: &str, args: Option<&FluentArgs>) -> String {
    let id = keys::message_id(key);
    let lang = language_of(lang);
    bundles::format(lang, &id, args)
        .or_else(|| bundles::format(UI_LANGUAGES[0], &id, args))
        .unwrap_or_else(|| {
            #[cfg(debug_assertions)]
            eprintln!("warning: missing i18n key `{key}`");
            key.to_owned()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args_are_substituted_without_isolation_marks() {
        let mut args = FluentArgs::new();
        args.set("count", 3);
        assert_eq!(t_args("status.boxes", &args), "3 boxes");
        args.set("count", 1);
        assert_eq!(t_args("status.boxes", &args), "1 box");
    }

    #[test]
    fn os_language_maps_dutch_variants() {
        for locale in ["nl", "nl-NL", "nl_BE.UTF-8", "NL-nl"] {
            assert_eq!(os_ui_language(Some(locale)), "nl", "{locale}");
        }
        for locale in [Some("de-DE"), Some("en-US"), Some("nld"), Some(""), None] {
            assert_eq!(os_ui_language(locale), "en", "{locale:?}");
        }
    }

    #[test]
    fn missing_dutch_key_falls_back_to_english() {
        // nl.ftl may lack a message: it then comes from en.ftl.
        assert_eq!(t_lang("nl", "ribbon.tab.file"), "File");
        assert_eq!(t_lang("nl", "ribbon.tab.home"), "Start");
        assert_eq!(t_lang("en", "ribbon.tab.home"), "Home");
        let mut args = FluentArgs::new();
        args.set("count", 2);
        assert_eq!(t_lang_args("nl", "status.boxes", &args), "2 boxes");
        // An unknown language reads as English; an unknown key as itself.
        assert_eq!(t_lang("fr", "ribbon.tab.home"), "Home");
        assert_eq!(t_lang("nl", "no.such.key"), "no.such.key");
    }

    #[test]
    fn source_names_single_source() {
        // Source-type names live in the ftl files only: the core's words
        // file names none of them, and Dutch has its own for each type.
        let words = include_str!("../../../../crates/reasoncraft_core/src/words.rs");
        for st in reasoncraft_core::SourceType::ALL {
            let key = format!("source.{}", st.key());
            for lang in UI_LANGUAGES {
                let name = t_lang(lang, &key);
                assert!(!words.contains(&format!("\"{name}\"")), "{lang} {name}");
            }
            let id = keys::message_id(&key);
            assert!(bundles::NL.contains(&format!("\n{id} = ")), "nl lacks {id}");
        }
    }

    #[test]
    fn region_tags_read_as_their_language() {
        assert_eq!(t_lang("nl-BE", "ribbon.tab.home"), "Start");
        assert_eq!(language_of("nl_NL"), "nl");
        assert_eq!(language_of("en-GB"), "en");
        assert_eq!(language_of("de"), "en");
    }

    #[test]
    fn ui_languages_parse_and_unknown_is_english() {
        for lang in UI_LANGUAGES {
            assert_eq!(t_lang(lang, "app.name"), "ReasonCraft");
        }
        // Only ever set to English here: other tests in this binary read
        // the UI language (see the module docs).
        set_ui_language("xx");
        assert_eq!(ui_language(), "en");
    }
}
