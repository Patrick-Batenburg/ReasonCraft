//! Argdown data `{…}`: a small YAML-flow subset of strings, booleans,
//! lists and maps. Anything else (numbers, nulls) is kept as a string.

use super::report::MAX_NESTING;

/// A data value.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Value {
    Str(String),
    Bool(bool),
    List(Vec<Value>),
    Map(Data),
}

/// A data map, keys in the order written.
pub(super) type Data = Vec<(String, Value)>;

impl Value {
    /// The value as text: a string, or `true`/`false`.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            Value::Bool(true) => Some("true"),
            Value::Bool(false) => Some("false"),
            Value::List(_) | Value::Map(_) => None,
        }
    }

    pub fn is_true(&self) -> bool {
        self.as_str()
            .is_some_and(|s| s.trim().eq_ignore_ascii_case("true"))
    }
}

/// The value of `key` in `data`.
pub(super) fn get<'a>(data: &'a Data, key: &str) -> Option<&'a Value> {
    data.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

/// The map starting at `chars[start]` (a `{`) and the index just after
/// its closing `}` (or the end, when it is missing).
pub(super) fn parse_map(chars: &[char], start: usize) -> (Data, usize) {
    let mut p = Parser { c: chars, i: start };
    let data = p.map(0);
    (data, p.i)
}

/// A YAML scalar as written in frontmatter: quoted or plain.
pub(super) fn scalar(s: &str) -> String {
    let chars: Vec<char> = s.trim().chars().collect();
    let mut p = Parser { c: &chars, i: 0 };
    match p.peek() {
        Some('"') => p.double_quoted(),
        Some('\'') => p.single_quoted(),
        _ => s.trim().to_owned(),
    }
}

struct Parser<'a> {
    c: &'a [char],
    i: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<char> {
        self.c.get(self.i).copied()
    }

    fn skip_ws(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.i += 1;
        }
    }

    /// A map; `self.i` is at its `{`.
    fn map(&mut self, depth: usize) -> Data {
        let mut out = Vec::new();
        self.i += 1;
        loop {
            self.skip_ws();
            let before = self.i;
            match self.peek() {
                None => break,
                Some('}') => {
                    self.i += 1;
                    break;
                }
                Some(',') => {
                    self.i += 1;
                    continue;
                }
                _ => {}
            }
            let key = match self.peek() {
                Some('"') => self.double_quoted(),
                Some('\'') => self.single_quoted(),
                _ => self.plain(&[':', ',', '}']),
            };
            self.skip_ws();
            let value = if self.peek() == Some(':') {
                self.i += 1;
                self.value(depth + 1)
            } else {
                Value::Str(String::new())
            };
            out.push((key, value));
            self.skip_ws();
            if self.i == before {
                self.i += 1;
            }
        }
        out
    }

    fn list(&mut self, depth: usize) -> Vec<Value> {
        let mut out = Vec::new();
        self.i += 1;
        loop {
            self.skip_ws();
            let before = self.i;
            match self.peek() {
                None => break,
                Some(']') => {
                    self.i += 1;
                    break;
                }
                Some(',') => {
                    self.i += 1;
                    continue;
                }
                _ => out.push(self.value(depth + 1)),
            }
            if self.i == before {
                self.i += 1;
            }
        }
        out
    }

    fn value(&mut self, depth: usize) -> Value {
        self.skip_ws();
        match self.peek() {
            Some('{' | '[') if depth >= MAX_NESTING => Value::Str(self.raw_nested()),
            Some('{') => Value::Map(self.map(depth)),
            Some('[') => Value::List(self.list(depth)),
            Some('"') => Value::Str(self.double_quoted()),
            Some('\'') => Value::Str(self.single_quoted()),
            _ => {
                let s = self.plain(&[',', '}', ']']);
                match s.as_str() {
                    "true" | "True" | "TRUE" => Value::Bool(true),
                    "false" | "False" | "FALSE" => Value::Bool(false),
                    _ => Value::Str(s),
                }
            }
        }
    }

    /// A plain scalar up to one of `stops`, trimmed.
    fn plain(&mut self, stops: &[char]) -> String {
        let start = self.i;
        while self.peek().is_some_and(|c| !stops.contains(&c)) {
            self.i += 1;
        }
        let s: String = self.c[start..self.i].iter().collect();
        s.trim().to_owned()
    }

    /// Too deep to read: the balanced text as written.
    fn raw_nested(&mut self) -> String {
        let start = self.i;
        let mut depth = 0usize;
        while let Some(c) = self.peek() {
            self.i += 1;
            match c {
                '{' | '[' => depth += 1,
                '}' | ']' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
        }
        self.c[start..self.i].iter().collect()
    }

    /// A `"…"` string with YAML escapes; `self.i` is at the quote.
    fn double_quoted(&mut self) -> String {
        let mut out = String::new();
        self.i += 1;
        while let Some(c) = self.peek() {
            self.i += 1;
            match c {
                '"' => break,
                '\\' => {
                    let Some(e) = self.peek() else { break };
                    self.i += 1;
                    match e {
                        'n' => out.push('\n'),
                        'r' => out.push('\r'),
                        't' => out.push('\t'),
                        '0' => out.push('\0'),
                        'x' => out.push(self.hex(2)),
                        'u' => out.push(self.hex(4)),
                        'U' => out.push(self.hex(8)),
                        other => out.push(other),
                    }
                }
                c => out.push(c),
            }
        }
        out
    }

    /// The character of the next `n` hex digits (U+FFFD when invalid).
    fn hex(&mut self, n: usize) -> char {
        let end = (self.i + n).min(self.c.len());
        let digits: String = self.c[self.i..end].iter().collect();
        self.i = end;
        u32::from_str_radix(&digits, 16)
            .ok()
            .and_then(char::from_u32)
            .unwrap_or('\u{FFFD}')
    }

    /// A `'…'` string (`''` is a quote); `self.i` is at the quote.
    fn single_quoted(&mut self) -> String {
        let mut out = String::new();
        self.i += 1;
        while let Some(c) = self.peek() {
            self.i += 1;
            if c == '\'' {
                if self.peek() == Some('\'') {
                    self.i += 1;
                } else {
                    break;
                }
            }
            out.push(c);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(s: &str) -> Data {
        let chars: Vec<char> = s.chars().collect();
        parse_map(&chars, 0).0
    }

    #[test]
    fn reads_flow_values() {
        let d =
            map(r#"{kind: reason, n: 3, ok: true, s: "a\"b}c\nd", l: ["x", y], m: {a: 'it''s'}}"#);
        assert_eq!(get(&d, "kind"), Some(&Value::Str("reason".into())));
        assert_eq!(get(&d, "n"), Some(&Value::Str("3".into())));
        assert!(get(&d, "ok").unwrap().is_true());
        assert_eq!(get(&d, "s"), Some(&Value::Str("a\"b}c\nd".into())));
        let l = Value::List(vec![Value::Str("x".into()), Value::Str("y".into())]);
        assert_eq!(get(&d, "l"), Some(&l));
        let m = Value::Map(vec![("a".into(), Value::Str("it's".into()))]);
        assert_eq!(get(&d, "m"), Some(&m));
        assert_eq!(scalar(r#""A === B""#), "A === B");
    }

    #[test]
    fn survives_garbage_and_depth() {
        for s in ["{", "{:", "{a: [", "{a: \"x", "{,,:,}", "{a: {b: [}]}}"] {
            map(s);
        }
        let deep = format!("{{a: {}{}}}", "[".repeat(500), "]".repeat(500));
        assert!(matches!(get(&map(&deep), "a"), Some(Value::List(_))));
    }
}
