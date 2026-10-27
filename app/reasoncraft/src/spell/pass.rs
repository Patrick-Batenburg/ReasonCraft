//! The spelling pass (F7): the misspellings of box text, sources and notes
//! one at a time, in find order from the selected box (wrapping around),
//! each with Change, Change all, Ignore, Ignore all and Add to dictionary.
//! The walk goes on in slices of at most [`SLICE`] a frame, and the
//! suggestions are worked out on a worker thread, so the window never
//! waits for either.

use std::time::{Duration, Instant};

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
/// How long the walk goes on in one frame before it waits for the next
/// (a box being checked is finished first).
const SLICE: Duration = Duration::from_millis(8);

/// Where a pass looks next: a box (`None`: the start of the walk), its
/// place in the walk when last seen (used if the box is gone) and a byte
/// offset in its text.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Place {
    node: Option<NodeId>,
    index: usize,
    offset: usize,
}

/// One tab's spelling pass.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpellPass {
    /// The box the pass started at.
    start: Option<NodeId>,
    /// Where to look next.
    at: Place,
    /// The misspelling on show (its box is selected), and its word.
    pub current: Option<FindHit>,
    pub word: String,
    /// Up to [`MAX_SUGGESTIONS`] for the word; `None` while they are being
    /// worked out.
    pub suggestions: Option<Vec<String>>,
    /// What Change puts in: the word, then the first suggestion once they
    /// arrive, until the user edits it.
    pub replacement: String,
    /// Every box has been looked at: "Spelling check complete".
    pub done: bool,
    /// The last frame's walk stopped at its time limit: it goes on next
    /// frame.
    sliced: bool,
    /// The longest a step of the walk took (tests check it stays within
    /// a frame's share).
    longest: Duration,
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

    /// Whether the pass must look (again) for its misspelling: it waits
    /// for the dictionary, or the document changed under the current one
    /// (its box no longer holds the word there).
    pub fn needs_advance(&self, doc: &Document) -> bool {
        !self.done && self.current_word(doc).is_none()
    }

    /// Whether the walk stopped at its time limit and goes on next frame.
    pub fn walking(&self) -> bool {
        self.sliced && !self.done
    }

    /// The longest one step of the walk took so far (a test hook).
    pub fn longest_slice(&self) -> Duration {
        self.longest
    }

    /// The word on show, while its box still holds it at the hit.
    pub fn current_word(&self, doc: &Document) -> Option<&str> {
        let hit = self.current.as_ref()?;
        let text = &doc.boxes.get(&hit.node)?.text;
        (text.get(hit.range.clone()) == Some(self.word.as_str())).then_some(self.word.as_str())
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

/// Move `tab`'s pass on towards the next misspelling from where it is,
/// for at most about [`SLICE`]; see [`walk_on`].
pub(super) fn advance(tab: &mut DocTab, service: &mut SpellService) {
    let started = Instant::now();
    walk_on(tab, service, started);
    if let Some(pass) = &mut tab.spelling {
        pass.longest = pass.longest.max(started.elapsed());
    }
}

/// Move `tab`'s pass to the next misspelling from where it is, selecting
/// and revealing its box; or finish it. While the dictionary loads it
/// waits, and past [`SLICE`] since `started` it stops at the next box;
/// [`super::poll`] goes on next frame.
fn walk_on(tab: &mut DocTab, service: &mut SpellService, started: Instant) {
    let Some(mut pass) = tab.spelling.take() else {
        return;
    };
    pass.sliced = false;
    let lang = tab.doc.meta.language.clone();
    let order = walk(&tab.doc, pass.start);
    pass.current = None;
    let found = pass
        .at
        .node
        .and_then(|n| order.iter().position(|&o| o == n));
    let (mut i, mut from) = match (pass.at.node, found) {
        (None, _) => (0, pass.at.offset),
        (Some(_), Some(i)) => (i, pass.at.offset),
        // The box is gone: go on from where it was.
        (Some(_), None) => (pass.at.index.min(order.len()), 0),
    };
    let first = i;
    while i < order.len() {
        let node = order[i];
        if i > first && started.elapsed() >= SLICE {
            pass.at = Place {
                node: Some(node),
                index: i,
                offset: from,
            };
            pass.sliced = true;
            tab.spelling = Some(pass);
            return;
        }
        let Some(text) = tab.doc.boxes.get(&node).map(|b| &b.text) else {
            (i, from) = (i + 1, 0);
            continue;
        };
        let Some(ranges) = service.misspelled(&lang, text) else {
            tab.spelling = Some(pass);
            return;
        };
        let hit = ranges
            .iter()
            .filter(|r| r.start >= from)
            .find_map(|r| Some((r.clone(), text.get(r.clone())?)));
        if let Some((r, word)) = hit {
            pass.at = Place {
                node: Some(node),
                index: i,
                offset: r.start,
            };
            pass.word = word.to_owned();
            pass.suggestions = None;
            pass.replacement = pass.word.clone();
            pass.current = Some(FindHit { node, range: r });
            take_suggestions(&mut pass, service, &lang);
            tab.selection = Some(Item::Box(node));
            tab.reveal_pending = true;
            tab.spelling = Some(pass);
            return;
        }
        (i, from) = (i + 1, 0);
    }
    pass.at = Place {
        node: None,
        index: order.len(),
        offset: 0,
    };
    pass.done = true;
    tab.spelling = Some(pass);
}

/// Pick up the suggestions for the word on show once they are worked out
/// (starting that the first time); the replacement becomes the first one
/// unless the user changed it already.
pub(super) fn take_suggestions(pass: &mut SpellPass, service: &mut SpellService, lang: &str) {
    if pass.current.is_none() || pass.suggestions.is_some() {
        return;
    }
    if let Some(found) = service.suggest_async(lang, &pass.word) {
        if pass.replacement == pass.word
            && let Some(first) = found.first()
        {
            pass.replacement = first.clone();
        }
        pass.suggestions = Some(found);
    }
}

/// The current misspelling and its word, while its box still holds the
/// word there and `word` (when given) is that word.
fn current(tab: &DocTab, word: Option<&str>) -> Option<(FindHit, String)> {
    let pass = tab.spelling.as_ref()?;
    let found = pass.current_word(&tab.doc)?;
    let hit = pass.current.clone()?;
    word.is_none_or(|w| w == found)
        .then(|| (hit, found.to_owned()))
}

/// Look on from byte `offset` of the current box.
fn resume_at(tab: &mut DocTab, service: &mut SpellService, offset: usize) {
    if let Some(pass) = &mut tab.spelling {
        pass.at.offset = offset;
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
    match action {
        Action::OpenSpelling => {
            if settings.spellcheck {
                *right_panel = true;
                tab.spelling = Some(SpellPass::new(&tab.doc, tab.selection));
                advance(tab, spell);
            }
        }
        Action::CloseSpelling => tab.spelling = None,
        // The rest act on the current misspelling: when the document
        // changed under it (or the action names another word), the pass
        // looks again from where it is instead.
        Action::SpellIgnore => match current(tab, None) {
            Some((hit, _)) => resume_at(tab, spell, hit.range.end),
            None => advance(tab, spell),
        },
        Action::SpellIgnoreAll(word) => match current(tab, Some(&word)) {
            Some((hit, word)) => {
                spell.ignore_all(&word);
                resume_at(tab, spell, hit.range.end);
            }
            None => advance(tab, spell),
        },
        Action::SpellAddWord(word) => match current(tab, Some(&word)) {
            Some((hit, word)) => {
                match spell.add_word(&word) {
                    Some(word) if !settings.spell_words.contains(&word) => {
                        settings.spell_words.push(word);
                    }
                    Some(_) => {}
                    None => *status_note = Some(t("spell.add_refused")),
                }
                resume_at(tab, spell, hit.range.end);
            }
            None => advance(tab, spell),
        },
        Action::SpellChange { hit, with } => {
            let Some((_, word)) = current(tab, None).filter(|(h, _)| *h == hit) else {
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
            let Some((hit, word)) = current(tab, Some(&word)) else {
                return advance(tab, spell);
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
