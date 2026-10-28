//! Snippets: adding, live text editing, source tags, removal and order.

use reasoncraft_core::{Snippet, SnippetSource, SourceType, Uuid};

use crate::doc_tab::{DocTab, EditSession, cap_snippet, tidy_source};
use crate::shell::link_modal::normalize_link;

// Changes that leave the snippet list as it was (same text, source or
// place) record nothing.

/// Snippet `id`'s citation field changed: show it live.
pub(super) fn edit_citation(tab: &mut DocTab, id: Uuid, citation: String) {
    tab.edit_snippet_live(id, |s, _| {
        if source_of(s).citation != citation {
            let mut src = source_of(s);
            src.citation = citation;
            s.source = Some(src);
        }
    });
}

/// Snippet `id`'s link field changed: show it live. While the typed link
/// is invalid, the link from before the session stays.
pub(super) fn edit_link(tab: &mut DocTab, id: Uuid, link: String) {
    tab.edit_snippet_live(id, |s, before| {
        let link = normalize_link(&link).unwrap_or_else(|_| {
            let before = before.snippets.iter().find(|b| b.id == id);
            before.and_then(|b| b.source.as_ref()?.link.clone())
        });
        if source_of(s).link != link {
            let mut src = source_of(s);
            src.link = link;
            s.source = Some(src);
        }
    });
}

/// Tag snippet `id` with a source (`None` clears). An invalid link is
/// refused, not stored.
pub(super) fn set_source(tab: &mut DocTab, id: Uuid, source: Option<SnippetSource>) {
    let Some(current) = tab.doc.snippets.iter().find(|s| s.id == id) else {
        return;
    };
    let Ok(source) = normalise(source) else {
        return;
    };
    if current.source != source {
        tab.edit(|d| d.set_snippet_source(id, source));
    }
}

pub(super) fn remove(tab: &mut DocTab, id: Uuid) {
    if tab.edit(|d| d.remove_snippet(id)).is_some() {
        tab.fix_selection();
    }
}

/// Move snippet `id` to `index` in the list (with it taken out).
pub(super) fn move_to(tab: &mut DocTab, id: Uuid, index: usize) {
    let list = &tab.doc.snippets;
    let to = index.min(list.len().saturating_sub(1));
    if list
        .iter()
        .position(|s| s.id == id)
        .is_some_and(|i| i != to)
    {
        tab.edit(|d| d.move_snippet(id, to));
    }
}

/// Add a snippet with `text`, cut at the length limit, and focus it (which
/// also scrolls it into view). An empty one opens an editing session, so
/// it and its first text are one history step; one with text is a step of
/// its own.
pub(super) fn add(tab: &mut DocTab, text: &str) {
    let (text, capped) = cap_snippet(text);
    let id = if text.is_empty() {
        let before = tab.doc.clone();
        let id = tab.doc.add_snippet("");
        tab.bump_revision();
        tab.begin_session_with(EditSession::Snippet { id }, before);
        tab.snippet_focus = Some(id);
        id
    } else {
        let Some(id) = tab.edit(|d| Ok(d.add_snippet(text))) else {
            return;
        };
        tab.snippet_focus = Some(id);
        id
    };
    tab.snippet_capped = capped.then_some(id);
}

/// `source` as stored: link checked and normalised (`Err` when it is not
/// a valid link), then tidied (see [`tidy_source`]).
fn normalise(source: Option<SnippetSource>) -> Result<Option<SnippetSource>, ()> {
    let Some(mut s) = source else {
        return Ok(None);
    };
    s.link = match s.link.as_deref().map(normalize_link) {
        None => None,
        Some(Ok(link)) => link,
        Some(Err(_)) => return Err(()),
    };
    Ok(tidy_source(Some(s)))
}

/// `s`'s source, or an empty one with the generic type.
fn source_of(s: &Snippet) -> SnippetSource {
    s.source
        .clone()
        .unwrap_or_else(|| SnippetSource::new(SourceType::Source, "", None))
}

#[cfg(test)]
mod tests;
