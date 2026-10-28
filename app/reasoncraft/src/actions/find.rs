//! Find and replace: the find bar's commands. A replacement never leaves
//! a box longer than [`MAX_BOX_CHARS`]: the box is cut in the same
//! history step, with the `status.text_cut` note.

use std::collections::HashMap;

use reasoncraft_core::caps::capped;
use reasoncraft_core::find::search_order;
use reasoncraft_core::{Document, EditError, FindHit, Item, MAX_BOX_CHARS, NodeId};

use super::Action;
use crate::app::ReasonCraftApp;
use crate::doc_tab::DocTab;
use crate::i18n::{FluentArgs, t, t_args};

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
        Action::ReplaceCurrent => {
            if replace_current(tab) {
                app.status_note = Some(t("status.text_cut"));
            }
        }
        Action::ReplaceAll => {
            let (count, cut) = replace_all(tab);
            let mut args = FluentArgs::new();
            args.set("count", count);
            app.status_note = Some(if cut {
                t("status.text_cut")
            } else {
                t_args("find.replaced", &args)
            });
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
fn step(tab: &mut DocTab, forward: bool) {
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

/// Cut box `node` to [`MAX_BOX_CHARS`] characters; true when it was cut.
pub(crate) fn cap_box(d: &mut Document, node: NodeId) -> bool {
    let Some(text) = d.boxes.get(&node).map(|b| &b.text) else {
        return false;
    };
    let kept = capped(text, MAX_BOX_CHARS);
    if kept.len() == text.len() {
        return false;
    }
    let kept = kept.to_owned();
    d.set_text(node, &kept).is_ok()
}

/// Replace the current hit as one history step (none when the text is
/// already the replacement) and go to the first hit after the inserted
/// text, so a replacement containing the query is not found again.
/// Returns whether the box had to be cut to [`MAX_BOX_CHARS`].
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
        let done = tab.edit(|d| {
            d.replace_range(hit.node, hit.range.clone(), &with)?;
            Ok(cap_box(d, hit.node))
        });
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
/// whether a box had to be cut to [`MAX_BOX_CHARS`].
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
    // Hits are grouped by box, so this leaves each box once.
    let mut nodes: Vec<NodeId> = hits.iter().map(|h| h.node).collect();
    nodes.dedup();
    let cut = tab.edit(|d| match d.replace_all(&query, &with, opts) {
        0 => Err(EditError::NotFound),
        _ => Ok(nodes.into_iter().fold(false, |cut, n| cap_box(d, n) | cut)),
    });
    (changes, cut.unwrap_or(false))
}
