//! The embedded translations: one Fluent bundle per UI language, built on
//! first use.

use std::sync::OnceLock;

use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource};
use unic_langid::LanguageIdentifier;

use super::UI_LANGUAGES;

pub(super) const EN: &str = include_str!("../../i18n/en.ftl");
pub(super) const NL: &str = include_str!("../../i18n/nl.ftl");

/// Each UI language's Fluent locale (for plural rules) and messages, in
/// [`UI_LANGUAGES`] order.
const SOURCES: [(&str, &str); UI_LANGUAGES.len()] = [("en-US", EN), ("nl-NL", NL)];

type Bundle = FluentBundle<FluentResource>;

/// The bundle of `lang`, which must be one of [`UI_LANGUAGES`].
fn bundle(lang: &str) -> &'static Bundle {
    static BUNDLES: [OnceLock<Bundle>; UI_LANGUAGES.len()] =
        [const { OnceLock::new() }; UI_LANGUAGES.len()];
    let i = UI_LANGUAGES
        .iter()
        .position(|l| *l == lang)
        .expect("a UI language");
    BUNDLES[i].get_or_init(|| {
        let (locale, source) = SOURCES[i];
        let resource = FluentResource::try_new(source.to_owned())
            .unwrap_or_else(|(_, errors)| panic!("{lang}.ftl does not parse: {errors:?}"));
        let locale: LanguageIdentifier = locale.parse().expect("valid language id");
        let mut bundle = FluentBundle::new_concurrent(vec![locale]);
        // No Unicode isolation marks around arguments: they show as boxes in egui.
        bundle.set_use_isolating(false);
        bundle
            .add_resource(resource)
            .unwrap_or_else(|errors| panic!("{lang}.ftl has duplicate keys: {errors:?}"));
        bundle
    })
}

/// Message `id` of UI language `lang`, formatted with `args`; `None` when
/// `lang` lacks it.
pub(super) fn format(lang: &str, id: &str, args: Option<&FluentArgs>) -> Option<String> {
    let bundle = bundle(lang);
    let pattern = bundle.get_message(id)?.value()?;
    let mut errors = Vec::new();
    let text = bundle.format_pattern(pattern, args, &mut errors);
    #[cfg(debug_assertions)]
    if !errors.is_empty() {
        eprintln!("warning: i18n message `{id}` ({lang}) formatted with errors: {errors:?}");
    }
    Some(text.into_owned())
}
