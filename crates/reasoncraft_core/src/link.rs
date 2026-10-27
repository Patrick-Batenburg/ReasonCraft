//! Links on boxes: which ones are safe to store and to open.

/// A link that is refused: not a safe `http`, `https` or `mailto` link.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidLink;

/// The invisible format characters (Unicode general category Cf, as of
/// Unicode 16, plus the unassigned U+2065 inside a Cf block): bidi
/// controls, zero-width marks, the soft hyphen, the BOM, Arabic and
/// Mongolian marks, interlinear annotations and tag characters. Any of
/// them can make a link read as another.
const FORMAT_CHARS: &[(char, char)] = &[
    ('\u{00AD}', '\u{00AD}'),
    ('\u{0600}', '\u{0605}'),
    ('\u{061C}', '\u{061C}'),
    ('\u{06DD}', '\u{06DD}'),
    ('\u{070F}', '\u{070F}'),
    ('\u{0890}', '\u{0891}'),
    ('\u{08E2}', '\u{08E2}'),
    ('\u{180E}', '\u{180E}'),
    ('\u{200B}', '\u{200F}'),
    ('\u{202A}', '\u{202E}'),
    ('\u{2060}', '\u{206F}'),
    ('\u{FEFF}', '\u{FEFF}'),
    ('\u{FFF9}', '\u{FFFB}'),
    ('\u{110BD}', '\u{110BD}'),
    ('\u{110CD}', '\u{110CD}'),
    ('\u{13430}', '\u{1343F}'),
    ('\u{1BCA0}', '\u{1BCA3}'),
    ('\u{1D173}', '\u{1D17A}'),
    ('\u{E0001}', '\u{E0001}'),
    ('\u{E0020}', '\u{E007F}'),
];

/// An invisible mark: a format character (see [`FORMAT_CHARS`]) or a
/// variation selector.
fn is_invisible_mark(c: char) -> bool {
    FORMAT_CHARS.iter().any(|&(lo, hi)| (lo..=hi).contains(&c))
        || matches!(c, '\u{FE00}'..='\u{FE0F}' | '\u{E0100}'..='\u{E01EF}')
}

/// Control characters and invisible marks.
fn is_hidden_char(c: char) -> bool {
    c.is_control() || is_invisible_mark(c)
}

/// Where the host of `s` (the address of a `mailto:` link) ends: at the
/// first `/`, `?` or `#` after the scheme (`?` or `#` for `mailto:`).
fn host_end(s: &str) -> usize {
    let lower = s.to_ascii_lowercase();
    let start = ["http://", "https://", "mailto:"]
        .iter()
        .find(|p| lower.starts_with(**p))
        .map_or(0, |p| p.len());
    let stops: &[char] = if lower.starts_with("mailto:") {
        &['?', '#']
    } else {
        &['/', '?', '#']
    };
    s[start..].find(stops).map_or(s.len(), |i| start + i)
}

/// `s` with the invisible marks after its host percent-encoded, so they
/// show as escapes and cannot reorder or hide what the link reads; in
/// the host itself they are refused. Words in scripts that need them
/// (the zero-width non-joiner of Persian, say) still work in a path.
fn encode_marks(s: &str) -> Result<String, InvalidLink> {
    let end = host_end(s);
    if s[..end].chars().any(is_invisible_mark) {
        return Err(InvalidLink);
    }
    let mut out = String::with_capacity(s.len());
    out.push_str(&s[..end]);
    for c in s[end..].chars() {
        if is_invisible_mark(c) {
            let mut buf = [0; 4];
            for b in c.encode_utf8(&mut buf).bytes() {
                out.push_str(&format!("%{b:02X}"));
            }
        } else {
            out.push(c);
        }
    }
    Ok(out)
}

/// `s` with `%XX` escapes decoded, ASCII-lowercased.
fn decoded_lower(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes
            .get(i + 1..i + 3)
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match (bytes[i], hex) {
            (b'%', Some(b)) => {
                out.push(b);
                i += 3;
            }
            (b, _) => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).to_ascii_lowercase()
}

/// Whether a `mailto:` link has a parameter a mail program could misuse:
/// one asking to attach a file (`attach` or `attachment`, in any case,
/// even percent-encoded), which could send local files along with a mail,
/// or one whose decoded name hides control characters or further `%`
/// escapes. The whole rest of the link counts, fragment included: mail
/// programs differ in where they think the parameters end.
fn mailto_unsafe_params(s: &str) -> bool {
    let Some((_, query)) = s.split_once('?') else {
        return false;
    };
    query.split(['&', ';']).any(|param| {
        let name = decoded_lower(param.split('=').next().unwrap_or_default());
        name.chars().any(|c| c.is_control() || c == '%')
            || matches!(name.trim(), "attach" | "attachment")
    })
}

