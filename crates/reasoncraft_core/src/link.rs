//! Links on boxes: which ones are safe to store and to open.

/// A link that is refused: not a safe `http`, `https` or `mailto` link.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidLink;

/// Control characters, and the invisible format characters that can make
/// a link read as another (bidi controls, zero-width marks, the BOM).
fn is_hidden_char(c: char) -> bool {
    c.is_control()
        || matches!(
            c,
            '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2060}'..='\u{2069}' | '\u{FEFF}'
        )
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
/// link. Anything else (other schemes, UNC paths and backslashes, spaces,
/// control or invisible format characters, `mailto:` links that attach a
/// file or hide escapes in a parameter name) is refused.
pub fn normalize_link(s: &str) -> Result<Option<String>, InvalidLink> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(None);
    }
    let bad = |c: char| c.is_whitespace() || is_hidden_char(c) || c == '\\';
    if s.chars().any(bad) {
        return Err(InvalidLink);
    }
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
