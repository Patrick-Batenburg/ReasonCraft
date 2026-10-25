//! Snippets: adding, live text editing, source tags, removal and order.

use reasoncraft_core::{SnippetSource, SourceType};

use super::Action;
use crate::doc_tab::{DocTab, cap_snippet};
use crate::shell::link_modal::normalize_link;

/// Carry out a snippet action. Changes that leave the snippet list as it
/// was (same text, source or place) record nothing.
pub(super) fn apply(tab: &mut DocTab, action: Action) {
    match action {
        Action::AddSnippet(text) => add(tab, &text),
        Action::EditSnippetText { id, text } => tab.set_snippet_text(id, &text),
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

/// `source` as stored: citation trimmed, link checked and normalised
/// (`Err` when it is not a valid link), and `None` when nothing is left
/// (generic type, no citation, no link).
fn normalise(source: Option<SnippetSource>) -> Result<Option<SnippetSource>, ()> {
    let Some(mut s) = source else {
        return Ok(None);
    };
    s.citation = s.citation.trim().to_owned();
    s.link = match s.link.as_deref().map(normalize_link) {
        None => None,
        Some(Ok(link)) => link,
        Some(Err(_)) => return Err(()),
    };
    let empty = s.citation.is_empty() && s.link.is_none() && s.source_type == SourceType::Source;
    Ok((!empty).then_some(s))
}

#[cfg(test)]
mod tests;
