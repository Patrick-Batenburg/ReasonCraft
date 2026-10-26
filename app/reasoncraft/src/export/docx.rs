//! The outline and the essay as Word (.docx) files.
//!
//! Paragraphs use Word's built-in style ids (`Title`, `Heading1`,
//! `Normal`), which [`base`] defines in `styles.xml` (the writer defines
//! none), so Word and LibreOffice show them in their style lists and
//! navigation pane. Text is the document's own, in the document language;
//! hints are grey like the muted text of the light theme.

use std::io::Cursor;

use docx_rs::{BreakType, Docx, Paragraph, Run, RunFonts, Style, StyleType};
use reasoncraft_core::essay::essay_order;
use reasoncraft_core::outline::{LineKind, OutlineLine, OutlineOptions};
use reasoncraft_core::words::{
    EssayHeading, essay_heading, linking_words, not_connected_heading, source_word,
};
use reasoncraft_core::{Document, EssaySlot};

use super::ExportError;
use super::docx_props::{finish_package, now_w3cdtf};
use crate::theme::Tokens;

/// Left indent per outline level: 0.6 cm in twentieths of a point.
pub(crate) const INDENT_TWIPS: i32 = 340;

/// Half-points: 11 pt text, 26 pt title, 16 pt headings.
const TEXT_SIZE: usize = 22;
const TITLE_SIZE: usize = 52;
const HEADING_SIZE: usize = 32;
/// The body font: Word's long-time default, which LibreOffice maps to the
/// metric-compatible Carlito.
const BODY_FONT: &str = "Calibri";

/// The document with the styles every paragraph below refers to.
fn base() -> Docx {
    let title = Style::new("Title", StyleType::Paragraph)
        .name("Title")
        .based_on("Normal")
        .next("Normal")
        .size(TITLE_SIZE)
        .bold();
    let heading = Style::new("Heading1", StyleType::Paragraph)
        .name("heading 1")
        .based_on("Normal")
        .next("Normal")
        .size(HEADING_SIZE)
        .bold()
        .outline_lvl(0);
    let fonts = RunFonts::new()
        .ascii(BODY_FONT)
        .hi_ansi(BODY_FONT)
        .cs(BODY_FONT)
        .east_asia(BODY_FONT);
    let now = now_w3cdtf();
    // The writer already defines `Normal`; only its size is set here.
    Docx::new()
        .default_size(TEXT_SIZE)
        .default_fonts(fonts)
        .created_at(&now)
        .updated_at(&now)
        .add_style(title)
        .add_style(heading)
}

/// The finished package, with `author` as its creator and its text in
/// document language `lang`.
fn pack(docx: Docx, author: &str, lang: &str) -> Result<Vec<u8>, ExportError> {
    let mut out = Cursor::new(Vec::new());
    docx.build()
        .pack(&mut out)
        .map_err(|e| ExportError::Render(e.to_string()))?;
    finish_package(out.into_inner(), &one_line(author), language_tag(lang))
}

/// Word's language tag for document language `lang`: Dutch for `nl`
/// (`nl-NL`, `nl_BE`, ...), British English otherwise, matching the
/// headings and linking words, which fall back to English.
fn language_tag(lang: &str) -> &'static str {
    let primary = lang.split(['-', '_']).next().unwrap_or_default();
    if primary.eq_ignore_ascii_case("nl") {
        "nl-NL"
    } else {
        "en-GB"
    }
}

/// `s` without the characters XML 1.0 cannot hold (control characters,
/// U+FFFE and U+FFFF); a tab becomes a space, as a run's text keeps no tabs.
fn clean(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(c, '\u{FFFE}' | '\u{FFFF}'))
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\t'))
        .map(|c| if c == '\t' { ' ' } else { c })
        .collect()
}

/// `text` on one line: runs of whitespace become one space.
fn one_line(text: &str) -> String {
    clean(text).split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The lines of `text` without leading and trailing blank lines.
fn content_lines(text: &str) -> Vec<String> {
    let lines: Vec<String> = clean(text)
        .lines()
        .map(|l| l.trim_end().to_owned())
        .collect();
    let blank = |l: &String| l.trim().is_empty();
    let Some(first) = lines.iter().position(|l| !blank(l)) else {
        return Vec::new();
    };
    let last = lines.iter().rposition(|l| !blank(l)).unwrap_or(first);
    lines[first..=last].to_vec()
}

/// A run with each line of `lines` after the first on a new row.
fn text_run(lines: &[String]) -> Run {
    let mut run = Run::new();
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            run = run.add_break(BreakType::TextWrapping);
        }
        run = run.add_text(line);
    }
    run
}

