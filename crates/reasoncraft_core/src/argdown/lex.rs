//! Argdown lines: frontmatter, comments, and what kind each line is.

use super::data::{self, Data};
use super::text::{find, link_at, unescape};

/// A relation sign.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Op {
    Support,
    Attack,
    Undercut,
    Contradict,
}

/// How a relation line relates its element to the line it hangs under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Rel {
    pub op: Op,
    /// `+>`, `->`, `_>`: the line above relates to this element.
    pub outgoing: bool,
    /// Written without a direction (`+`, `-`, `_`).
    pub plain: bool,
}

/// A statement or argument as written on a line.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Element {
    pub argument: bool,
    pub title: Option<String>,
    /// Written as a definition (`[T]: …`, `<T>: …`), or untitled text.
    pub defines: bool,
    /// The text, still with escapes and markup.
    pub text: String,
    pub data: Data,
}

/// What a line holds.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum LineKind {
    Blank,
    Heading(String),
    /// `----`, or `-- rule --` (the rule, if any).
    Inference(Option<String>),
    /// A lone `--` that opens a rule written over several lines.
    InferenceOpen,
    /// `(n) …` in a premise-conclusion structure.
    Premise(Element),
    Relation(Rel, Element),
    Element(Element),
    /// Plain text: an untitled statement, or more text for the line above.
    Text(String),
}

/// A lexed line: 1-based number, indentation (a tab counts 4) and kind.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Line {
    pub number: usize,
    pub indent: usize,
    pub kind: LineKind,
}

/// The frontmatter lines (between `===` lines at the start) and the
/// lexed lines of the rest.
pub(super) fn lex(src: &str) -> (Vec<String>, Vec<Line>) {
    let src = src.strip_prefix('\u{FEFF}').unwrap_or(src);
    let raw: Vec<&str> = src.lines().collect();
    let mut front = Vec::new();
    let mut body_start = 0;
    let first = raw.iter().position(|l| !l.trim().is_empty());
    if let Some(f) = first.filter(|&f| raw[f].trim().starts_with("===")) {
        let close = raw[f + 1..]
            .iter()
            .position(|l| l.trim().starts_with("==="));
        if let Some(c) = close {
            front = raw[f + 1..f + 1 + c]
                .iter()
                .map(|l| l.to_string())
                .collect();
            body_start = f + c + 2;
        }
    }
    let body = strip_comments(&raw[body_start.min(raw.len())..].join("\n"));
    let lines = body
        .split('\n')
        .enumerate()
        .map(|(k, l)| line(body_start + k + 1, l))
        .collect();
    (front, lines)
}

