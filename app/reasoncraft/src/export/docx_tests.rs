use std::io::{Cursor, Read};

use reasoncraft_core::outline::{OutlineOptions, build_outline};
use reasoncraft_core::{CardKind, CardStrength, Document, EssaySlot, Item, NewLoose, Parent};

use super::docx::{INDENT_TWIPS, essay_docx, language_tag, outline_docx};

const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

fn part(bytes: &[u8], name: &str) -> String {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).expect("a zip");
    let mut out = String::new();
    zip.by_name(name)
        .unwrap_or_else(|_| panic!("{name} is missing"))
        .read_to_string(&mut out)
        .unwrap();
    out
}

/// A claim with a strong reason (and a source under it) and an objection.
fn sample() -> (Document, Vec<reasoncraft_core::CardId>) {
    let mut d = Document::new("Cities should ban cars");
    let root = Parent::Box(d.root);
    let mut cards = Vec::new();
    for (kind, text, strength) in [
        (CardKind::Objection, "Shops lose customers", None),
        (
            CardKind::Reason,
            "Air gets cleaner & safer",
            Some(CardStrength::Strong),
        ),
    ] {
        let (c, _) = d.add_card(root, kind, text).unwrap();
        d.set_card_strength(c, strength).unwrap();
        cards.push(c);
    }
    (d, cards)
}

/// The text of each paragraph with its style id.
fn paragraphs(xml: &str) -> Vec<(Option<String>, String)> {
    let doc = roxmltree::Document::parse(xml).expect("well-formed document.xml");
    doc.descendants()
        .filter(|n| n.has_tag_name((W, "p")))
        .map(|p| {
            let style = p
                .descendants()
                .find(|n| n.has_tag_name((W, "pStyle")))
                .and_then(|n| n.attribute((W, "val")))
                .map(str::to_owned);
            let text = p
                .descendants()
                .filter(|n| n.has_tag_name((W, "t")))
                .filter_map(|n| n.text())
                .collect::<String>();
            (style, text)
        })
        .collect()
}

#[test]
fn docx_is_valid_ooxml() {
    let (mut doc, _) = sample();
    doc.set_essay_text(EssaySlot::Intro, "Intro text").unwrap();
    for bytes in [
        essay_docx(&doc, "en").unwrap(),
        outline_docx(
            &build_outline(&doc, &OutlineOptions::default()),
            &OutlineOptions::default(),
            "en",
            "",
        )
        .unwrap(),
    ] {
        let types = part(&bytes, "[Content_Types].xml");
        roxmltree::Document::parse(&types).expect("well-formed content types");
        assert!(types.contains("wordprocessingml.document.main+xml"));
        let styles = part(&bytes, "word/styles.xml");
        let styles = roxmltree::Document::parse(&styles).expect("well-formed styles.xml");
        let ids: Vec<&str> = styles
            .descendants()
            .filter(|n| n.has_tag_name((W, "style")))
            .filter_map(|n| n.attribute((W, "styleId")))
            .collect();
        let mut unique = ids.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), ids.len(), "style ids are unique: {ids:?}");
        for id in ["Normal", "Title", "Heading1"] {
            assert!(ids.contains(&id), "style {id} is defined: {ids:?}");
        }
        let paras = paragraphs(&part(&bytes, "word/document.xml"));
        assert!(paras.iter().any(|(s, _)| s.as_deref() == Some("Title")));
    }
    let paras = paragraphs(&part(&essay_docx(&doc, "en").unwrap(), "word/document.xml"));
    let headings: Vec<&str> = paras
        .iter()
        .filter(|(s, _)| s.as_deref() == Some("Heading1"))
        .map(|(_, t)| t.as_str())
        .collect();
    assert_eq!(
        headings,
        [
            "Introduction",
            "Section 1: Air gets cleaner & safer",
            "Section 2: Shops lose customers",
            "Conclusion"
        ]
    );
}