fn styled(style: &str, text: &str) -> Paragraph {
    Paragraph::new()
        .style(style)
        .add_run(Run::new().add_text(one_line(text)))
}

fn heading(text: &str) -> Paragraph {
    styled("Heading1", text)
}

/// The outline, by `author`: the claim as title, one indented paragraph per line, and
/// parked fragments under a heading in the document language.
pub fn outline_docx(
    lines: &[OutlineLine],
    opts: &OutlineOptions,
    lang: &str,
    author: &str,
) -> Result<Vec<u8>, ExportError> {
    let words = linking_words(lang);
    let mut docx = base();
    let mut any_parked = false;
    for line in lines {
        if line.parked && !any_parked {
            any_parked = true;
            docx = docx.add_paragraph(heading(not_connected_heading(lang)));
        }
        if matches!(line.kind, LineKind::Claim) && !line.parked {
            docx = docx.add_paragraph(styled("Title", &line.text));
            continue;
        }
        let depth = i32::try_from(line.depth.min(1000)).unwrap_or(1000);
        let mut p =
            Paragraph::new()
                .style("Normal")
                .indent(Some(INDENT_TWIPS * depth), None, None, None);
        if let LineKind::Source(t) = &line.kind {
            let label = format!("{}: ", source_word(t, lang));
            p = p.add_run(Run::new().add_text(label).italic());
        } else {
            if opts.numbering && !line.number.is_empty() {
                let number = format!("{} ", one_line(&line.number));
                p = p.add_run(Run::new().add_text(number).bold());
            }
            let word = words.word(line.label);
            if opts.labels && !word.is_empty() {
                p = p.add_run(Run::new().add_text(format!("{word} ")).italic());
            }
        }
        p = p.add_run(text_run(&content_lines(&line.text)));
        docx = docx.add_paragraph(p);
    }
    pack(docx, author, lang)
}

/// `RRGGBB` for the muted text of the light theme.
fn muted_hex() -> String {
    let c = Tokens::light().text_muted;
    format!("{:02X}{:02X}{:02X}", c.r(), c.g(), c.b())
}

/// The paragraphs of `slot`: what was written (one Normal paragraph per
/// line), else `statement` as an italic grey hint.
fn slot(mut docx: Docx, doc: &Document, slot: EssaySlot, statement: &str) -> Docx {
    let written: Vec<String> = doc
        .essay
        .text(slot)
        .map(content_lines)
        .unwrap_or_default()
        .into_iter()
        .filter(|l| !l.trim().is_empty())
        .collect();
    if !written.is_empty() {
        for line in written {
            docx = docx.add_paragraph(
                Paragraph::new()
                    .style("Normal")
                    .add_run(Run::new().add_text(line)),
            );
        }
        return docx;
    }
    let hint = one_line(statement);
    if hint.is_empty() {
        return docx;
    }
    docx.add_paragraph(
        Paragraph::new()
            .style("Normal")
            .add_run(Run::new().add_text(hint).italic().color(muted_hex())),
    )
}

/// The essay: title (the document title, else the claim), the author,
/// the introduction, one section per branch and the conclusion.
pub fn essay_docx(doc: &Document, lang: &str) -> Result<Vec<u8>, ExportError> {
    let claim = doc.boxes.get(&doc.root).map_or("", |n| n.text.as_str());
    let mut docx = base();
    let title = doc.meta.title.trim();
    let title = if title.is_empty() { claim } else { title };
    if !one_line(title).is_empty() {
        docx = docx.add_paragraph(styled("Title", title));
    }
    let author = one_line(&doc.meta.author);
    if !author.is_empty() {
        docx = docx.add_paragraph(styled("Normal", &author));
    }
    docx = docx.add_paragraph(heading(essay_heading(lang, EssayHeading::Introduction)));
    docx = slot(docx, doc, EssaySlot::Intro, claim);
    let word = essay_heading(lang, EssayHeading::Section);
    for (n, section) in essay_order(doc).iter().enumerate() {
        let own: Vec<String> = section
            .elements
            .iter()
            .filter(|e| e.depth == 0)
            .map(|e| one_line(&e.text))
            .filter(|t| !t.is_empty())
            .collect();
        let mut text = format!("{word} {}", n + 1);
        if !own.is_empty() {
            text = format!("{text}: {}", own.join("; "));
        }
        docx = docx.add_paragraph(heading(&text));
        for e in &section.elements {
            docx = slot(docx, doc, e.slot, &e.text);
        }
    }
    docx = docx.add_paragraph(heading(essay_heading(lang, EssayHeading::Conclusion)));
    docx = slot(docx, doc, EssaySlot::Conclusion, claim);
    pack(docx, &doc.meta.author, lang)
}