/// `text` with comments (`// …`, `/* … */`, `<!-- … -->`) blanked out;
/// line breaks stay, so line numbers do too. Escaped characters, quoted
/// strings inside data and the `//` of a URL (after `:`) are no comment.
fn strip_comments(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let (mut data, mut quote) = (0usize, false);
    let mut i = 0;
    let starts = |i: usize, s: &str| {
        s.chars()
            .enumerate()
            .all(|(k, c)| chars.get(i + k) == Some(&c))
    };
    while let Some(&c) = chars.get(i) {
        let end = match c {
            '\n' => {
                (data, quote) = (0, false);
                None
            }
            '\\' => {
                out.push(c);
                if let Some(&n) = chars.get(i + 1)
                    && n != '\n'
                {
                    out.push(n);
                    i += 2;
                    continue;
                }
                i += 1;
                continue;
            }
            '"' if data > 0 => {
                quote = !quote;
                None
            }
            _ if quote => None,
            '{' => {
                data += 1;
                None
            }
            '}' => {
                data = data.saturating_sub(1);
                None
            }
            _ if data > 0 => None,
            '/' if starts(i, "//") && (i == 0 || chars[i - 1] != ':') => Some(
                chars[i..]
                    .iter()
                    .position(|&c| c == '\n')
                    .map_or(chars.len(), |n| i + n),
            ),
            '/' if starts(i, "/*") => Some(close_of(&chars, i + 2, "*/")),
            '<' if starts(i, "<!--") => Some(close_of(&chars, i + 4, "-->")),
            _ => None,
        };
        match end {
            Some(end) => {
                for &k in &chars[i..end] {
                    out.push(if k == '\n' { '\n' } else { ' ' });
                }
                i = end;
            }
            None => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

/// The index just after the first `close` at or after `from` (the end
/// when there is none).
fn close_of(chars: &[char], from: usize, close: &str) -> usize {
    let close: Vec<char> = close.chars().collect();
    (from..chars.len())
        .find(|&i| chars[i..].starts_with(&close))
        .map_or(chars.len(), |i| i + close.len())
}

fn line(number: usize, s: &str) -> Line {
    let mut indent = 0;
    for c in s.chars().take_while(|c| c.is_whitespace()) {
        indent += if c == '\t' { 4 } else { 1 };
    }
    Line {
        number,
        indent,
        kind: kind(s.trim()),
    }
}

fn kind(t: &str) -> LineKind {
    if t.is_empty() || t.starts_with("===") {
        return LineKind::Blank;
    }
    if t.starts_with('#') {
        let rest = t.trim_start_matches('#');
        if rest.is_empty() || rest.starts_with(char::is_whitespace) {
            return LineKind::Heading(rest.trim().to_owned());
        }
    }
    if t == "--" {
        return LineKind::InferenceOpen;
    }
    if let Some(rule) = inference(t) {
        return LineKind::Inference(rule);
    }
    if let Some(rest) = premise_number(t) {
        return LineKind::Premise(element(rest));
    }
    if let Some((rel, rest)) = relation(t) {
        return LineKind::Relation(rel, element(rest));
    }
    if t.starts_with('[') || t.starts_with('<') {
        let e = element(t);
        if e.title.is_some() {
            return LineKind::Element(e);
        }
    }
    LineKind::Text(t.to_owned())
}

/// `----` (with any data after it) gives `Some(None)`; `-- rule --`
/// gives the rule.
fn inference(t: &str) -> Option<Option<String>> {
    let hyphens = t.chars().take_while(|&c| c == '-').count();
    let rest = t[hyphens..].trim();
    if hyphens >= 4 && (rest.is_empty() || rest.starts_with('{')) {
        return Some(None);
    }
    let inner = t.strip_prefix("--")?.strip_suffix("--")?;
    let rule = inner.trim_matches('-').trim();
    Some((!rule.is_empty()).then(|| rule.to_owned()))
}

/// For `(n) rest`, the rest.
fn premise_number(t: &str) -> Option<&str> {
    let inner = t.strip_prefix('(')?;
    let digits = inner.chars().take_while(char::is_ascii_digit).count();
    let rest = inner[digits..].strip_prefix(')')?;
    (digits > 0 && (rest.is_empty() || rest.starts_with(char::is_whitespace))).then_some(rest)
}

/// A relation sign at the start of `t`, and the rest.
fn relation(t: &str) -> Option<(Rel, &str)> {
    use Op::{Attack, Contradict, Support, Undercut};
    let signs = [
        ("><", Contradict, false, false),
        ("<+", Support, false, false),
        ("<-", Attack, false, false),
        ("<_", Undercut, false, false),
        ("+>", Support, true, false),
        ("->", Attack, true, false),
        ("_>", Undercut, true, false),
        ("+", Support, false, true),
        ("-", Attack, false, true),
        ("_", Undercut, false, true),
    ];
    for (sign, op, outgoing, plain) in signs {
        let Some(rest) = t.strip_prefix(sign) else {
            continue;
        };
        let spaced = rest.is_empty() || rest.starts_with(char::is_whitespace);
        if spaced || (!plain && (rest.starts_with('[') || rest.starts_with('<'))) {
            return Some((
                Rel {
                    op,
                    outgoing,
                    plain,
                },
                rest.trim(),
            ));
        }
    }
    None
}

/// A statement or argument: `[T]`, `[T]: text`, `<T>`, `<T>: text`, or
/// plain text; each may end in data `{…}`.
pub(super) fn element(t: &str) -> Element {
    let chars: Vec<char> = t.trim().chars().collect();
    let mut e = Element::default();
    let mut body = 0;
    let open = chars.first().copied();
    if let Some(open @ ('[' | '<')) = open
        && (open == '<' || link_at(&chars, 0).is_none())
    {
        let close = if open == '[' { ']' } else { '>' };
        if let Some(end) = find(&chars, 1, close).filter(|&end| end > 1) {
            let title: String = chars[1..end].iter().collect();
            e.argument = open == '<';
            e.title = Some(unescape(&title)).filter(|t| !t.is_empty());
            body = end + 1;
            if chars.get(body) == Some(&':') {
                e.defines = true;
                body += 1;
            }
        }
    }
    if e.title.is_none() {
        (e.argument, e.defines, body) = (false, true, 0);
    }
    // The text runs up to the data; anything after the data is text too.
    let mut text = String::new();
    let mut i = body;
    while let Some(&c) = chars.get(i) {
        match c {
            '\\' => {
                text.extend(chars[i..(i + 2).min(chars.len())].iter());
                i += 2;
            }
            '{' if e.data.is_empty() => {
                let (data, end) = data::parse_map(&chars, i);
                e.data = data;
                i = end.max(i + 1);
            }
            c => {
                text.push(c);
                i += 1;
            }
        }
    }
    e.text = text.trim().to_owned();
    e
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_of_lines() {
        let k = |s: &str| kind(s);
        assert_eq!(
            k("# Not connected"),
            LineKind::Heading("Not connected".into())
        );
        assert!(matches!(k("#tag text"), LineKind::Text(_)));
        assert_eq!(k("----"), LineKind::Inference(None));
        assert_eq!(
            k("-- Modus ponens --"),
            LineKind::Inference(Some("Modus ponens".into()))
        );
        assert_eq!(k("--"), LineKind::InferenceOpen);
        let LineKind::Premise(p) = k("(2) [T]: text {acceptance: accept}") else {
            panic!()
        };
        assert_eq!(
            (p.title.as_deref(), p.text.as_str(), p.defines),
            (Some("T"), "text", true)
        );
        assert_eq!(p.data.len(), 1);
        let LineKind::Relation(r, e) = k("<- <O1>") else {
            panic!()
        };
        assert_eq!(
            (r.op, r.outgoing, e.argument, e.defines),
            (Op::Attack, false, true, false)
        );
        let LineKind::Relation(r, _) = k("-> [X]") else {
            panic!()
        };
        assert!(r.outgoing && r.op == Op::Attack);
        assert!(matches!(k("[a](http://b.c) text"), LineKind::Text(_)));
        assert!(matches!(k("-x"), LineKind::Text(_)));
    }

    #[test]
    fn comments_are_blanked() {
        let s =
            "a // c\nb /* x\ny */ z <!-- q --> w\n[T]: t {link: \"https://a.b\"} // c\nhttp://y.z";
        let out = strip_comments(s);
        assert_eq!(out.lines().count(), 5);
        assert!(!out.contains('c') && !out.contains('x') && !out.contains('q'));
        assert!(out.contains("https://a.b") && out.contains("http:"));
        assert!(strip_comments(r"a \// b").contains("b"));
    }

    #[test]
    fn frontmatter_is_split_off() {
        let (front, lines) = lex("===\ntitle: \"T\"\n===\n\n[A]: a");
        assert_eq!(front, vec!["title: \"T\"".to_owned()]);
        assert_eq!(lines.last().map(|l| l.number), Some(5));
    }
}