#[test]
fn essay_docx_contains_paragraph_text() {
    let (mut doc, cards) = sample();
    doc.meta.author = "Ada <Lovelace>".into();
    let reason = doc.cards[&cards[1]].premises[0];
    doc.set_essay_text(
        EssaySlot::Intro,
        "Cars are \u{1}the problem.\n\nSecond line",
    )
    .unwrap();
    doc.set_essay_text(EssaySlot::Item(Item::Box(reason)), "Clean air: ünïcode ✓")
        .unwrap();
    let bytes = essay_docx(&doc, "en").unwrap();
    let xml = part(&bytes, "word/document.xml");
    let paras = paragraphs(&xml);
    let has = |style: &str, text: &str| {
        paras
            .iter()
            .any(|(s, t)| s.as_deref() == Some(style) && t == text)
    };
    // The title falls back to the claim; the author follows it.
    assert_eq!(
        paras[0],
        (Some("Title".into()), "Cities should ban cars".into())
    );
    assert_eq!(paras[1], (Some("Normal".into()), "Ada <Lovelace>".into()));
    assert!(has("Normal", "Cars are the problem."));
    assert!(has("Normal", "Second line"));
    assert!(has("Normal", "Clean air: ünïcode ✓"));
    // The slot without text is an italic grey hint with its statement.
    let hint = "Shops lose customers";
    assert!(has("Normal", hint));
    let tree = roxmltree::Document::parse(&xml).unwrap();
    let run = tree
        .descendants()
        .filter(|n| n.has_tag_name((W, "r")))
        .find(|r| r.descendants().any(|t| t.text() == Some(hint)))
        .unwrap();
    let prop = |tag: &str| run.descendants().find(|n| n.has_tag_name((W, tag)));
    assert!(prop("i").is_some(), "hint is italic");
    assert_eq!(prop("color").unwrap().attribute((W, "val")), Some("56565F"));
    // A written paragraph is neither italic nor grey.
    let written = tree
        .descendants()
        .filter(|n| n.has_tag_name((W, "r")))
        .find(|r| r.descendants().any(|t| t.text() == Some("Second line")))
        .unwrap();
    assert!(!written.descendants().any(|n| n.has_tag_name((W, "i"))));

    // A document title wins over the claim.
    doc.meta.title = "My essay".into();
    let paras = paragraphs(&part(&essay_docx(&doc, "en").unwrap(), "word/document.xml"));
    assert_eq!(paras[0], (Some("Title".into()), "My essay".into()));
}

#[test]
fn outline_docx_dutch_words() {
    let (doc, _) = sample();
    let opts = OutlineOptions::default();
    let lines = build_outline(&doc, &opts);
    let bytes = outline_docx(&lines, &opts, "nl", "").unwrap();
    let xml = part(&bytes, "word/document.xml");
    let paras = paragraphs(&xml);
    assert_eq!(
        paras[0],
        (Some("Title".into()), "Cities should ban cars".into())
    );
    let texts: Vec<&str> = paras.iter().map(|(_, t)| t.as_str()).collect();
    assert!(
        texts
            .iter()
            .any(|t| t.contains("omdat Air gets cleaner & safer")),
        "{texts:?}"
    );
    assert!(
        texts
            .iter()
            .any(|t| t.contains("maar Shops lose customers"))
    );
    assert!(!xml.contains("because"));

    // Number bold, linking word italic, indent 0.6 cm per depth.
    let tree = roxmltree::Document::parse(&xml).unwrap();
    let para = tree
        .descendants()
        .filter(|n| n.has_tag_name((W, "p")))
        .find(|p| p.descendants().any(|t| t.text() == Some("omdat ")))
        .unwrap();
    let runs: Vec<_> = para
        .children()
        .filter(|n| n.has_tag_name((W, "r")))
        .collect();
    let has = |r: &roxmltree::Node, tag: &str| r.descendants().any(|n| n.has_tag_name((W, tag)));
    assert!(has(&runs[0], "b") && !has(&runs[0], "i"), "number is bold");
    assert!(has(&runs[1], "i") && !has(&runs[1], "b"), "word is italic");
    let left = para
        .descendants()
        .find(|n| n.has_tag_name((W, "ind")))
        .and_then(|n| n.attribute((W, "left")))
        .unwrap();
    let depth = lines
        .iter()
        .find(|l| l.text.starts_with("Air"))
        .unwrap()
        .depth;
    assert_eq!(left, (INDENT_TWIPS * depth as i32).to_string());

    // The same document in English uses English words.
    let en = outline_docx(&lines, &opts, "en", "").unwrap();
    assert!(part(&en, "word/document.xml").contains("because "));
}

