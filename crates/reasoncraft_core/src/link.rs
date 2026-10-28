//! Links on boxes: which ones are safe to store and to open.

/// A link that is refused: not a safe `http`, `https` or `mailto` link.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidLink;

/// The invisible marks, as of Unicode 16: every format character
/// (general category Cf) and every `Default_Ignorable_Code_Point`
/// (Unicode's `DerivedCoreProperties.txt`), which includes the code
/// points reserved as ignorable. Bidi controls, zero-width marks, the
/// soft hyphen, the BOM, Arabic and Mongolian marks, the combining
/// grapheme joiner, Hangul fillers, Khmer inherent vowels, interlinear
/// annotations, tag characters and variation selectors. They render as
/// nothing (or are meant to), so any of them can make a link read as
/// another. Cf = format character, DI = default ignorable.
const INVISIBLE_MARKS: &[(char, char)] = &[
    ('\u{00AD}', '\u{00AD}'),   // Cf, DI: soft hyphen
    ('\u{034F}', '\u{034F}'),   // DI: combining grapheme joiner
    ('\u{0600}', '\u{0605}'),   // Cf: Arabic number signs
    ('\u{061C}', '\u{061C}'),   // Cf, DI: Arabic letter mark
    ('\u{06DD}', '\u{06DD}'),   // Cf: Arabic end of ayah
    ('\u{070F}', '\u{070F}'),   // Cf: Syriac abbreviation mark
    ('\u{0890}', '\u{0891}'),   // Cf: Arabic pound and piastre marks
    ('\u{08E2}', '\u{08E2}'),   // Cf: Arabic disputed end of ayah
    ('\u{115F}', '\u{1160}'),   // DI: Hangul choseong and jungseong fillers
    ('\u{17B4}', '\u{17B5}'),   // DI: Khmer inherent vowels
    ('\u{180B}', '\u{180F}'),   // Cf, DI: Mongolian variation selectors, vowel separator
    ('\u{200B}', '\u{200F}'),   // Cf, DI: zero-width marks, LRM, RLM
    ('\u{202A}', '\u{202E}'),   // Cf, DI: bidi embeddings and overrides
    ('\u{2060}', '\u{206F}'), // Cf, DI: word joiner, invisible operators, isolates (U+2065 reserved)
    ('\u{3164}', '\u{3164}'), // DI: Hangul filler
    ('\u{FE00}', '\u{FE0F}'), // DI: variation selectors
    ('\u{FEFF}', '\u{FEFF}'), // Cf, DI: byte order mark
    ('\u{FFA0}', '\u{FFA0}'), // DI: halfwidth Hangul filler
    ('\u{FFF0}', '\u{FFFB}'), // DI: reserved; Cf: interlinear annotations
    ('\u{110BD}', '\u{110BD}'), // Cf: Kaithi number sign
    ('\u{110CD}', '\u{110CD}'), // Cf: Kaithi number sign above
    ('\u{13430}', '\u{1343F}'), // Cf: Egyptian hieroglyph format controls
    ('\u{1BCA0}', '\u{1BCA3}'), // Cf, DI: shorthand format controls
    ('\u{1D173}', '\u{1D17A}'), // Cf, DI: musical beam and slur controls
    ('\u{E0000}', '\u{E0FFF}'), // Cf, DI: tags, variation selectors 17-256, reserved
];

/// An invisible mark (see [`INVISIBLE_MARKS`]).
fn is_invisible_mark(c: char) -> bool {
    INVISIBLE_MARKS
        .iter()
        .any(|&(lo, hi)| (lo..=hi).contains(&c))
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
/// or one whose decoded name hides control characters, invisible marks
/// (typed, or escaped by [`normalize_link`] or by hand) or further `%`
/// escapes. The whole rest of the link counts, fragment included: mail
/// programs differ in where they think the parameters end.
fn mailto_unsafe_params(s: &str) -> bool {
    let Some((_, query)) = s.split_once('?') else {
        return false;
    };
    query.split(['&', ';']).any(|param| {
        let name = decoded_lower(param.split('=').next().unwrap_or_default());
        name.chars().any(|c| is_hidden_char(c) || c == '%')
            || matches!(name.trim(), "attach" | "attachment")
    })
}

/// Whether a `mailto:` link, `rest` after the scheme with `address` up to
/// its `?` or `#`, could make a mail program misuse it: an escaped `?`,
/// `&` or `#` in the address (a program that decodes before it splits
/// reads parameters there), or an unsafe parameter (see
/// [`mailto_unsafe_params`]) before or after decoding.
fn mailto_unsafe(address: &str, rest: &str) -> bool {
    let address = address.to_ascii_lowercase();
    ["%3f", "%26", "%23"].iter().any(|e| address.contains(e))
        || mailto_unsafe_params(rest)
        || mailto_unsafe_params(&decoded_lower(rest))
}

/// Check a typed link. `http://`, `https://` and `mailto:` links are kept;
/// a bare domain such as `example.org/x` gets `https://`; empty means no
/// link. Invisible marks (see [`INVISIBLE_MARKS`]) after the host are
/// percent-encoded. Anything else (other schemes, UNC paths and
/// backslashes, spaces, control characters, invisible marks in the host,
/// a user name (`user@`) before an http(s) host, `mailto:` links with an
/// escaped `?`, `&` or `#` in the address, that attach a file or hide
/// escapes in a parameter name) is refused.
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
            let authority = &s[scheme.len()..host_end(s)];
            let unsafe_part = if scheme == "mailto:" {
                mailto_unsafe(authority, rest)
            } else {
                // A user name: `https://www.bank.example@evil.example/`
                // goes to evil.example.
                authority.contains('@')
            };
            if unsafe_part {
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
mod tests;
