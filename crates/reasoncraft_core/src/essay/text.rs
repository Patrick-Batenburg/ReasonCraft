//! The essay as Markdown and as plain text.
//!
//! Both list a title (the document title, else the claim), the
//! introduction, one section per branch in [`essay_order`], and the
//! conclusion. Written paragraphs appear as written; a slot without text
//! shows its map statement as a hint (italic in Markdown, in brackets in
//! plain text). The introduction and conclusion hint at the claim.

use crate::model::Document;
use crate::outline_md::{content_lines, escape};
use crate::words::{EssayHeading, essay_heading};

use super::{EssaySlot, essay_order};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Flavor {
    Markdown,
    Plain,
}

/// `text` on one line: runs of whitespace become one space.
fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

struct Writer<'a> {
    doc: &'a Document,
    flavor: Flavor,
    blocks: Vec<String>,
}

impl Writer<'_> {
    fn heading(&mut self, level: usize, text: &str) {
        let text = one_line(text);
        self.blocks.push(match self.flavor {
            Flavor::Markdown => {
                let marks = "#".repeat(level);
                if text.is_empty() {
                    marks
                } else {
                    format!("{marks} {}", escape(&text, false))
                }
            }
            Flavor::Plain => text,
        });
    }

    /// The paragraph written for `slot`, or a hint built from `statement`.
    fn slot(&mut self, slot: EssaySlot, statement: &str) {
        let written = self.doc.essay.text(slot).map(content_lines);
        if let Some(lines) = written.filter(|l| !l.is_empty()) {
            let lines: Vec<String> = match self.flavor {
                Flavor::Markdown => lines.iter().map(|l| escape(l, true)).collect(),
                Flavor::Plain => lines.iter().map(|l| (*l).to_owned()).collect(),
            };
            self.blocks.push(lines.join("\n"));
            return;
        }
        let hint = one_line(statement);
        if hint.is_empty() {
            return;
        }
        self.blocks.push(match self.flavor {
            Flavor::Markdown => format!("*{}*", escape(&hint, false)),
            Flavor::Plain => format!("[{hint}]"),
        });
    }
}

fn essay_text(doc: &Document, lang: &str, flavor: Flavor) -> String {
    let claim = doc.boxes.get(&doc.root).map_or("", |n| n.text.as_str());
    let mut w = Writer {
        doc,
        flavor,
        blocks: Vec::new(),
    };
    let title = doc.meta.title.trim();
    let title = if title.is_empty() { claim } else { title };
    if !one_line(title).is_empty() {
        w.heading(1, title);
    }
    w.heading(2, essay_heading(lang, EssayHeading::Introduction));
    w.slot(EssaySlot::Intro, claim);
    let section = essay_heading(lang, EssayHeading::Section);
    for (n, s) in essay_order(doc).iter().enumerate() {
        let own: Vec<String> = s
            .elements
            .iter()
            .filter(|e| e.depth == 0)
            .map(|e| one_line(&e.text))
            .filter(|t| !t.is_empty())
            .collect();
        let mut heading = format!("{section} {}", n + 1);
        if !own.is_empty() {
            heading = format!("{heading}: {}", own.join("; "));
        }
        w.heading(2, &heading);
        for e in &s.elements {
            w.slot(e.slot, &e.text);
        }
    }
    w.heading(2, essay_heading(lang, EssayHeading::Conclusion));
    w.slot(EssaySlot::Conclusion, claim);
    w.blocks.join("\n\n")
}

/// The essay as Markdown: `#` title, `##` headings in the document
/// language, paragraphs, and italic hints for slots without text.
pub fn essay_markdown(doc: &Document, lang: &str) -> String {
    essay_text(doc, lang, Flavor::Markdown)
}

/// The essay as plain text: the same structure as [`essay_markdown`]
/// without markup; hints are in square brackets.
pub fn essay_plain_text(doc: &Document, lang: &str) -> String {
    essay_text(doc, lang, Flavor::Plain)
}
