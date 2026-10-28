//! Comments on boxes, the comment popover and the author name.

use reasoncraft_core::{Item, NodeId, Uuid, Verdict};

use super::Action;
use crate::app::ReasonCraftApp;
use crate::doc_tab::DocTab;

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
        Action::AddComment { node, text, .. } => can_add(tab, *node, text),
        Action::RemoveComment { node, comment } => can_remove(tab, *node, *comment),
        Action::OpenComments(node) => tab.doc.boxes.contains_key(node),
        Action::CommentSelected => selected_box(tab).is_some(),
        _ => return None,
    })
}

fn can_add(tab: &DocTab, node: NodeId, text: &str) -> bool {
    tab.doc.boxes.contains_key(&node) && !text.trim().is_empty()
}

fn can_remove(tab: &DocTab, node: NodeId, comment: Uuid) -> bool {
    tab.doc
        .boxes
        .get(&node)
        .is_some_and(|n| n.comments.iter().any(|c| c.id == comment))
}

// A blank comment, or one for a box that does not exist, changes nothing
// and records no history.

/// Comment on box `node` as the author in the settings (no name: stored
/// empty, shown as "Anonymous" in the reader's language).
pub(super) fn add(app: &mut ReasonCraftApp, node: NodeId, text: &str, verdict: Option<Verdict>) {
    let author = app.settings.author.trim().to_owned();
    if let Some(tab) = app.tabs.get_mut(app.active)
        && can_add(tab, node, text)
    {
        tab.edit(|d| d.add_comment(node, &author, text.trim(), verdict));
    }
}

pub(super) fn remove(tab: &mut DocTab, node: NodeId, comment: Uuid) {
    if can_remove(tab, node, comment) {
        tab.edit(|d| d.remove_comment(node, comment));
    }
}

/// Show the comment popover of the selected box (ribbon, Ctrl+Shift+M).
pub(super) fn comment_selected(tab: &mut DocTab) {
    if let Some(b) = selected_box(tab) {
        open(tab, b);
    }
}

/// Show the comment popover of `node`, selecting the box (a box that
/// does not exist is ignored).
pub(super) fn open(tab: &mut DocTab, node: NodeId) {
    if !tab.doc.boxes.contains_key(&node) {
        return;
    }
    tab.selection = Some(Item::Box(node));
    tab.comments_open = Some(node);
}

#[cfg(test)]
mod tests;
