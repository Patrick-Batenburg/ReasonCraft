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
//! binary (`tests/ui_language/`) and take turns there. Tests elsewhere
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
    let primary = reasoncraft_core::meta::primary_language(tag);
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
mod tests;
