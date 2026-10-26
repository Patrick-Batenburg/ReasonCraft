//! Find and replace: the find bar's commands.

use std::collections::HashMap;

use reasoncraft_core::find::search_order;
use reasoncraft_core::{EditError, FindHit, Item};

use super::Action;
use crate::app::ReasonCraftApp;
use crate::doc_tab::DocTab;
use crate::i18n::{FluentArgs, t_args};

pub(super) fn apply(app: &mut ReasonCraftApp, action: Action) {
    let Some(tab) = app.tabs.get_mut(app.active) else {
        return;
    };
    match action {
        Action::OpenFind { replace } => {
            tab.find.open = true;
            tab.find.replace = replace;
            tab.find.focus = true;
        }
        Action::CloseFind => {
            tab.find.open = false;
            tab.find.current = None;
            tab.find.focus = false;
        }
        Action::FindNext => step(tab, true),
        Action::FindPrev => step(tab, false),
        Action::ReplaceCurrent => replace_current(tab),
        Action::ReplaceAll => {
            let count = replace_all(tab);
            let mut args = FluentArgs::new();
            args.set("count", count);
            app.status_note = Some(t_args("find.replaced", &args));
        }
        Action::SetFindQuery(query) => {
            tab.find.query = query;
            go_first(tab);
        }
        Action::SetReplaceText(text) => tab.find.replacement = text,
        Action::SetFindOptions(opts) => {
            tab.find.opts = opts;
            go_first(tab);
        }
        _ => unreachable!("not a find action"),
    }
}

/// Replacing needs something to look for.
pub(super) fn is_enabled(tab: &DocTab, action: &Action) -> Option<bool> {
    match action {
        Action::ReplaceCurrent | Action::ReplaceAll => Some(!tab.find.query.is_empty()),
        _ => None,
    }
}

/// Make `hit` the current one: select its box and reveal it.
fn go_to(tab: &mut DocTab, hit: Option<FindHit>) {
    if let Some(h) = &hit {
        tab.selection = Some(Item::Box(h.node));
        tab.reveal_pending = true;
    }
    tab.find.current = hit;
}

fn go_first(tab: &mut DocTab) {
    let first = tab.find_hits().first().cloned();
    go_to(tab, first);
}

/// The next (or previous) hit after the current one, wrapping around; the
/// first (or last) when there is no current hit. A closed bar opens.
fn step(tab: &mut DocTab, forward: bool) {
    if !tab.find.open {
        tab.find.open = true;
        tab.find.focus = tab.find.query.is_empty();
    }
    let at = tab.find_position();
    let n = tab.find_hits().len();
    if n == 0 {
        tab.find.current = None;
        return;
    }
    let i = match (at, forward) {
        (Some(i), true) => (i + 1) % n,
        (Some(i), false) => (i + n - 1) % n,
        (None, true) => 0,
        (None, false) => n - 1,
    };
    let hit = tab.find_hits()[i].clone();
    go_to(tab, Some(hit));
}

/// Replace the current hit as one history step (none when the text is
/// already the replacement) and go to the first hit after the inserted
/// text, so a replacement containing the query is not found again.
fn replace_current(tab: &mut DocTab) {
    let Some(at) = tab.find_position() else {
        return step(tab, true);
    };
    let hit = tab.find_hits()[at].clone();
    let with = tab.find.replacement.clone();
    let same = tab.doc.boxes[&hit.node].text[hit.range.clone()] == with;
    if !same
        && tab
            .edit(|d| d.replace_range(hit.node, hit.range.clone(), &with))
            .is_none()
    {
        return;
    }
    let order: HashMap<_, _> = search_order(&tab.doc)
        .into_iter()
        .enumerate()
        .map(|(i, n)| (n, i))
        .collect();
    let key = |node, start| (order.get(&node).copied(), start);
    let after = key(hit.node, hit.range.start + with.len());
    let hits = tab.find_hits();
    let next = hits
        .iter()
        .find(|h| key(h.node, h.range.start) >= after)
        .or(hits.first())
        .cloned();
    go_to(tab, next);
}

/// Replace every hit as one history step; returns how many changed (none
/// is no step).
fn replace_all(tab: &mut DocTab) -> usize {
    let (query, opts) = (tab.find.query.clone(), tab.find.opts);
    let with = tab.find.replacement.clone();
    let hits = tab.find_hits().to_vec();
    let text = |h: &FindHit| &tab.doc.boxes[&h.node].text[h.range.clone()];
    let changes = hits.iter().filter(|h| text(h) != with).count();
    tab.find.current = None;
    if changes == 0 {
        return 0;
    }
    tab.edit(|d| match d.replace_all(&query, &with, opts) {
        0 => Err(EditError::NotFound),
        _ => Ok(()),
    });
    changes
}