#[test]
fn outline_docx_options_and_parked_heading() {
    let (mut doc, _) = sample();
    doc.add_loose(NewLoose::Category, "Loose thought");
    let opts = OutlineOptions {
        numbering: false,
        labels: false,
        notes: false,
    };
    let lines = build_outline(&doc, &opts);
    let bytes = outline_docx(&lines, &opts, "nl", "").unwrap();
    let paras = paragraphs(&part(&bytes, "word/document.xml"));
    let texts: Vec<&str> = paras.iter().map(|(_, t)| t.as_str()).collect();
    assert!(texts.contains(&"Air gets cleaner & safer"), "{texts:?}");
    assert!(!texts.iter().any(|t| t.contains("omdat")));
    // Parked fragments follow a Heading 1 in the document language.
    let at = paras
        .iter()
        .position(|p| *p == (Some("Heading1".into()), "Niet verbonden".into()))
        .expect("parked heading");
    assert_eq!(paras[at + 1].1, "Loose thought");
}

#[test]
fn text_formats_run_through_the_export_job() {
    use super::{ExportFormat, ExportOptions, ExportWork, Outcome, PageSetup, Snapshot, Target};

    let (mut doc, _) = sample();
    doc.set_essay_text(EssaySlot::Intro, "Intro text").unwrap();
    let dir = tempfile::tempdir().unwrap();
    for (format, name) in [
        (ExportFormat::OutlineDocx, "outline.docx"),
        (ExportFormat::EssayDocx, "essay.docx"),
        (ExportFormat::EssayMarkdown, "essay.md"),
    ] {
        let path = dir.path().join(name);
        let work = ExportWork {
            format,
            options: ExportOptions::default(),
            page: PageSetup::default(),
            snapshot: Snapshot::Doc {
                doc: Box::new(doc.clone()),
                outline: OutlineOptions::default(),
            },
            target: Target::File(path.clone()),
        };
        assert_eq!(work.run().unwrap(), Outcome::Saved(path.clone()));
        let bytes = std::fs::read(&path).unwrap();
        if format == ExportFormat::EssayMarkdown {
            assert!(String::from_utf8(bytes).unwrap().contains("Intro text"));
        } else {
            part(&bytes, "word/document.xml");
        }
    }
}

/// The text of element `tag` (namespace `ns`) in `xml`.
fn element(xml: &str, ns: &str, tag: &str) -> Option<String> {
    let doc = roxmltree::Document::parse(xml).expect("well-formed xml");
    doc.descendants()
        .find(|n| n.has_tag_name((ns, tag)))
        .map(|n| n.text().unwrap_or("").to_owned())
}

const DC: &str = "http://purl.org/dc/elements/1.1/";
const CP: &str = "http://schemas.openxmlformats.org/package/2006/metadata/core-properties";
const DCTERMS: &str = "http://purl.org/dc/terms/";

