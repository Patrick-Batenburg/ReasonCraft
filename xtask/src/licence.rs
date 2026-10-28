//! `cargo xtask licence-rtf <out.rtf>`: the Windows installer's licence page.
//!
//! Adapted from WordCraft (https://github.com/storytold/wordcraft, packaging/windows/package.ps1's
//! `ConvertTo-RtfText` at aadb077), MIT OR Apache-2.0, used under Apache-2.0; changed: ported to
//! Rust, ReasonCraft's title and sentence.

use std::path::Path;

/// The RTF header: Tahoma 8 pt, 6 pt after each paragraph.
pub const HEADER: &str = "{\\rtf1\\ansi\\ansicpg1252\\deff0{\\fonttbl{\\f0\\fswiss\\fcharset0 Tahoma;}}\\viewkind4\\uc1\\pard\\sa120\\f0\\fs16 ";

/// `text` as 7-bit RTF: `\`, `{` and `}` escaped, everything above ASCII as `\uN?` per UTF-16
/// unit (N signed, as RTF wants).
pub fn rtf_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for unit in text.encode_utf16() {
        match unit {
            0x5C | 0x7B | 0x7D => {
                out.push('\\');
                out.push(char::from(unit as u8));
            }
            0..=127 => out.push(char::from(unit as u8)),
            _ => out.push_str(&format!("\\u{}?", unit as i16)),
        }
    }
    out
}

/// The licence page: a bold title, one sentence, then both licence texts with their hard-wrapped
/// lines joined into paragraphs (so they wrap to the dialog's width) and each title in bold.
pub fn rtf(mit: &str, apache: &str) -> String {
    let mut paragraphs = vec![
        "\\b ReasonCraft\\b0".to_string(),
        rtf_escape(
            "ReasonCraft is licensed under the MIT License or the Apache License, Version 2.0, \
             at your option. Both licence texts follow. Bundled third-party material keeps its \
             own licence, listed in THIRD-PARTY.md.",
        ),
    ];
    for text in [mit, apache] {
        let mut title = true;
        // A blank line (or one of only spaces) ends a paragraph.
        for para in text
            .lines()
            .map(str::trim)
            .collect::<Vec<_>>()
            .split(|l| l.is_empty())
        {
            if para.is_empty() {
                continue;
            }
            let escaped = rtf_escape(&para.join(" "));
            paragraphs.push(if title {
                format!("\\b {escaped}\\b0")
            } else {
                escaped
            });
            title = false;
        }
    }
    format!("{HEADER}{}\\par\r\n}}\r\n", paragraphs.join("\\par\r\n"))
}

/// Writes the licence page built from the workspace's `LICENSE-MIT` and `LICENSE-APACHE`.
pub fn write_rtf(root: &Path, out: &Path) -> Result<(), String> {
    let read = |name: &str| {
        std::fs::read_to_string(root.join(name)).map_err(|e| format!("read {name}: {e}"))
    };
    let text = rtf(&read("LICENSE-MIT")?, &read("LICENSE-APACHE")?);
    if let Some(dir) = out.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    }
    std::fs::write(out, text).map_err(|e| format!("write {}: {e}", out.display()))?;
    println!("wrote {}", out.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rtf_escapes_braces_backslashes_and_unicode() {
        assert_eq!(rtf_escape("a{b}\\ é€"), "a\\{b\\}\\\\ \\u233?\\u8364?");
        // Above 32767, RTF's \u takes a signed 16-bit number.
        assert_eq!(rtf_escape("\u{FFFD}"), "\\u-3?");
        // Outside the BMP: two UTF-16 surrogates, both negative.
        assert_eq!(rtf_escape("😀"), "\\u-10179?\\u-8704?");
    }

    /// The text's unescaped `{` and `}`, skipping control symbols.
    fn bare_braces(text: &str) -> usize {
        let mut chars = text.chars();
        let mut count = 0;
        while let Some(c) = chars.next() {
            match c {
                '\\' => {
                    chars.next();
                }
                '{' | '}' => count += 1,
                _ => {}
            }
        }
        count
    }

    #[test]
    fn rtf_holds_both_licences() {
        let root = crate::workspace_dir();
        let mit = std::fs::read_to_string(root.join("LICENSE-MIT")).unwrap();
        let apache = std::fs::read_to_string(root.join("LICENSE-APACHE")).unwrap();
        let out = rtf(&mit, &apache);
        assert!(out.starts_with("{\\rtf1\\ansi"), "{}", &out[..40]);
        assert!(out.contains("\\b MIT License\\b0"));
        assert!(out.contains("Apache License"));
        assert!(out.contains("\\b ReasonCraft\\b0"));
        assert!(out.is_ascii(), "RTF is written as 7-bit ASCII");
        let body = out
            .strip_prefix(HEADER)
            .and_then(|b| b.trim_end().strip_suffix('}'))
            .expect("the header, the paragraphs, the closing brace");
        for para in body.split("\\par") {
            assert_eq!(bare_braces(para), 0, "a bare brace in {para:?}");
        }
    }
}
