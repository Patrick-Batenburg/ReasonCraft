//! User-visible strings, looked up in the embedded `i18n/en.ftl`.
//!
//! Keys are dotted (`ribbon.tab.home`). Fluent identifiers cannot contain
//! dots, so each dot maps to a dash in the `.ftl` file (`ribbon-tab-home`).

use std::sync::OnceLock;

use fluent_bundle::FluentResource;
use fluent_bundle::concurrent::FluentBundle;
use unic_langid::LanguageIdentifier;

pub use fluent_bundle::FluentArgs;

const EN: &str = include_str!("../i18n/en.ftl");

fn bundle() -> &'static FluentBundle<FluentResource> {
    static BUNDLE: OnceLock<FluentBundle<FluentResource>> = OnceLock::new();
    BUNDLE.get_or_init(|| {
        let resource = FluentResource::try_new(EN.to_owned())
            .unwrap_or_else(|(_, errors)| panic!("en.ftl does not parse: {errors:?}"));
        let lang: LanguageIdentifier = "en-US".parse().expect("valid language id");
        let mut bundle = FluentBundle::new_concurrent(vec![lang]);
        // No Unicode isolation marks around arguments: they show as boxes in egui.
        bundle.set_use_isolating(false);
        bundle
            .add_resource(resource)
            .expect("en.ftl has no duplicate keys");
        bundle
    })
}

/// The English text for `key`. A missing key returns the key itself.
pub fn t(key: &str) -> String {
    lookup(key, None)
}

/// Like [`t`], with arguments for placeables such as `{ $count }`.
pub fn t_args(key: &str, args: &FluentArgs) -> String {
    lookup(key, Some(args))
}

fn lookup(key: &str, args: Option<&FluentArgs>) -> String {
    let bundle = bundle();
    let id = key.replace('.', "-");
    let Some(pattern) = bundle.get_message(&id).and_then(|m| m.value()) else {
        #[cfg(debug_assertions)]
        eprintln!("warning: missing i18n key `{key}`");
        return key.to_owned();
    };
    let mut errors = Vec::new();
    let text = bundle.format_pattern(pattern, args, &mut errors);
    #[cfg(debug_assertions)]
    if !errors.is_empty() {
        eprintln!("warning: i18n key `{key}` formatted with errors: {errors:?}");
    }
    text.into_owned()
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
    fn every_source_type_has_a_name_and_a_prompt() {
        use reasoncraft_core::SourceType;
        for st in SourceType::ALL {
            let key = st.key();
            for id in [format!("source-{key}"), format!("source-prompt-{key}")] {
                let line = format!(
                    "
{id} = "
                );
                assert!(EN.contains(&line), "en.ftl lacks `{id}`");
            }
            assert_ne!(t(&format!("source.{key}")), format!("source.{key}"));
            assert_ne!(
                t(&format!("source.prompt.{key}")),
                format!("source.prompt.{key}")
            );
        }
    }
}
