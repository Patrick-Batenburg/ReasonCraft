//! The spelling panel's commands, in the active tab. All but opening and
//! closing act on the current misspelling: when the document changed
//! under it (or the command names another word), the pass looks again
//! from where it is instead.

use reasoncraft_core::{EditError, FindHit, FindOptions, find_all};

use super::SpellService;
use super::pass::{SpellPass, advance, current, resume_at};
use crate::app::ReasonCraftApp;
use crate::doc_tab::DocTab;
use crate::i18n::t;

/// The active tab and the spelling service, if there is a tab.
fn parts(app: &mut ReasonCraftApp) -> Option<(&mut DocTab, &mut SpellService)> {
    let tab = app.tabs.get_mut(app.active)?;
    Some((tab, &mut app.spell))
}

/// Start a pass from the selection and show its panel (nothing with
/// spellcheck off).
pub(crate) fn open(app: &mut ReasonCraftApp) {
    if !app.settings.spellcheck {
        return;
    }
    app.right_panel = true;
    if let Some((tab, spell)) = parts(app) {
        tab.spelling = Some(SpellPass::new(&tab.doc, tab.selection));
        advance(tab, spell);
    }
}

pub(crate) fn close(tab: &mut DocTab) {
    tab.spelling = None;
}

/// Skip the current misspelling.
pub(crate) fn ignore(app: &mut ReasonCraftApp) {
    let Some((tab, spell)) = parts(app) else {
        return;
    };
    match current(tab, None) {
        Some((hit, _)) => resume_at(tab, spell, hit.range.end),
        None => advance(tab, spell),
    }
}

/// Accept `word` everywhere until the app closes, and go on.
pub(crate) fn ignore_all(app: &mut ReasonCraftApp, word: &str) {
    let Some((tab, spell)) = parts(app) else {
        return;
    };
    match current(tab, Some(word)) {
        Some((hit, word)) => {
            spell.ignore_all(&word);
            resume_at(tab, spell, hit.range.end);
        }
        None => advance(tab, spell),
    }
}

/// Add `word` to the personal dictionary (kept in the settings), and go on.
pub(crate) fn add_word(app: &mut ReasonCraftApp, word: &str) {
    let ReasonCraftApp {
        tabs,
        active,
        spell,
        settings,
        status_note,
        ..
    } = app;
    let Some(tab) = tabs.get_mut(*active) else {
        return;
    };
    match current(tab, Some(word)) {
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
    }
}

/// Replace the current misspelling `hit` with `with` (one history step)
/// and go to the next. A stale hit only moves on.
pub(crate) fn change(app: &mut ReasonCraftApp, hit: FindHit, with: &str) {
    let Some(tab) = app.tabs.get_mut(app.active) else {
        return;
    };
    let spell = &mut app.spell;
    let Some((_, word)) = current(tab, None).filter(|(h, _)| *h == hit) else {
        return advance(tab, spell);
    };
    if word != with {
        let done = tab.edit(|d| d.replace_range(hit.node, hit.range.clone(), with));
        if done == Some(true) {
            app.status_note = Some(t("status.text_cut"));
        }
    }
    resume_at(tab, spell, hit.range.start + with.len());
}

/// Replace every whole-word, same-case `word` with `with` (one history
/// step) and go to the next misspelling.
pub(crate) fn change_all(app: &mut ReasonCraftApp, word: &str, with: &str) {
    let Some(tab) = app.tabs.get_mut(app.active) else {
        return;
    };
    let spell = &mut app.spell;
    let Some((hit, word)) = current(tab, Some(word)) else {
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
    let changed = word != with
        && tab
            .edit(|d| match d.replace_all(&word, with, opts) {
                done if done.count == 0 => Err(EditError::NotFound),
                done => Ok(done.cut),
            })
            .inspect(|&cut| {
                if cut {
                    app.status_note = Some(t("status.text_cut"));
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
