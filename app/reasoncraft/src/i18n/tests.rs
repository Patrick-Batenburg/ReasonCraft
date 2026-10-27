use std::collections::{BTreeMap, BTreeSet};

use fluent_syntax::ast::{Entry, Expression, InlineExpression, Pattern, PatternElement};

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
fn unknown_language_reads_english_and_unknown_key_itself() {
    assert_eq!(t_lang("nl", "ribbon.tab.home"), "Start");
    assert_eq!(t_lang("en", "ribbon.tab.home"), "Home");
    assert_eq!(t_lang("fr", "ribbon.tab.home"), "Home");
    assert_eq!(t_lang("nl", "no.such.key"), "no.such.key");
}

#[test]
fn missing_dutch_key_falls_back_to_english() {
    // The English bundle of test builds has one message nl.ftl lacks.
    assert!(!bundles::NL.contains("test-only_english"));
    assert_eq!(t_lang("nl", "test.only_english"), "Only in English");
    assert_eq!(t_lang("en", "test.only_english"), "Only in English");
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

/// Each message of an `.ftl` file: its value pattern.
fn messages(source: &'static str) -> BTreeMap<&'static str, Pattern<&'static str>> {
    let resource = fluent_syntax::parser::parse(source).expect("the .ftl file parses");
    resource
        .body
        .into_iter()
        .filter_map(|entry| match entry {
            Entry::Message(m) => Some((m.id.name, m.value.expect("a message has a value"))),
            Entry::Term(t) => panic!("unexpected term -{}", t.id.name),
            Entry::Junk { content } => panic!("junk in .ftl: {content}"),
            _ => None,
        })
        .collect()
}

/// The variable names (`count` for `{ $count }`) a pattern uses, in its
/// text, selectors and every variant.
fn placeables(pattern: &Pattern<&'static str>) -> BTreeSet<&'static str> {
    fn pattern_vars(p: &Pattern<&'static str>, out: &mut BTreeSet<&'static str>) {
        for element in &p.elements {
            if let PatternElement::Placeable { expression } = element {
                expression_vars(expression, out);
            }
        }
    }
    fn expression_vars(e: &Expression<&'static str>, out: &mut BTreeSet<&'static str>) {
        match e {
            Expression::Select { selector, variants } => {
                inline_vars(selector, out);
                variants.iter().for_each(|v| pattern_vars(&v.value, out));
            }
            Expression::Inline(i) => inline_vars(i, out),
        }
    }
    fn inline_vars(i: &InlineExpression<&'static str>, out: &mut BTreeSet<&'static str>) {
        match i {
            InlineExpression::VariableReference { id } => {
                out.insert(id.name);
            }
            InlineExpression::Placeable { expression } => expression_vars(expression, out),
            InlineExpression::FunctionReference { arguments, .. } => {
                let named = arguments.named.iter().map(|n| &n.value);
                arguments
                    .positional
                    .iter()
                    .chain(named)
                    .for_each(|a| inline_vars(a, out));
            }
            _ => {}
        }
    }
    let mut out = BTreeSet::new();
    pattern_vars(pattern, &mut out);
    out
}

#[test]
fn nl_has_same_keys_and_placeables() {
    let (en, nl) = (messages(bundles::EN), messages(bundles::NL));
    let en_keys: BTreeSet<_> = en.keys().collect();
    let nl_keys: BTreeSet<_> = nl.keys().collect();
    let missing: Vec<_> = en_keys.difference(&nl_keys).collect();
    let extra: Vec<_> = nl_keys.difference(&en_keys).collect();
    assert!(missing.is_empty(), "nl.ftl lacks {missing:?}");
    assert!(extra.is_empty(), "nl.ftl has keys en.ftl lacks: {extra:?}");
    for (id, pattern) in &en {
        assert_eq!(
            placeables(&nl[id]),
            placeables(pattern),
            "placeables of {id}"
        );
    }
}

/// Messages whose Dutch text is rightly the same as the English: names,
/// formats, key caps and texts made only of placeables.
const SAME_IN_DUTCH: &[&str] = &[
    "app-name",
    "window-title",
    "action-zoom_100",
    "action-link",
    "action-spelling",
    "spell-title",
    "find-match_case",
    "find-whole_word",
    "modal-ok",
    "ui_language-en",
    "ui_language-nl",
    "key-ctrl",
    "key-shift",
    "key-alt",
    "key-cmd",
    "key-opt",
    "key-control",
    "key-esc",
    "key-enter",
    "key-tab",
    "key-backspace",
    "key-delete",
    "ribbon-tab-help",
    "text-tab-help",
    "group-help",
    "backstage-document",
    "document-title",
    "group-debug",
    "check-heading",
    "check-card",
    "source-media",
    "source-web",
    "status-zoom",
    "status-zoom_value",
    "link-title",
    "link-hint",
    "export-png",
    "export-svg",
    "export-pdf",
    "export-format-outline_md",
    "export-format-outline_docx",
    "export-format-essay_md",
    "export-format-essay_docx",
    "export-format-argdown",
    "export-argdown",
    "export-dialog-ok",
    "export-png-scale_value",
    "export-page-a4",
    "export-page-letter",
    "settings-ui_scale_unit",
    "a11y-in_group",
    "spell-suggesting",
];

#[test]
fn nl_differs_from_en_for_most_keys() {
    let (en, nl) = (messages(bundles::EN), messages(bundles::NL));
    let same: Vec<_> = en
        .iter()
        .filter(|(id, p)| nl.get(*id) == Some(*p))
        .map(|(id, _)| *id)
        .collect();
    // An untranslated copy is caught however few there are; the allow-list
    // names the texts that are rightly the same.
    let copied: Vec<_> = same
        .iter()
        .filter(|id| !SAME_IN_DUTCH.contains(id))
        .collect();
    assert!(copied.is_empty(), "English text in nl.ftl: {copied:?}");
    let differ = en.len() - same.len();
    assert!(
        differ * 10 >= en.len() * 9,
        "only {differ} of {} messages differ",
        en.len()
    );
}

#[test]
fn dutch_plural_forms() {
    let key = "check.explain.incomplete_evaluation";
    let text = |count: u32| {
        let mut args = FluentArgs::new();
        args.set("count", count);
        t_lang_args("nl", key, &args)
    };
    assert!(text(1).starts_with("1 onderdeel is "), "{}", text(1));
    assert!(text(2).starts_with("2 onderdelen zijn "), "{}", text(2));
    let mut args = FluentArgs::new();
    args.set("count", 1);
    assert_eq!(t_lang_args("nl", "status.boxes", &args), "1 vak");
    args.set("count", 2);
    assert_eq!(t_lang_args("nl", "status.boxes", &args), "2 vakken");
}
