//! Argdown comments (`// …`, `/* … */`, `<!-- … -->`), blanked out
//! before lines are read.

/// `text` with comments (`// …`, `/* … */`, `<!-- … -->`) blanked out;
/// line breaks stay, so line numbers do too. Escaped characters, quoted
/// strings inside data and the `//` of a URL (after `:`) are no comment.
pub(super) fn strip_comments(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    // Inside data: the brace depth and the open quote, if any.
    let (mut data, mut quote) = (0usize, None::<char>);
    let mut i = 0;
    let starts = |i: usize, s: &str| {
        s.chars()
            .enumerate()
            .all(|(k, c)| chars.get(i + k) == Some(&c))
    };
    while let Some(&c) = chars.get(i) {
        let end = match c {
            '\n' => {
                (data, quote) = (0, None);
                None
            }
            // YAML single quotes: no escapes but `''`.
            '\'' if quote == Some('\'') => {
                if chars.get(i + 1) == Some(&'\'') {
                    out.push_str("''");
                    i += 2;
                    continue;
                }
                quote = None;
                None
            }
            _ if quote == Some('\'') => None,
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
                quote = if quote.is_some() { None } else { Some('"') };
                None
            }
            _ if quote.is_some() => None,
            // A quote opens a quoted value only where a value starts.
            '\'' if data > 0 && opens_value(&chars[..i]) => {
                quote = Some('\'');
                None
            }
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

/// Whether a value starts after `before`: its last non-space character
/// is `{`, `[`, `,` or `:`.
fn opens_value(before: &[char]) -> bool {
    let last = before.iter().rev().find(|c| !c.is_whitespace());
    matches!(last, Some('{' | '[' | ',' | ':'))
}

/// The index just after the first `close` at or after `from` (the end
/// when there is none).
fn close_of(chars: &[char], from: usize, close: &str) -> usize {
    let close: Vec<char> = close.chars().collect();
    (from..chars.len())
        .find(|&i| chars[i..].starts_with(&close))
        .map_or(chars.len(), |i| i + close.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comments_are_blanked() {
        let s =
            "a // c\nb /* x\ny */ z <!-- q --> w\n[T]: t {link: \"https://a.b\"} // c\nhttp://y.z";
        let out = strip_comments(s);
        assert_eq!(out.lines().count(), 5);
        assert!(!out.contains('c') && !out.contains('x') && !out.contains('q'));
        assert!(out.contains("https://a.b") && out.contains("http:"));
        assert!(strip_comments(r"a \// b").contains("b"));
        // Single-quoted data values keep braces and slashes; an apostrophe
        // in a plain value opens nothing.
        let kept = "[T]: t {a: 'x } y // z', b: Sam's} // gone";
        let out = strip_comments(kept);
        assert!(out.contains("'x } y // z', b: Sam's}") && !out.contains("gone"));
    }
}
