//! The help pages' Markdown subset, read and drawn.
//!
//! Supported: `# ` and `## ` headings, paragraphs (lines up to an empty
//! line), list items (`- `), `**bold**`, `` `code` `` and links
//! `[text](topic-id)` to other topics. Everything else, including HTML,
//! other headings and lists, and links to anything but a topic, is plain
//! text. Nothing is ever fetched or opened outside the app.

use std::ops::Range;

use eframe::egui::{
    Align, CursorIcon, FontSelection, Galley, Label, Pos2, Rangef, Rect, RichText, Role, Sense,
    StrokeKind, Ui, WidgetInfo, text::LayoutJob, vec2,
};

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

/// A paragraph: one wrapped text, its bold, code and links in it, so the
/// words wrap as one text (a bold word never leaves a line starting with
/// a space) and never spill past the width it is given. Each link is a
/// named, focusable link over its first line; a click anywhere on its
/// lines opens its topic.
fn paragraph(ui: &mut Ui, spans: &[Span], tk: &Tokens) -> Option<&'static str> {
    let mut job = LayoutJob::default();
    // Each link's characters in the text, its text and its topic.
    let mut links = Vec::new();
    for span in spans {
        let text = match span {
            Span::Link { text, .. } => RichText::new(text)
                .size(BODY)
                .underline()
                .color(ui.visuals().hyperlink_color),
            other => rich(other, tk),
        };
        let start = job.text.chars().count();
        text.append_to(&mut job, ui.style(), FontSelection::Default, Align::Center);
        if let Span::Link { text, topic } = span {
            links.push((start..job.text.chars().count(), text, *topic));
        }
    }
    let width = ui.available_width();
    job.wrap.max_width = width;
    job.halign = Align::Min;
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    let size = vec2(galley.size().x.min(width), galley.size().y);
    // As a label senses: focusable for a screen reader; clicks for links.
    let mut sense = if ui.memory(|m| m.options.screen_reader) {
        Sense::focusable_noninteractive()
    } else {
        Sense::hover()
    };
    if !links.is_empty() {
        sense |= Sense::CLICK;
    }
    let (rect, response) = ui.allocate_exact_size(size, sense);
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, galley.text()));
    let mut clicked = None;
    let pointer = response.hover_pos();
    for (chars, text, topic) in links {
        let rows = char_rects(&galley, chars, rect.min);
        let Some(&first) = rows.first() else {
            continue;
        };
        let link = ui.interact(first, response.id.with(text), Sense::click());
        link.widget_info(|| WidgetInfo::labeled(Role::Link, true, text));
        let on_rows = |at: Option<Pos2>| at.is_some_and(|p| rows.iter().any(|r| r.contains(p)));
        if link.hovered() || on_rows(pointer) {
            ui.set_cursor_icon(CursorIcon::PointingHand);
        }
        if link.clicked() || (response.clicked() && on_rows(response.interact_pointer_pos())) {
            clicked = Some(topic);
        }
        if link.has_focus() {
            let stroke = ui.visuals().selection.stroke;
            for r in &rows {
                ui.painter()
                    .rect_stroke(r.expand(1.0), 2.0, stroke, StrokeKind::Outside);
            }
        }
    }
    ui.painter().galley(rect.min, galley, tk.text_muted);
    clicked
}

/// The rects, one per visual line, of the characters `chars` of `galley`
/// painted at `origin`.
fn char_rects(galley: &Galley, chars: Range<usize>, origin: Pos2) -> Vec<Rect> {
    let mut rects = Vec::new();
    let mut at = 0;
    for row in &galley.rows {
        let mut x = Rangef::NOTHING;
        for (i, glyph) in row.glyphs.iter().enumerate() {
            if chars.contains(&(at + i)) {
                x.min = x.min.min(glyph.pos.x);
                x.max = x.max.max(glyph.max_x());
            }
        }
        if x.min < x.max {
            let y = row.rect().y_range();
            rects.push(Rect::from_x_y_ranges(x, y).translate(origin.to_vec2()));
        }
        at += row.char_count_including_newline().0;
    }
    rects
}
