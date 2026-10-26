//! The spelling pass (F7): the misspellings of box text, sources and notes
//! one at a time, in find order from the selected box (wrapping around),
//! each with Change, Change all, Ignore, Ignore all and Add to dictionary.

use reasoncraft_core::find::search_order;
use reasoncraft_core::{Document, EditError, FindHit, FindOptions, Item, NodeId, find_all};

use super::SpellService;
use crate::actions::{Action, cap_box};
use crate::app::ReasonCraftApp;
use crate::doc_tab::DocTab;
use crate::i18n::t;

/// How many suggestions the panel offers.
pub const MAX_SUGGESTIONS: usize = 6;
/// How many characters of context show on each side of the word.
pub const CONTEXT_CHARS: usize = 30;

/// One tab's spelling pass.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpellPass {
    /// The box the pass started at.
    start: Option<NodeId>,
    /// Where to look next: the box's place in the walk from `start`, and a
    /// byte offset in its text.
    at: (usize, usize),
    /// The misspelling on show (its box is selected), and its word.
    pub current: Option<FindHit>,
    pub word: String,
    pub suggestions: Vec<String>,
    /// What Change puts in: the first suggestion until the user edits it.
    pub replacement: String,
    /// Every box has been looked at: "Spelling check complete".
    pub done: bool,
}

impl SpellPass {
    /// A pass from `selection` (from the top with nothing selected).
    fn new(doc: &Document, selection: Option<Item>) -> Self {
        let start = match selection {
            Some(Item::Box(b)) => Some(b),
            Some(Item::Card(c)) => doc.cards.get(&c).and_then(|c| c.premises.first().copied()),
            None => None,
        };
        Self {
            start,
            ..Self::default()
        }
    }

    /// Whether the pass waits for its first (or next) misspelling: the
    /// dictionary is still loading.
    pub fn waiting(&self) -> bool {
        self.current.is_none() && !self.done
    }
}

/// The boxes in find order, starting at `start` and wrapping around.
fn walk(doc: &Document, start: Option<NodeId>) -> Vec<NodeId> {
    let mut order = search_order(doc);
    if let Some(i) = start.and_then(|s| order.iter().position(|&n| n == s)) {
        order.rotate_left(i);
    }
    order
}

/// Move `tab`'s pass to the next misspelling from where it is, selecting
/// and revealing its box; or finish it. While the dictionary loads it
/// waits (see [`super::poll`]).
pub(super) fn advance(tab: &mut DocTab, service: &mut SpellService) {
    let Some(mut pass) = tab.spelling.take() else {
        return;
    };
    let lang = tab.doc.meta.language.clone();
    let order = walk(&tab.doc, pass.start);
    pass.current = None;
    let (mut i, mut from) = pass.at;
    while i < order.len() {
        let node = order[i];
        let Some(text) = tab.doc.boxes.get(&node).map(|b| &b.text) else {
            (i, from) = (i + 1, 0);
            continue;
        };
        let Some(ranges) = service.misspelled(&lang, text) else {
            tab.spelling = Some(pass);
            return;
        };
        if let Some(r) = ranges.iter().find(|r| r.start >= from) {
            pass.at = (i, r.start);
            pass.word = text[r.clone()].to_owned();
            pass.suggestions = service.suggest(&lang, &pass.word, MAX_SUGGESTIONS);
            pass.replacement = pass.suggestions.first().unwrap_or(&pass.word).clone();
            pass.current = Some(FindHit {
                node,
                range: r.clone(),
            });
            tab.selection = Some(Item::Box(node));
            tab.reveal_pending = true;
            tab.spelling = Some(pass);
            return;
        }
        (i, from) = (i + 1, 0);
    }
    pass.at = (order.len(), 0);
    pass.done = true;
    tab.spelling = Some(pass);
}

/// The current misspelling, if `hit` still is it and its box still holds
/// the word there.
fn current(tab: &DocTab, hit: &FindHit) -> Option<String> {
    let pass = tab.spelling.as_ref()?;
    let text = &tab.doc.boxes.get(&hit.node)?.text;
    (pass.current.as_ref() == Some(hit) && text.get(hit.range.clone()) == Some(pass.word.as_str()))
        .then(|| pass.word.clone())
}

/// Look on from byte `offset` of the current box.
fn resume_at(tab: &mut DocTab, service: &mut SpellService, offset: usize) {
    if let Some(pass) = &mut tab.spelling {
        pass.at.1 = offset;
    }
    advance(tab, service);
}

