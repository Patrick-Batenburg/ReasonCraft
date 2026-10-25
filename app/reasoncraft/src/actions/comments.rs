//! Comments on boxes, the comment popover and the author name.

use reasoncraft_core::{Item, NodeId};

use super::Action;
use crate::app::ReasonCraftApp;
use crate::doc_tab::DocTab;
use crate::i18n::t;

/// The box that "Comment" (ribbon, Ctrl+Shift+M) opens comments for.
fn selected_box(tab: &DocTab) -> Option<NodeId> {
    match tab.selection? {
        Item::Box(b) => tab.doc.boxes.contains_key(&b).then_some(b),
        Item::Card(_) => None,
    }
}

/// Whether a comment action applies now; `None` for other actions.
pub(super) fn is_enabled(tab: &DocTab, action: &Action) -> Option<bool> {
    Some(match action {
        Action::AddComment { node, text, .. } => {
            tab.doc.boxes.contains_key(node) && !text.trim().is_empty()
        }
        Action::RemoveComment { node, comment } => tab
            .doc
            .boxes
            .get(node)
            .is_some_and(|n| n.comments.iter().any(|c| c.id == *comment)),
        Action::OpenComments(node) => tab.doc.boxes.contains_key(node),
        Action::CommentSelected => selected_box(tab).is_some(),
        _ => return None,
    })
}

/// Carry out a comment action. Adding needs the author name from the
/// settings, so this works on the whole app. A blank comment, or one for
/// a box that does not exist, changes nothing and records no history.
pub(super) fn apply(app: &mut ReasonCraftApp, action: Action) {
    match action {
        Action::SetAuthor(name) => {
            app.settings.author = name.trim().to_owned();
            return;
        }
        Action::CloseComments => {
            if let Some(tab) = app.tabs.get_mut(app.active) {
                tab.comments_open = None;
            }
            return;
        }
        _ => {}
    }
    let author = match app.settings.author.trim() {
        "" => t("comment.anonymous"),
        name => name.to_owned(),
    };
    let Some(tab) = app.tabs.get_mut(app.active) else {
        return;
    };
    if !is_enabled(tab, &action).unwrap_or(false) {
        return;
    }
    match action {
        Action::AddComment {
            node,
            text,
            verdict,
        } => {
            tab.edit(|d| d.add_comment(node, &author, text.trim(), verdict));
        }
        Action::RemoveComment { node, comment } => {
            tab.edit(|d| d.remove_comment(node, comment));
        }
        Action::OpenComments(node) => open(tab, node),
        Action::CommentSelected => {
            if let Some(b) = selected_box(tab) {
                open(tab, b);
            }
        }
        _ => {}
    }
}

/// Show the comment popover of `node`, selecting the box.
fn open(tab: &mut DocTab, node: NodeId) {
    tab.selection = Some(Item::Box(node));
    tab.comments_open = Some(node);
}

#[cfg(test)]
mod tests;
