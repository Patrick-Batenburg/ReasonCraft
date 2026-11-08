use std::collections::BTreeSet;

use super::render::{Block, Span, parse};
use super::{TOPICS, index, source, title};
use crate::i18n::UI_LANGUAGES;

#[test]
fn every_topic_exists_in_both_languages() {
    let wanted: BTreeSet<&str> = TOPICS.into_iter().collect();
    assert_eq!(wanted.len(), 17, "17 distinct topics");
    for lang in UI_LANGUAGES {
        let found: BTreeSet<&str> = index::FILES
            .iter()
            .filter(|(l, ..)| *l == lang)
            .map(|(_, topic, _)| *topic)
            .collect();
        assert_eq!(found, wanted, "the {lang} help pages");
        for topic in TOPICS {
            let text = source(lang, topic);
            assert!(
                text.starts_with("# "),
                "{lang}/{topic} starts with its title"
            );
            assert!(text.len() > 300, "{lang}/{topic} has content");
        }
    }
    // No pages in other languages, and each language's pages are its own.
    assert!(index::FILES.iter().all(|(l, ..)| UI_LANGUAGES.contains(l)));
    // Titles that are a proper name may read the same in both languages.
    const PROPER_NAMES: [&str; 1] = ["argdown"];
    for topic in TOPICS {
        assert_ne!(source("en", topic), source("nl", topic), "{topic}");
        if !PROPER_NAMES.contains(&topic) {
            assert_ne!(title("en", topic), title("nl", topic), "{topic}'s title");
        }
    }
}

/// Every `[text](target)` in a page names a topic, and the renderer turns
/// each of them into a link.
#[test]
fn topic_links_resolve() {
    for &(lang, topic, text) in index::FILES {
        let mut targets = Vec::new();
        let mut rest = text;
        while let Some(at) = rest.find("](") {
            let after = &rest[at + 2..];
            let end = after.find(')').expect("a link ends with `)`");
            targets.push(&after[..end]);
            rest = &after[end..];
        }
        for target in &targets {
            assert!(
                TOPICS.contains(target),
                "{lang}/{topic}: a link to `{target}`, which is no topic"
            );
            assert_ne!(target, &topic, "{lang}/{topic} links to itself");
        }
        let blocks = parse(text);
        let links: Vec<&str> = blocks
            .iter()
            .flat_map(Block::spans)
            .filter_map(|s| match s {
                Span::Link { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(
            links.len(),
            targets.len(),
            "{lang}/{topic}: every link is read"
        );
        // Screen readers tell links apart by their text.
        let distinct: BTreeSet<&str> = links.iter().copied().collect();
        assert_eq!(
            distinct.len(),
            links.len(),
            "{lang}/{topic}: a link text twice"
        );
    }
}

#[test]
fn renderer_reads_the_subset() {
    let src = "# Title\n\nSome **bold** and `code` and [Sources](sources)\nnext line.\n\n## Part\n- one\n  more\n- two\n";
    let blocks = parse(src);
    let text = |t: &str| Span::Text(t.to_owned());
    assert_eq!(
        blocks,
        vec![
            Block::Heading(1, vec![text("Title")]),
            Block::Paragraph(vec![
                text("Some "),
                Span::Bold("bold".to_owned()),
                text(" and "),
                Span::Code("code".to_owned()),
                text(" and "),
                Span::Link {
                    text: "Sources".to_owned(),
                    topic: "sources",
                },
                text(" next line."),
            ]),
            Block::Heading(2, vec![text("Part")]),
            Block::Item(vec![text("one more")]),
            Block::Item(vec![text("two")]),
        ]
    );
}

#[test]
fn renderer_ignores_unknown_markup() {
    let lines = [
        "### Small heading",
        "* star item",
        "1. numbered",
        "> quote",
        "<b>html</b> and _em_ and *em*",
        "[web](https://example.org) and [nowhere](no-such-topic)",
        "**open and `open and [open",
        "| a | b |",
    ];
    let blocks = parse(&lines.join("\n"));
    assert_eq!(
        blocks,
        vec![Block::Paragraph(vec![Span::Text(lines.join(" "))])],
        "all of it is one paragraph of plain text"
    );
    // A heading needs its space; HTML stays text in a list item too.
    assert_eq!(
        parse("#NoSpace\n\n- <i>x</i>"),
        vec![
            Block::Paragraph(vec![Span::Text("#NoSpace".to_owned())]),
            Block::Item(vec![Span::Text("<i>x</i>".to_owned())]),
        ]
    );
}

/// Every character of the help (and the bullet the lists are drawn with)
/// is in the UI font: egui has no fallback for a missing glyph.
#[test]
fn help_text_is_in_the_ui_font() {
    for &(lang, topic, text) in index::FILES {
        for c in text.chars().filter(|c| !c.is_ascii()) {
            let font_has = crate::canvas::input::ui_font_has(c);
            assert!(font_has, "{lang}/{topic}: no glyph for {c:?}");
        }
    }
    assert!(crate::canvas::input::ui_font_has('•'));
}

#[test]
fn parsed_pages_are_kept_per_language_and_topic() {
    use std::sync::Arc;
    let en = super::blocks("en", "sources");
    assert_eq!(&*en, &parse(source("en", "sources"))[..]);
    // Read once: the same blocks again, for any tag of the language.
    assert!(Arc::ptr_eq(&en, &super::blocks("en-GB", "sources")));
    let nl = super::blocks("nl", "sources");
    assert_eq!(&*nl, &parse(source("nl", "sources"))[..]);
    assert_ne!(en, nl);
}
