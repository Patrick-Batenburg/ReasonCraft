//! The outline as Markdown, and as plain text in the document language.

use crate::caps::MAX_INDENT;
use crate::model::SourceType;
use crate::outline::{LineKind, OutlineLine, OutlineOptions, outline_plain_text};
use crate::words::{linking_words, not_connected_heading};

/// The text of an i18n key (such as `source.law`) in the document
/// language, from the app's translations: the core has no source-type
/// names of its own.
pub type Label<'a> = &'a dyn Fn(&str) -> String;

/// The name of `source_type`, looked up as `source.<key>`.
pub fn source_name(source_type: &SourceType, label: Label) -> String {
    label(&format!("source.{}", source_type.key()))
}

/// Escape Markdown specials in `s` (including `|` for tables, `&` for
/// entities and `~` for strikethrough). At the start of a line, a leading
/// `-`, `+` or `1.` would begin a list and a leading `=` could underline
/// the line above as a heading, so those are escaped too; leading spaces
/// and tabs (four would begin a code block) become `&#32;` and `&#9;`.
pub(crate) fn escape(s: &str, line_start: bool) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    let mut rest = s;
    if line_start {
        let indent = s.len() - s.trim_start_matches([' ', '\t']).len();
        for c in s[..indent].chars() {
            out.push_str(if c == '\t' { "&#9;" } else { "&#32;" });
        }
        let s = &s[indent..];
        rest = s;
        let digits = s.bytes().take_while(u8::is_ascii_digit).count();
        if digits > 0 && s[digits..].starts_with(['.', ')']) {
            out.push_str(&s[..digits]);
            out.push('\\');
            rest = &s[digits..];
        } else if s.starts_with(['-', '+', '=']) {
            out.push('\\');
        }
    }
    for c in rest.chars() {
        if matches!(
            c,
            '\\' | '*' | '_' | '#' | '`' | '[' | ']' | '<' | '>' | '|' | '&' | '~'
        ) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// The lines of `s` without its leading and trailing blank
/// (whitespace-only) lines; blank lines inside are kept.
pub(crate) fn content_lines(s: &str) -> Vec<&str> {
    let lines: Vec<&str> = s.lines().collect();
    let blank = |l: &&str| l.trim().is_empty();
    let Some(first) = lines.iter().position(|l| !blank(l)) else {
        return Vec::new();
    };
    let last = lines.iter().rposition(|l| !blank(l)).unwrap_or(first);
    lines[first..=last].to_vec()
}

fn push_item(
    out: &mut String,
    line: &OutlineLine,
    indent: usize,
    opts: &OutlineOptions,
    lang: &str,
    label: Label,
) {
    let pad = " ".repeat(indent);
    let mut head: Vec<String> = Vec::with_capacity(3);
    let lines = content_lines(&line.text);
    let mut text = lines.iter().copied();
    let first = text.next().unwrap_or_default();
    if let LineKind::Source(t) = &line.kind {
        head.push(format!("_{}:_", source_name(t, label)));
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
/// a second heading in the document language `lang`; source-type names
/// come from `label`.
pub fn outline_markdown(
    lines: &[OutlineLine],
    opts: &OutlineOptions,
    lang: &str,
    label: Label,
) -> String {
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
            push_item(&mut out, line, level.min(MAX_INDENT) * 2, opts, lang, label);
        }
    }
    out.truncate(out.trim_end().len());
    out
}

/// Indented plain text with the linking words of the document language
/// `lang`; source-type names come from `label`.
pub fn outline_plain_text_lang(
    lines: &[OutlineLine],
    opts: &OutlineOptions,
    lang: &str,
    label: Label,
) -> String {
    let words = linking_words(lang);
    outline_plain_text(
        lines,
        opts,
        &|l| words.word(l).to_owned(),
        &|t: &SourceType| source_name(t, label),
    )
}
