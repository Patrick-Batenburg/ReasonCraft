//! The help pages' Markdown subset, read and drawn.
//!
//! Supported: `# ` and `## ` headings, paragraphs (lines up to an empty
//! line), list items (`- `), `**bold**`, `` `code` `` and links
//! `[text](topic-id)` to other topics. Everything else, including HTML,
//! other headings and lists, and links to anything but a topic, is plain
//! text. Nothing is ever fetched or opened outside the app.

use eframe::egui::{self, FontSelection, Label, RichText, Ui, text::LayoutJob};

use crate::theme::Tokens;

/// A run of text within a block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Span {
    Text(String),
    Bold(String),
    Code(String),
    /// A link to another topic (by its id).
    Link {
        text: String,
        topic: &'static str,
    },
}

/// A block of a page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    /// A heading of level 1 (`#`) or 2 (`##`).
    Heading(u8, Vec<Span>),
    Paragraph(Vec<Span>),
    /// One item of a list.
    Item(Vec<Span>),
}

impl Block {
    pub fn spans(&self) -> &[Span] {
        match self {
            Block::Heading(_, s) | Block::Paragraph(s) | Block::Item(s) => s,
        }
    }
}

/// Read `src` into blocks.
pub fn parse(src: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    // The paragraph or list item being read: is it an item, and its text.
    let mut open: Option<(bool, String)> = None;
    let flush = |open: &mut Option<(bool, String)>, blocks: &mut Vec<Block>| {
        if let Some((item, text)) = open.take() {
            let spans = inline(&text);
            blocks.push(if item {
                Block::Item(spans)
            } else {
                Block::Paragraph(spans)
            });
        }
    };
    for line in src.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            flush(&mut open, &mut blocks);
        } else if let Some((level, text)) = heading(line) {
            flush(&mut open, &mut blocks);
            blocks.push(Block::Heading(level, inline(text.trim())));
        } else if let Some(text) = line.strip_prefix("- ") {
            flush(&mut open, &mut blocks);
            open = Some((true, text.trim().to_owned()));
        } else if let Some((_, text)) = &mut open {
            text.push(' ');
            text.push_str(trimmed);
        } else {
            open = Some((false, trimmed.to_owned()));
        }
    }
    flush(&mut open, &mut blocks);
    blocks
}

fn heading(line: &str) -> Option<(u8, &str)> {
    if let Some(text) = line.strip_prefix("# ") {
        Some((1, text))
    } else {
        line.strip_prefix("## ").map(|text| (2, text))
    }
}

/// The spans of one block's text.
fn inline(text: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut plain = String::new();
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        if let Some((span, len)) = markup(rest) {
            if !plain.is_empty() {
                spans.push(Span::Text(std::mem::take(&mut plain)));
            }
            spans.push(span);
            rest = &rest[len..];
        } else {
            plain.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    if !plain.is_empty() {
        spans.push(Span::Text(plain));
    }
    spans
}

/// The bold, code or link span `rest` starts with, and its length in
/// bytes; `None` when it starts with none (an unclosed marker, an empty
/// one, or a link to something that is not a topic).
fn markup(rest: &str) -> Option<(Span, usize)> {
    if let Some(after) = rest.strip_prefix("**") {
        let end = after.find("**").filter(|&e| e > 0)?;
        Some((Span::Bold(after[..end].to_owned()), end + 4))
    } else if let Some(after) = rest.strip_prefix('`') {
        let end = after.find('`').filter(|&e| e > 0)?;
        Some((Span::Code(after[..end].to_owned()), end + 2))
    } else if let Some(after) = rest.strip_prefix('[') {
        let close = after.find("](").filter(|&c| c > 0)?;
        let text = &after[..close];
        if text.contains(['[', ']']) {
            return None;
        }
        let target = &after[close + 2..];
        let end = target.find(')')?;
        let topic = super::topic(&target[..end])?;
        let span = Span::Link {
            text: text.to_owned(),
            topic,
        };
        Some((span, 1 + close + 2 + end + 1))
    } else {
        None
    }
}

/// Body text size; headings are larger.
const BODY: f32 = 13.0;

/// Draw `blocks` and return the topic of a link clicked this frame.
/// Body text is muted and bold text full strength, so bold stands out
/// without a bold font; both meet the contrast rules.
pub fn show(ui: &mut Ui, blocks: &[Block], tk: &Tokens) -> Option<&'static str> {
    let mut clicked = None;
    ui.spacing_mut().item_spacing.y = 6.0;
    for (i, block) in blocks.iter().enumerate() {
        match block {
            Block::Heading(level, spans) => {
                if i > 0 {
                    ui.add_space(if *level == 1 { 8.0 } else { 4.0 });
                }
                let size = if *level == 1 { 19.0 } else { 15.5 };
                let text: String = spans.iter().map(span_text).collect();
                let rich = RichText::new(text).size(size).strong().color(tk.box_text);
                ui.add(Label::new(rich).wrap().selectable(false));
            }
            Block::Paragraph(spans) => clicked = clicked.or(paragraph(ui, spans, tk)),
            Block::Item(spans) => {
                ui.horizontal_top(|ui| {
                    let bullet = RichText::new("•").size(BODY).color(tk.text_muted);
                    ui.add(Label::new(bullet).selectable(false));
                    ui.vertical(|ui| {
                        ui.set_width(ui.available_width());
                        clicked = clicked.or(paragraph(ui, spans, tk));
                    });
                });
            }
        }
    }
    clicked
}

fn span_text(span: &Span) -> &str {
    match span {
        Span::Text(s) | Span::Bold(s) | Span::Code(s) => s,
        Span::Link { text, .. } => text,
    }
}

/// How a span other than a link looks.
fn rich(span: &Span, tk: &Tokens) -> RichText {
    match span {
        Span::Bold(s) => RichText::new(s).size(BODY).strong().color(tk.box_text),
        Span::Code(s) => RichText::new(s).size(BODY).code().color(tk.box_text),
        other => RichText::new(span_text(other))
            .size(BODY)
            .color(tk.text_muted),
    }
}

/// A paragraph: one wrapped label (one name for screen readers), or, when
/// it holds links, its spans and links in a wrapping row.
fn paragraph(ui: &mut Ui, spans: &[Span], tk: &Tokens) -> Option<&'static str> {
    if !spans.iter().any(|s| matches!(s, Span::Link { .. })) {
        let mut job = LayoutJob::default();
        for span in spans {
            rich(span, tk).append_to(
                &mut job,
                ui.style(),
                FontSelection::Default,
                egui::Align::Center,
            );
        }
        ui.add(Label::new(job).wrap().selectable(false));
        return None;
    }
    let mut clicked = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for span in spans {
            match span {
                Span::Link { text, topic } => {
                    // A link stays on one line: the rows of a wrapped one
                    // would be separate, nameless click targets.
                    let wrap = ui.style().wrap_mode;
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                    let link = RichText::new(text).size(BODY).underline();
                    if ui.link(link).clicked() {
                        clicked = Some(*topic);
                    }
                    ui.style_mut().wrap_mode = wrap;
                }
                other => {
                    ui.add(Label::new(rich(other, tk)).wrap().selectable(false));
                }
            }
        }
    });
    clicked
}
