//! Find and replace: the find bar's commands. A replacement never leaves
//! a box longer than [`MAX_BOX_CHARS`](reasoncraft_core::MAX_BOX_CHARS):
//! the core cuts it, and the app adds the `status.text_cut` note.

use std::collections::HashMap;

use reasoncraft_core::find::search_order;
use reasoncraft_core::{Document, EditError, FindHit, FindOptions, Item, NodeId};

use super::Action;
use crate::app::ReasonCraftApp;
use crate::doc_tab::DocTab;
use crate::i18n::{FluentArgs, t, t_args};

/// Show the find bar (with the replacement field when `replace`) and
/// focus its query field.
pub(super) fn open(tab: &mut DocTab, replace: bool) {
    tab.find.open = true;
    tab.find.replace = replace;
    tab.find.focus = true;
}

/// Hide the find bar.
pub(super) fn close(tab: &mut DocTab) {
    tab.find.open = false;
    tab.find.current = None;
    tab.find.focus = false;
}

/// Replace the current hit, with a note when the box had to be cut.
pub(super) fn replace_one(app: &mut ReasonCraftApp) {
    if let Some(tab) = app.tabs.get_mut(app.active)
        && replace_current(tab)
    {
        app.status_note = Some(t("status.text_cut"));
    }
}

/// Replace every hit, with a note of how many (or that a box was cut).
pub(super) fn replace_every(app: &mut ReasonCraftApp) {
    let Some(tab) = app.tabs.get_mut(app.active) else {
        return;
    };
    let (count, cut) = replace_all(tab);
    let mut args = FluentArgs::new();
    args.set("count", count);
    app.status_note = Some(if cut {
        t("status.text_cut")
    } else {
        t_args("find.replaced", &args)
    });
}

/// The query changed: go to its first hit.
pub(super) fn set_query(tab: &mut DocTab, query: String) {
    tab.find.query = query;
    go_first(tab);
}

/// Match case and whole word changed: go to the first hit.
pub(super) fn set_options(tab: &mut DocTab, opts: FindOptions) {
    tab.find.opts = opts;
    go_first(tab);
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

/// A place in the document in search order: the box's index in
/// [`search_order`] (`None` for a box that is gone), then the byte offset.
type Place = (Option<usize>, usize);

/// The [`Place`] of an offset in a box of `doc`.
fn place_fn(doc: &Document) -> impl Fn(NodeId, usize) -> Place + use<> {
    let order: HashMap<_, _> = search_order(doc)
        .into_iter()
        .enumerate()
        .map(|(i, n)| (n, i))
        .collect();
    move |node, at| (order.get(&node).copied(), at)
}

/// The next (or previous) hit after the current one, wrapping around. A
/// current hit that went stale (its box's text changed) still counts
/// where it was: Next goes to the first hit at or after its place,
/// Previous to the last one before it. With no current hit, the first
/// (or last). A closed bar opens.
pub(super) fn step(tab: &mut DocTab, forward: bool) {
    if !tab.find.open {
        tab.find.open = true;
        tab.find.focus = tab.find.query.is_empty();
    }
    let at = tab.find_position();
    let stale = tab.find.current.clone().filter(|_| at.is_none());
    let place = place_fn(&tab.doc);
    let hits = tab.find_hits();
    let n = hits.len();
    if n == 0 {
        tab.find.current = None;
        return;
    }
    let i = match (at, stale) {
        (Some(i), _) if forward => (i + 1) % n,
        (Some(i), _) => (i + n - 1) % n,
        (None, Some(old)) => {
            let old = place(old.node, old.range.start);
            let from = hits
                .iter()
                .position(|h| place(h.node, h.range.start) >= old)
                .unwrap_or(n);
            if forward {
                from % n
            } else {
                (from + n - 1) % n
            }
        }
        (None, None) if forward => 0,
        (None, None) => n - 1,
    };
    let hit = hits[i].clone();
    go_to(tab, Some(hit));
}

/// Replace the current hit as one history step (none when the text is
/// already the replacement) and go to the first hit after the inserted
/// text, so a replacement containing the query is not found again.
/// Returns whether the box had to be cut.
fn replace_current(tab: &mut DocTab) -> bool {
    let Some(at) = tab.find_position() else {
        step(tab, true);
        return false;
    };
    let hit = tab.find_hits()[at].clone();
    let with = tab.find.replacement.clone();
    let same = tab.doc.boxes[&hit.node].text[hit.range.clone()] == with;
    let mut cut = false;
    if !same {
        let done = tab.edit(|d| d.replace_range(hit.node, hit.range.clone(), &with));
        match done {
            Some(c) => cut = c,
            None => return false,
        }
    }
    let place = place_fn(&tab.doc);
    let after = place(hit.node, hit.range.start + with.len());
    let hits = tab.find_hits();
    let next = hits
        .iter()
        .find(|h| place(h.node, h.range.start) >= after)
        .or(hits.first())
        .cloned();
    go_to(tab, next);
    cut
}

/// Replace every hit as one history step (none when nothing changes).
/// Returns how many hits changed, which leaves out hits that already read
/// as the replacement ([`Document::replace_all`] counts every hit), and
/// whether a box had to be cut.
fn replace_all(tab: &mut DocTab) -> (usize, bool) {
    let (query, opts) = (tab.find.query.clone(), tab.find.opts);
    let with = tab.find.replacement.clone();
    let hits = tab.find_hits().to_vec();
    let text = |h: &FindHit| &tab.doc.boxes[&h.node].text[h.range.clone()];
    let changes = hits.iter().filter(|h| text(h) != with).count();
    tab.find.current = None;
    if changes == 0 {
        return (0, false);
    }
    let cut = tab.edit(|d| match d.replace_all(&query, &with, opts) {
        done if done.count == 0 => Err(EditError::NotFound),
        done => Ok(done.cut),
    });
    (changes, cut.unwrap_or(false))
}
