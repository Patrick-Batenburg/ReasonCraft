//! Finding commands by what the user types.

use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

use super::{Command, all};
use crate::i18n::t;

/// The commands whose translated label and group label contain every word
/// of `query`, at most `limit` of them. Matching ignores case and
/// diacritics ("cafe" finds "Café"). Commands with more words matched at
/// the start of a word come first, then registry order. A blank query
/// finds nothing.
pub fn search(query: &str, limit: usize) -> Vec<&'static Command> {
    let words: Vec<String> = fold(query).split_whitespace().map(str::to_owned).collect();
    if words.is_empty() {
        return Vec::new();
    }
    let mut hits: Vec<(usize, &'static Command)> = all()
        .iter()
        .filter_map(|cmd| {
            let text = fold(&format!("{} {}", t(cmd.label), t(cmd.group)));
            let mut prefix_hits = 0;
            for word in &words {
                let mut found = text.match_indices(word.as_str()).peekable();
                found.peek()?;
                if found.any(|(i, _)| starts_word(&text, i)) {
                    prefix_hits += 1;
                }
            }
            Some((prefix_hits, cmd))
        })
        .collect();
    // Stable: equal scores keep registry order.
    hits.sort_by_key(|&(prefix_hits, _)| std::cmp::Reverse(prefix_hits));
    hits.into_iter().take(limit).map(|(_, cmd)| cmd).collect()
}

/// Lowercase, without the combining marks of the canonical decomposition.
fn fold(s: &str) -> String {
    s.nfd()
        .filter(|&c| !is_combining_mark(c))
        .flat_map(char::to_lowercase)
        .collect()
}

/// Whether byte `i` of `text` begins a word.
fn starts_word(text: &str, i: usize) -> bool {
    text[..i]
        .chars()
        .next_back()
        .is_none_or(|c| !c.is_alphanumeric())
}
