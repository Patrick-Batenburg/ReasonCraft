//! Snippets: adding, live text editing, source tags, removal and order.

use reasoncraft_core::{Snippet, SnippetSource, SourceType};

use super::Action;
use crate::doc_tab::{DocTab, cap_snippet, tidy_source};
use crate::shell::link_modal::normalize_link;

/// Carry out a snippet action. Changes that leave the snippet list as it
/// was (same text, source or place) record nothing.
pub(super) fn apply(tab: &mut DocTab, action: Action) {
    match action {
        Action::AddSnippet(text) => add(tab, &text),
        Action::EditSnippetText { id, text } => tab.set_snippet_text(id, &text),
        Action::EditSnippetCitation { id, citation } => tab.edit_snippet_live(id, |s, _| {
            if source_of(s).citation != citation {
                let mut src = source_of(s);
                src.citation = citation;
                s.source = Some(src);
            }
        }),
        Action::EditSnippetLink { id, link } => tab.edit_snippet_live(id, |s, before| {
            // While the typed link is invalid, the link from before the
            // session stays.
            let link = normalize_link(&link).unwrap_or_else(|_| {
                let before = before.snippets.iter().find(|b| b.id == id);
                before.and_then(|b| b.source.as_ref()?.link.clone())
            });
            if source_of(s).link != link {
                let mut src = source_of(s);
                src.link = link;
                s.source = Some(src);
            }
        }),
        Action::SetSnippetSource { id, source } => {
            let Some(current) = tab.doc.snippets.iter().find(|s| s.id == id) else {
                return;
            };
            // An invalid link is refused, not stored.
            let Ok(source) = normalise(source) else {
                return;
            };
            if current.source != source {
                tab.edit(|d| d.set_snippet_source(id, source));
            }
        }
        Action::RemoveSnippet(id) => {
            if tab.edit(|d| d.remove_snippet(id)).is_some() {
                tab.fix_selection();
            }
        }
        Action::MoveSnippet { id, index } => {
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
        _ => {}
    }
}

/// Add a snippet with `text`, cut at the length limit, and focus it (which
/// also scrolls it into view). An empty one opens an editing session, so
/// it and its first text are one history step; one with text is a step of
/// its own.
fn add(tab: &mut DocTab, text: &str) {
    let (text, capped) = cap_snippet(text);
    let id = if text.is_empty() {
        let before = tab.doc.clone();
        let id = tab.doc.add_snippet("");
        tab.bump_revision();
        tab.snippet_edit = Some((id, before));
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