/// Check a typed link. `http://`, `https://` and `mailto:` links are kept;
/// a bare domain such as `example.org/x` gets `https://`; empty means no
/// link. Invisible marks (format characters, variation selectors) after
/// the host are percent-encoded. Anything else (other schemes, UNC paths
/// and backslashes, spaces, control characters, invisible marks in the
/// host, `mailto:` links that attach a file or hide escapes in a
/// parameter name) is refused.
pub fn normalize_link(s: &str) -> Result<Option<String>, InvalidLink> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(None);
    }
    let bad = |c: char| c.is_whitespace() || c.is_control() || c == '\\';
    if s.chars().any(bad) {
        return Err(InvalidLink);
    }
    let encoded = encode_marks(s)?;
    let s = encoded.as_str();
    let lower = s.to_ascii_lowercase();
    for scheme in ["http://", "https://", "mailto:"] {
        if lower.starts_with(scheme) {
            // A host (or an address) must follow, not a path.
            let rest = &s[scheme.len()..];
            let host_first = rest
                .chars()
                .next()
                .is_some_and(|c| c.is_alphanumeric() || c == '[');
            if scheme == "mailto:" && mailto_unsafe_params(rest) {
                return Err(InvalidLink);
            }
            return if host_first {
                Ok(Some(s.to_owned()))
            } else {
                Err(InvalidLink)
            };
        }
    }
    let host_port = s.split(['/', '?', '#']).next().unwrap_or_default();
    let (host, port) = host_port.split_once(':').unwrap_or((host_port, ""));
    let host_ok = host.contains('.')
        && !host.starts_with('.')
        && !host.ends_with('.')
        && host
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '.');
    let port_ok = !host_port.contains(':') || (!port.is_empty() && port.parse::<u16>().is_ok());
    if host_ok && port_ok {
        Ok(Some(format!("https://{s}")))
    } else {
        Err(InvalidLink)
    }
}

/// The URL to hand to the browser for a stored link, or `None` when it is
/// not a safe `http`, `https` or `mailto` link. A file may have been written
/// by anyone, so every open goes through this check.
pub fn safe_link(s: &str) -> Option<String> {
    normalize_link(s).ok().flatten()
}

/// `s` with control and invisible format characters shown as U+FFFD, for
/// displaying a stored link that may contain them.
pub fn visible_link(s: &str) -> String {
    s.chars()
        .map(|c| if is_hidden_char(c) { '\u{FFFD}' } else { c })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every invisible format character (Unicode category Cf) and
    /// variation selector: bidi controls, zero-width marks, the soft
    /// hyphen, Arabic and Mongolian marks, interlinear annotations and tag
    /// characters. Refused in the host, percent-encoded after it.
    #[test]
    fn cf_characters_refused_in_links() {
        for c in [
            '\u{202E}',
            '\u{00AD}',
            '\u{0600}',
            '\u{061C}',
            '\u{06DD}',
            '\u{070F}',
            '\u{0890}',
            '\u{08E2}',
            '\u{180E}',
            '\u{200B}',
            '\u{2064}',
            '\u{206A}',
            '\u{FEFF}',
            '\u{FFF9}',
            '\u{110BD}',
            '\u{13430}',
            '\u{1BCA0}',
            '\u{1D173}',
            '\u{E0001}',
            '\u{E0041}',
            '\u{FE0F}',
            '\u{E0100}',
        ] {
            let code = format!("U+{:04X}", c as u32);
            for host in [
                format!("https://exa{c}mple.org/x"),
                format!("exa{c}mple.org/x"),
                format!("mailto:a{c}b@example.org"),
                format!("{c}https://example.org"),
            ] {
                assert_eq!(normalize_link(&host), Err(InvalidLink), "{code} {host:?}");
                assert_eq!(safe_link(&host), None, "{code}");
            }
            let mut escaped = String::new();
            for b in c.to_string().bytes() {
                escaped.push_str(&format!("%{b:02X}"));
            }
            for (url, want) in [
                ("https://example.org/a{c}b", "https://example.org/a{e}b"),
                (
                    "example.org/a?q={c}#f{c}",
                    "https://example.org/a?q={e}#f{e}",
                ),
                (
                    "mailto:a@example.org?subject={c}",
                    "mailto:a@example.org?subject={e}",
                ),
            ] {
                let url = url.replace("{c}", &c.to_string());
                let want = want.replace("{e}", &escaped);
                assert_eq!(safe_link(&url).as_deref(), Some(want.as_str()), "{code}");
            }
            let url = format!("https://example.org/a{c}b");
            assert_eq!(visible_link(&url), "https://example.org/a\u{FFFD}b");
        }
        // A Persian word with a zero-width non-joiner keeps working in a
        // path; a bidi override in the host is refused.
        let persian = "https://fa.example.org/\u{0645}\u{06CC}\u{200C}\u{062E}\u{0648}\u{0627}\u{0647}\u{0645}";
        assert_eq!(
            safe_link(persian).as_deref(),
            Some(
                "https://fa.example.org/\u{0645}\u{06CC}%E2%80%8C\u{062E}\u{0648}\u{0627}\u{0647}\u{0645}"
            )
        );
        assert_eq!(safe_link("https://evil\u{202E}gro.example/"), None);
        // Visible text in other scripts is fine.
        let fine = "https://example.org/caf\u{E9}/\u{0645}\u{0631}\u{062D}\u{0628}\u{0627}";
        assert_eq!(safe_link(fine), Some(fine.to_owned()));
    }
}
