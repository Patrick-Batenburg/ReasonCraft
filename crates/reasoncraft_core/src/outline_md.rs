//! The outline as Markdown, and as plain text in the document language.

use crate::model::SourceType;
use crate::outline::{LineKind, OutlineLine, OutlineOptions, outline_plain_text};
use crate::words::{linking_words, not_connected_heading, source_word};

/// Escape Markdown specials in `s`. At the start of a line, a leading
/// `-`, `+` or `1.` would begin a list, so those are escaped too.
fn escape(s: &str, line_start: bool) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    let mut rest = s;
    if line_start {
        let digits = s.bytes().take_while(u8::is_ascii_digit).count();
        if digits > 0 && s[digits..].starts_with(['.', ')']) {
            out.push_str(&s[..digits]);
            out.push('\\');
            rest = &s[digits..];
        } else if s.starts_with(['-', '+']) {
            out.push('\\');
        }
    }
    for c in rest.chars() {
        if matches!(c, '\\' | '*' | '_' | '#' | '`' | '[' | ']' | '<' | '>') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

fn push_item(
    out: &mut String,
    line: &OutlineLine,
    indent: usize,
    opts: &OutlineOptions,
    lang: &str,
) {
    let pad = " ".repeat(indent);
    let mut head: Vec<String> = Vec::with_capacity(3);
    let mut text = line.text.lines();
    let first = text.next().unwrap_or_default();
    if let LineKind::Source(t) = &line.kind {
        head.push(format!("_{}:_", source_word(t, lang)));
    } else {
        if opts.numbering && !line.number.is_empty() {
            head.push(format!("**{}**", escape(&line.number, false)));
        }
        let word = linking_words(lang).word(line.label);
        if opts.labels && !word.is_empty() {
            head.push(format!("*{word}*"));
        }
    }
    let first = escape(first, head.is_empty());
    if !first.is_empty() {
        head.push(first);
    }
    out.push_str(&pad);
    out.push('-');
    if !head.is_empty() {
        out.push(' ');
        out.push_str(&head.join(" "));
    }
    out.push('\n');
    for more in text {
        if !more.is_empty() {
            out.push_str(&pad);
            out.push_str("  ");
            out.push_str(&escape(more, true));
        }
        out.push('\n');
    }
}

/// The outline as Markdown: the claim as `#` heading, other lines as nested
/// list items (two spaces per level below the claim), parked fragments under
/// a second heading in the document language.
pub fn outline_markdown(lines: &[OutlineLine], opts: &OutlineOptions, lang: &str) -> String {
    let mut out = String::new();
    let mut any_parked = false;
    for line in lines {
        if line.parked && !any_parked {
            any_parked = true;
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str("## ");
            out.push_str(not_connected_heading(lang));
            out.push_str("\n\n");
        }
        if matches!(line.kind, LineKind::Claim) && !line.parked {
            let text = line.text.lines().collect::<Vec<_>>().join(" ");
            out.push('#');
            let text = escape(text.trim(), false);
            if !text.is_empty() {
                out.push(' ');
                out.push_str(&text);
            }
            out.push_str("\n\n");
        } else {
            let level = if line.parked {
                line.depth
            } else {
                line.depth.saturating_sub(1)
            };
            push_item(&mut out, line, level * 2, opts, lang);
        }
    }
    out.truncate(out.trim_end().len());
    out
}

/// Indented plain text with the words of the document language.
pub fn outline_plain_text_lang(lines: &[OutlineLine], opts: &OutlineOptions, lang: &str) -> String {
    let words = linking_words(lang);
    outline_plain_text(
        lines,
        opts,
        &|l| words.word(l).to_owned(),
        &|t: &SourceType| source_word(t, lang).to_owned(),
    )
}