/// Carry out a spelling action in the active tab.
pub(crate) fn apply(app: &mut ReasonCraftApp, action: Action) {
    let ReasonCraftApp {
        tabs,
        active,
        spell,
        settings,
        status_note,
        right_panel,
        ..
    } = app;
    let Some(tab) = tabs.get_mut(*active) else {
        return;
    };
    let end = tab
        .spelling
        .as_ref()
        .and_then(|p| p.current.as_ref())
        .map(|h| h.range.end);
    match action {
        Action::OpenSpelling => {
            if settings.spellcheck {
                *right_panel = true;
                tab.spelling = Some(SpellPass::new(&tab.doc, tab.selection));
                advance(tab, spell);
            }
        }
        Action::CloseSpelling => tab.spelling = None,
        Action::SpellIgnore => {
            if let Some(end) = end {
                resume_at(tab, spell, end);
            }
        }
        Action::SpellIgnoreAll(word) => {
            spell.ignore_all(&word);
            resume_at(tab, spell, end.unwrap_or(0));
        }
        Action::SpellAddWord(word) => {
            if spell.add_word(&word) {
                let word = word.trim().to_owned();
                if !settings.spell_words.contains(&word) {
                    settings.spell_words.push(word);
                }
            } else {
                *status_note = Some(t("spell.add_refused"));
            }
            resume_at(tab, spell, end.unwrap_or(0));
        }
        Action::SpellChange { hit, with } => {
            let Some(word) = current(tab, &hit) else {
                return advance(tab, spell);
            };
            if word != with {
                let done = tab.edit(|d| {
                    d.replace_range(hit.node, hit.range.clone(), &with)?;
                    Ok(cap_box(d, hit.node))
                });
                if done == Some(true) {
                    *status_note = Some(t("status.text_cut"));
                }
            }
            resume_at(tab, spell, hit.range.start + with.len());
        }
        Action::SpellChangeAll { word, with } => {
            let Some(hit) = tab.spelling.as_ref().and_then(|p| p.current.clone()) else {
                return;
            };
            let opts = FindOptions {
                match_case: true,
                whole_word: true,
            };
            let hits = find_all(&tab.doc, &word, opts);
            let before = hits
                .iter()
                .filter(|h| h.node == hit.node && h.range.start < hit.range.start)
                .count();
            let mut nodes: Vec<NodeId> = hits.iter().map(|h| h.node).collect();
            nodes.dedup();
            let changed = word != with
                && tab
                    .edit(|d| match d.replace_all(&word, &with, opts) {
                        0 => Err(EditError::NotFound),
                        _ => Ok(nodes.into_iter().fold(false, |cut, n| cap_box(d, n) | cut)),
                    })
                    .inspect(|&cut| {
                        if cut {
                            *status_note = Some(t("status.text_cut"));
                        }
                    })
                    .is_some();
            let offset = if changed {
                let shift = with.len() as isize - word.len() as isize;
                (hit.range.start as isize + before as isize * shift) as usize + with.len()
            } else {
                hit.range.end
            };
            resume_at(tab, spell, offset);
        }
        _ => unreachable!("not a spelling action"),
    }
}

/// The word at `range` of `text` with up to [`CONTEXT_CHARS`] characters
/// on each side: (before, word, after), "…" where the text goes on.
pub fn context(text: &str, range: std::ops::Range<usize>) -> (String, String, String) {
    let before: Vec<char> = text[..range.start].chars().collect();
    let skip = before.len().saturating_sub(CONTEXT_CHARS);
    let mut head: String = before[skip..].iter().collect();
    if skip > 0 {
        head.insert(0, '…');
    }
    let rest = &text[range.end..];
    let mut tail: String = rest.chars().take(CONTEXT_CHARS).collect();
    if rest.chars().count() > CONTEXT_CHARS {
        tail.push('…');
    }
    (head, text[range].to_owned(), tail)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_trims_to_thirty_characters() {
        let text = format!("{} teh {}", "a".repeat(40), "b".repeat(40));
        let at = text.find("teh").unwrap();
        let (head, word, tail) = context(&text, at..at + 3);
        assert_eq!(word, "teh");
        assert_eq!(head, format!("…{} ", "a".repeat(29)));
        assert_eq!(tail, format!(" {}…", "b".repeat(29)));
        assert_eq!(
            context("teh", 0..3),
            (String::new(), "teh".into(), String::new())
        );
    }
}
