//! What the writer cannot set in a Word file, patched into the finished
//! package: the author in the core properties (`docProps/core.xml`; the
//! writer only knows a fixed placeholder) and the document language in the
//! default run properties of `word/styles.xml` (the writer has no language
//! setting), so spelling and hyphenation use the document's language.

use std::io::{Cursor, Read, Write};
use std::time::{SystemTime, UNIX_EPOCH};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use super::ExportError;

const CORE: &str = "docProps/core.xml";
const STYLES: &str = "word/styles.xml";
/// What the writer puts where the author goes.
const PLACEHOLDER: &str = "unknown";

/// Now in UTC, as W3CDTF (`2026-10-25T14:03:09Z`).
pub(super) fn now_w3cdtf() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    w3cdtf(secs)
}

/// `secs` after the Unix epoch as W3CDTF, in UTC.
fn w3cdtf(secs: u64) -> String {
    let (days, rest) = (secs / 86_400, secs % 86_400);
    let (y, m, d) = civil_from_days(days);
    let (hh, mm, ss) = (rest / 3600, rest / 60 % 60, rest % 60);
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

/// The date `days` after 1970-01-01 (Howard Hinnant's algorithm).
fn civil_from_days(days: u64) -> (u64, u64, u64) {
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z % 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + u64::from(m <= 2);
    (y, m, d)
}

fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c => out.push(c),
        }
    }
    out
}

/// `core` with `author` as creator and last editor.
fn with_author(core: &str, author: &str) -> String {
    let author = escape(author);
    let mut out = core.to_owned();
    for tag in ["dc:creator", "cp:lastModifiedBy"] {
        out = out.replace(
            &format!("<{tag}>{PLACEHOLDER}</{tag}>"),
            &format!("<{tag}>{author}</{tag}>"),
        );
    }
    out
}

/// `styles` with `tag` (`nl-NL`) as the language of the default run
/// properties (which [`super::docx`] always fills with the font and
/// size); unchanged when they cannot be found.
fn with_language(styles: &str, tag: &str) -> String {
    let close = styles.find("<w:rPrDefault>").and_then(|start| {
        let end = start + styles[start..].find("</w:rPrDefault>")?;
        Some(start + styles[start..end].rfind("</w:rPr>")?)
    });
    let Some(close) = close else {
        return styles.to_owned();
    };
    // `lang` comes late in a run's properties: after the fonts and size.
    let lang = format!("<w:lang w:val=\"{}\" />", escape(tag));
    [&styles[..close], &lang, &styles[close..]].concat()
}

fn zip_error(e: impl std::fmt::Display) -> ExportError {
    ExportError::Render(e.to_string())
}

/// The package `docx` with `author` (already on one line, control
/// characters removed) as its creator and last editor, unless it is
/// empty, and `lang_tag` (`en-GB`) as the language of its text. Every
/// other entry is copied as it is.
pub(super) fn finish_package(
    docx: Vec<u8>,
    author: &str,
    lang_tag: &str,
) -> Result<Vec<u8>, ExportError> {
    let mut src = ZipArchive::new(Cursor::new(docx)).map_err(zip_error)?;
    let mut out = ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..src.len() {
        let mut file = src.by_index(i).map_err(zip_error)?;
        let name = file.name().map_err(zip_error)?.into_owned();
        if !(name == STYLES || name == CORE && !author.is_empty()) {
            out.raw_copy_file(file).map_err(zip_error)?;
            continue;
        }
        let mut xml = String::new();
        file.read_to_string(&mut xml)?;
        drop(file);
        let xml = if name == STYLES {
            with_language(&xml, lang_tag)
        } else {
            with_author(&xml, author)
        };
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        out.start_file(name.as_str(), options).map_err(zip_error)?;
        out.write_all(xml.as_bytes())?;
    }
    Ok(out.finish().map_err(zip_error)?.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_are_utc_w3cdtf() {
        assert_eq!(w3cdtf(0), "1970-01-01T00:00:00Z");
        assert_eq!(w3cdtf(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(w3cdtf(1_792_930_989), "2026-10-25T12:23:09Z");
    }

    #[test]
    fn author_is_escaped() {
        let core = "<dc:creator>unknown</dc:creator><cp:lastModifiedBy>unknown</cp:lastModifiedBy>";
        assert_eq!(
            with_author(core, "A&<B>"),
            "<dc:creator>A&amp;&lt;B&gt;</dc:creator>\
             <cp:lastModifiedBy>A&amp;&lt;B&gt;</cp:lastModifiedBy>"
        );
    }
}