#[test]
fn docx_core_properties_name_the_author_and_the_time() {
    let (mut doc, _) = sample();
    doc.meta.author = "Ada & <Lovelace>".into();
    let opts = OutlineOptions::default();
    let lines = build_outline(&doc, &opts);
    for bytes in [
        essay_docx(&doc, "en").unwrap(),
        outline_docx(&lines, &opts, "en", &doc.meta.author).unwrap(),
    ] {
        let core = part(&bytes, "docProps/core.xml");
        assert_eq!(
            element(&core, DC, "creator").as_deref(),
            Some("Ada & <Lovelace>")
        );
        assert_eq!(
            element(&core, CP, "lastModifiedBy").as_deref(),
            Some("Ada & <Lovelace>")
        );
        for tag in ["created", "modified"] {
            let stamp = element(&core, DCTERMS, tag).unwrap();
            assert_eq!(stamp.len(), 20, "{stamp}");
            assert!(
                stamp.ends_with('Z') && stamp.as_bytes()[10] == b'T',
                "{stamp}"
            );
            assert!(stamp.as_str() > "2020", "{stamp} is now, not 1970");
        }
        // The rest of the package is intact.
        let paras = paragraphs(&part(&bytes, "word/document.xml"));
        assert_eq!(paras[0].1, "Cities should ban cars");
        part(&bytes, "[Content_Types].xml");
    }
    // Without an author the core properties are still well-formed.
    let bytes = outline_docx(&lines, &opts, "en", "  ").unwrap();
    let core = part(&bytes, "docProps/core.xml");
    assert!(element(&core, DC, "creator").is_some());
}

#[test]
fn docx_text_drops_noncharacters_and_tabs() {
    let (mut doc, _) = sample();
    doc.set_essay_text(EssaySlot::Intro, "One\u{FFFE}\ttwo\u{FFFF} three")
        .unwrap();
    let bytes = essay_docx(&doc, "en").unwrap();
    let xml = part(&bytes, "word/document.xml");
    let paras = paragraphs(&xml);
    assert!(paras.iter().any(|(_, t)| t == "One two three"), "{paras:?}");
    assert!(!xml.contains('\u{FFFE}') && !xml.contains('\u{FFFF}'));
    assert!(!xml.contains('\t'));
}

#[test]
fn docx_sets_a_default_body_font() {
    let (doc, _) = sample();
    let styles = part(&essay_docx(&doc, "en").unwrap(), "word/styles.xml");
    let tree = roxmltree::Document::parse(&styles).unwrap();
    let fonts = tree
        .descendants()
        .find(|n| n.has_tag_name((W, "docDefaults")))
        .and_then(|d| d.descendants().find(|n| n.has_tag_name((W, "rFonts"))))
        .expect("docDefaults has fonts");
    assert_eq!(fonts.attribute((W, "ascii")), Some("Calibri"));
    assert_eq!(fonts.attribute((W, "hAnsi")), Some("Calibri"));
}

#[test]
fn docx_sets_document_language() {
    let (doc, _) = sample();
    let lines = build_outline(&doc, &OutlineOptions::default());
    let opts = OutlineOptions::default();
    for (lang, tag) in [("nl", "nl-NL"), ("en", "en-GB")] {
        for bytes in [
            essay_docx(&doc, lang).unwrap(),
            outline_docx(&lines, &opts, lang, "Ann").unwrap(),
        ] {
            let styles = part(&bytes, "word/styles.xml");
            let tree = roxmltree::Document::parse(&styles).unwrap();
            let lang_el = tree
                .descendants()
                .find(|n| n.has_tag_name((W, "rPrDefault")))
                .and_then(|d| d.descendants().find(|n| n.has_tag_name((W, "lang"))))
                .expect("the default run properties have a language");
            assert_eq!(lang_el.attribute((W, "val")), Some(tag));
            assert!(styles.contains(&format!("w:lang w:val=\"{tag}\"")));
        }
    }
}

#[test]
fn every_ui_language_has_a_word_language_tag() {
    let tags = crate::i18n::UI_LANGUAGES.map(language_tag);
    assert_eq!(tags, ["en-GB", "nl-NL"]);
    assert_eq!(language_tag("nl_BE"), "nl-NL");
    assert_eq!(language_tag("de"), "en-GB");
}
