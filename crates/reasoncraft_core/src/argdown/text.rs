//! Argdown text to plain box text: escapes, inline formatting, links,
//! mentions and tags.

/// Statement text after clean-up.
#[derive(Debug, Default, PartialEq)]
pub(super) struct Cleaned {
    pub text: String,
    /// The first `[text](url)` link's URL.
    pub link: Option<String>,
    /// Whether tags were left out.
    pub tags: bool,
}

/// A character of the text and whether it was escaped (so it never marks
/// formatting).
#[derive(Clone, Copy)]
struct Tok {
    c: char,
    literal: bool,
}

/// `raw` as plain text: `\x` becomes `x`; `@[T]` and `@<T>` become `T`;
/// `[text](url)` keeps its text (the first URL is returned); tags `#tag`
/// and `#(a tag)` go; `_x_`, `__x__`, `*x*` and `**x**` lose their
/// markers. The result is trimmed.
pub(super) fn clean(raw: &str) -> Cleaned {
    let chars: Vec<char> = raw.chars().collect();
    let mut out = Cleaned::default();
    let mut toks: Vec<Tok> = Vec::with_capacity(chars.len());
    let mut i = 0;
    while let Some(&c) = chars.get(i) {
        let next = chars.get(i + 1).copied();
        match c {
            '\\' => {
                toks.push(Tok {
                    c: next.unwrap_or('\\'),
                    literal: true,
                });
                i += 2;
            }
            '@' if matches!(next, Some('[' | '<')) => {
                let close = if next == Some('[') { ']' } else { '>' };
                match find(&chars, i + 2, close) {
                    Some(end) => {
                        push_literal(&mut toks, &unescape_chars(&chars[i + 2..end]));
                        i = end + 1;
                    }
                    None => {
                        toks.push(Tok { c, literal: false });
                        i += 1;
                    }
                }
            }
            '[' => match link_at(&chars, i) {
                Some((text_end, url_end)) => {
                    let text = &chars[i + 1..text_end];
                    toks.extend(text.iter().map(|&c| Tok { c, literal: false }));
                    let url: String = chars[text_end + 2..url_end].iter().collect();
                    out.link.get_or_insert(url.trim().to_owned());
                    i = url_end + 1;
                }
                None => {
                    toks.push(Tok { c, literal: false });
                    i += 1;
                }
            },
            '#' if i == 0 || chars[i - 1].is_whitespace() => {
                out.tags = true;
                i = if next == Some('(') {
                    find(&chars, i + 2, ')').map_or(chars.len(), |e| e + 1)
                } else {
                    let rest = chars[i..].iter().position(|c| c.is_whitespace());
                    rest.map_or(chars.len(), |n| i + n)
                };
                // The space before the tag goes with it.
                if toks.last().is_some_and(|t| t.c == ' ' && !t.literal) {
                    toks.pop();
                }
            }
            c => {
                toks.push(Tok { c, literal: false });
                i += 1;
            }
        }
    }
    strip_formatting(&mut toks);
    let text: String = toks.iter().map(|t| t.c).collect();
    out.text = text.trim().to_owned();
    out
}

/// `s` with each `\x` read as `x` (for titles).
pub(super) fn unescape(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    unescape_chars(&chars).trim().to_owned()
}

fn unescape_chars(chars: &[char]) -> String {
    let mut out = String::with_capacity(chars.len());
    let mut it = chars.iter();
    while let Some(&c) = it.next() {
        out.push(if c == '\\' {
            *it.next().unwrap_or(&c)
        } else {
            c
        });
    }
    out
}

fn push_literal(toks: &mut Vec<Tok>, s: &str) {
    toks.extend(s.chars().map(|c| Tok { c, literal: true }));
}

/// The index of the first unescaped `close` at or after `from`.
pub(super) fn find(chars: &[char], from: usize, close: char) -> Option<usize> {
    let mut i = from;
    while let Some(&c) = chars.get(i) {
        if c == '\\' {
            i += 2;
            continue;
        }
        if c == close {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// For a `[text](url)` at `start`: the index of `]` and of `)`.
pub(super) fn link_at(chars: &[char], start: usize) -> Option<(usize, usize)> {
    let text_end = find(chars, start + 1, ']')?;
    if chars.get(text_end + 1) != Some(&'(') {
        return None;
    }
    let url_end = find(chars, text_end + 2, ')')?;
    Some((text_end, url_end))
}

/// Remove paired `*`, `**`, `_` and `__` markers. A marker opens when a
/// non-space follows it and closes when a non-space precedes it; an
/// underscore must also sit at a word edge, so `snake_case` stays.
fn strip_formatting(toks: &mut Vec<Tok>) {
    // Runs of one marker character: (start, len).
    let mut runs: Vec<(usize, usize)> = Vec::new();
    let mut i = 0;
    while i < toks.len() {
        let t = toks[i];
        if !t.literal && (t.c == '*' || t.c == '_') {
            let len = toks[i..]
                .iter()
                .take_while(|u| !u.literal && u.c == t.c)
                .count();
            if len <= 2 {
                runs.push((i, len));
            }
            i += len;
        } else {
            i += 1;
        }
    }
    let at = |k: Option<usize>| k.and_then(|k| toks.get(k)).map(|t| t.c);
    let space = |c: Option<char>| c.is_none_or(char::is_whitespace);
    let word = |c: Option<char>| c.is_some_and(char::is_alphanumeric);
    let mut remove = vec![false; toks.len()];
    let mut open: Vec<(usize, usize)> = Vec::new();
    for &(start, len) in &runs {
        let c = toks[start].c;
        let before = at(start.checked_sub(1));
        let after = at(Some(start + len));
        let can_close = !space(before) && (c == '*' || !word(after));
        let can_open = !space(after) && (c == '*' || !word(before));
        let partner = open.iter().rposition(|&(s, l)| toks[s].c == c && l == len);
        match partner {
            Some(p) if can_close => {
                let (s, l) = open[p];
                open.truncate(p);
                remove[s..s + l].fill(true);
                remove[start..start + len].fill(true);
            }
            _ if can_open => open.push((start, len)),
            _ => {}
        }
    }
    let mut k = 0;
    toks.retain(|_| {
        k += 1;
        !remove[k - 1]
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_markup() {
        let c = clean(
            r"  A \[b\] _it_ **bold** snake_case @[Title] and @<Arg> #tag #(two words) \#no  ",
        );
        assert_eq!(c.text, "A [b] it bold snake_case Title and Arg #no");
        assert!(c.tags);
        let c = clean("see [the site](https://a.b) and [x](http://c.d)");
        assert_eq!(c.text, "see the site and x");
        assert_eq!(c.link.as_deref(), Some("https://a.b"));
        assert_eq!(clean(r"a \_b\_ 2 * 3 * 4").text, "a _b_ 2 * 3 * 4");
        assert_eq!(clean(r"end\").text, "end\\");
        assert_eq!(unescape(r"A \[x\]"), "A [x]");
    }
}
