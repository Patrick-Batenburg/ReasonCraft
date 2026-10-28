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
    let persian =
        "https://fa.example.org/\u{0645}\u{06CC}\u{200C}\u{062E}\u{0648}\u{0627}\u{0647}\u{0645}";
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

/// An invisible mark in a `mailto:` parameter name cannot hide an
/// attachment, typed or escaped.
#[test]
fn mailto_attach_with_invisible_marks_refused() {
    for bad in [
        "mailto:x@y.z?attach\u{FEFF}=/etc/passwd",
        "mailto:x@y.z?attach\u{200B}=/etc/passwd",
        "mailto:x@y.z?\u{200B}attach=/x",
        "mailto:x@y.z?subject=a&attachment\u{FEFF}=/x",
        "mailto:x@y.z?attach%EF%BB%BF=/etc/passwd",
        "mailto:x@y.z?attach%E2%80%8B=/etc/passwd",
        "mailto:x@y.z?%E2%80%8Battach=/x",
    ] {
        assert_eq!(normalize_link(bad), Err(InvalidLink), "{bad:?}");
        assert_eq!(safe_link(bad), None, "{bad:?}");
    }
    // In a value the mark is only escaped.
    assert_eq!(
        safe_link("mailto:x@y.z?subject=a\u{200B}b").as_deref(),
        Some("mailto:x@y.z?subject=a%E2%80%8Bb")
    );
}

/// The default-ignorable code points that are not format characters
/// (Hangul fillers, the combining grapheme joiner, Khmer inherent vowels,
/// Mongolian variation selectors, unassigned ignorables) render as
/// nothing too: refused in the host, percent-encoded after it.
#[test]
fn default_ignorables_refused_in_host() {
    for c in [
        '\u{034F}',
        '\u{115F}',
        '\u{1160}',
        '\u{17B4}',
        '\u{17B5}',
        '\u{180B}',
        '\u{180C}',
        '\u{180D}',
        '\u{180F}',
        '\u{2065}',
        '\u{3164}',
        '\u{FFA0}',
        '\u{FFF0}',
        '\u{E0000}',
        '\u{E0080}',
        '\u{E01F0}',
        '\u{E0FFF}',
    ] {
        let code = format!("U+{:04X}", c as u32);
        for host in [
            format!("https://www.bank.example{c}.evil.example/login"),
            format!("https://exa{c}mple.org/x"),
            format!("exa{c}mple.org/x"),
            format!("mailto:a{c}b@example.org"),
        ] {
            assert_eq!(normalize_link(&host), Err(InvalidLink), "{code} {host:?}");
        }
        let mut escaped = String::new();
        for b in c.to_string().bytes() {
            escaped.push_str(&format!("%{b:02X}"));
        }
        assert_eq!(
            safe_link(&format!("https://example.org/a{c}b")),
            Some(format!("https://example.org/a{escaped}b")),
            "{code}"
        );
        assert_eq!(
            visible_link(&format!("https://example.org/a{c}b")),
            "https://example.org/a\u{FFFD}b"
        );
    }
    // Their visible neighbours stay.
    for fine in [
        "https://example.org/\u{1161}",
        "https://example.org/\u{3165}",
        "https://example.org/\u{FFA1}",
        "https://example.org/\u{17B6}",
        "https://example.org/\u{0350}",
    ] {
        assert_eq!(safe_link(fine).as_deref(), Some(fine), "{fine:?}");
    }
}

/// A user name before `@` in an http(s) link can make it read as another
/// site's: `https://www.bank.example@evil.example/` goes to evil.example.
#[test]
fn userinfo_refused_in_web_links() {
    for bad in [
        "https://www.bank.example@evil.example/login",
        "http://www.bank.example@evil.example",
        "HTTPS://user:pass@evil.example/",
        "https://@evil.example/",
        "www.bank.example@evil.example/login",
    ] {
        assert_eq!(normalize_link(bad), Err(InvalidLink), "{bad:?}");
        assert_eq!(safe_link(bad), None, "{bad:?}");
    }
    // After the host, `@` is just a character.
    for fine in [
        "https://example.org/@user",
        "https://example.org/a?mail=a@b.example",
        "https://example.org/#@x",
        "mailto:a@b.example",
    ] {
        assert_eq!(safe_link(fine).as_deref(), Some(fine), "{fine:?}");
    }
}

/// An escaped `?`, `&` or `#` in a `mailto:` address can turn into
/// parameters for a mail program that decodes before it splits.
#[test]
fn mailto_escaped_delimiters_in_address_refused() {
    for bad in [
        "mailto:a@b.example%3Fattach=C:/Users/x/secret",
        "mailto:a@b.example%3fattach=C:/Users/x/secret",
        "mailto:a@b.example%3Fsubject=x%26attach=C:/x",
        "mailto:a@b.example%26attach=C:/x",
        "mailto:a@b.example%23x",
        "mailto:a@b.example%3Fsubject=hi",
        "mailto:a@b.example?subject=x%26attach%3DC:/x",
        "mailto:a@b.example?subject=x%26%61ttach=C:/x",
    ] {
        assert_eq!(normalize_link(bad), Err(InvalidLink), "{bad:?}");
        assert_eq!(safe_link(bad), None, "{bad:?}");
    }
    for fine in [
        "mailto:a@b.example?subject=Q%26A",
        "mailto:a@b.example?subject=100%25&body=a%3Fb",
        "mailto:a%2Bb@b.example",
    ] {
        assert_eq!(safe_link(fine).as_deref(), Some(fine), "{fine:?}");
    }
}
