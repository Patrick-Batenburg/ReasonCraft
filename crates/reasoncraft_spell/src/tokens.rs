//! Word tokenizer for spell checking.

use std::ops::Range;

fn is_connector(c: char) -> bool {
    matches!(c, '\'' | '’' | '-')
}

fn is_acronym(token: &str) -> bool {
    let letters = token.chars().count();
    letters <= 5 && token.chars().all(|c| c.is_uppercase())
}

/// Byte ranges of checkable words in `text`.
///
/// A word is a run of letters that may hold internal `'`, `’` or `-`. Chunks that look
/// like URLs or e-mail addresses, tokens with digits, single letters and short
/// all-caps acronyms are skipped.
pub fn words(text: &str) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut chunk_start = None;
    for (i, c) in text
        .char_indices()
        .chain(std::iter::once((text.len(), ' ')))
    {
        match (c.is_whitespace(), chunk_start) {
            (false, None) => chunk_start = Some(i),
            (true, Some(s)) => {
                chunk_words(text, s..i, &mut out);
                chunk_start = None;
            }
            _ => {}
        }
    }
    out
}

fn chunk_words(text: &str, chunk: Range<usize>, out: &mut Vec<Range<usize>>) {
    let s = &text[chunk.clone()];
    if s.contains("://") || s.contains('@') || s.to_lowercase().starts_with("www.") {
        return;
    }
    let chars: Vec<(usize, char)> = s.char_indices().collect();
    let mut i = 0;
    while i < chars.len() {
        if !chars[i].1.is_alphanumeric() {
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len()
            && (chars[i].1.is_alphanumeric()
                || (is_connector(chars[i].1)
                    && i > start
                    && chars.get(i + 1).is_some_and(|n| n.1.is_alphanumeric())))
        {
            i += 1;
        }
        let range =
            chunk.start + chars[start].0..chunk.start + chars.get(i).map_or(s.len(), |c| c.0);
        let token = &text[range.clone()];
        if token.chars().count() > 1 && !token.chars().any(|c| c.is_numeric()) && !is_acronym(token)
        {
            out.push(range);
        }
    }
}
