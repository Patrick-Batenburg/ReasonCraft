//! Cut, Copy and Paste of map items: a box or card with everything under
//! it, within a map and between tabs.
//!
//! Copy keeps the item (a [`Fragment`], with the images it shows) on the
//! app's own clipboard and puts its text, one box per line, on the system
//! clipboard for other programs. Paste puts a fresh copy under the
//! selection (in the parking area with nothing selected). When the system
//! clipboard holds other text by then (copied in another program), Paste
//! adds that text as a new box instead, like text dropped on the map.

use reasoncraft_core::{Assets, Fragment, Item, Parent, RemoveMode, can_attach};

use super::insert::{self, Insertable, target_allowed};
use super::structure::delete;
use crate::app::ReasonCraftApp;
use crate::doc_tab::DocTab;

/// What Copy and Cut keep, shared by every tab.
#[derive(Clone, Debug)]
pub struct Copied {
    fragment: Fragment,
    /// The images the fragment's boxes show.
    assets: Assets,
    /// The fragment as text: what went on the system clipboard.
    text: String,
}

impl Copied {
    /// Whether `text` (from the system clipboard) is still what Copy put
    /// there, line endings and trailing space aside.
    fn is_ours(&self, text: &str) -> bool {
        text.replace("\r\n", "\n").trim_end() == self.text.trim_end()
    }
}

/// The selection, if it can be copied: any box but the root claim, or a card.
fn copyable(tab: &DocTab) -> Option<Item> {
    let item = tab.selection?;
    (item != Item::Box(tab.doc.root) && tab.contains(item)).then_some(item)
}

/// Whether Copy and Cut apply to `tab`'s selection.
pub(super) fn can_copy(tab: &DocTab) -> bool {
    copyable(tab).is_some()
}

/// Where Paste puts things: under the selected box or card, or in the
/// parking area with nothing selected.
fn paste_target(tab: &DocTab) -> Parent {
    match tab.selection {
        Some(Item::Box(b)) => Parent::Box(b),
        Some(Item::Card(c)) => Parent::Card(c),
        None => Parent::Parking,
    }
}

/// Whether Paste could add something at the selection: the copied item
/// fits there, or text from another program would.
pub(super) fn can_paste(app: &ReasonCraftApp) -> bool {
    let Some(tab) = app.tabs.get(app.active) else {
        return false;
    };
    let to = paste_target(tab);
    let text = Insertable::Text {
        text: "text".to_owned(),
        source: None,
    };
    app.clipboard
        .as_ref()
        .is_some_and(|c| can_attach(&tab.doc, to, c.fragment.new_item()))
        || target_allowed(&tab.doc, &text, Some(to), None)
}

/// Keep the selection on the app's clipboard and its text on the system's.
pub(super) fn copy(app: &mut ReasonCraftApp) {
    let Some(tab) = app.tabs.get(app.active) else {
        return;
    };
    let Some(fragment) = copyable(tab).and_then(|item| tab.doc.copy_fragment(item)) else {
        return;
    };
    let mut assets = Assets::default();
    for key in fragment.assets() {
        assets.copy_from(&tab.assets, key);
    }
    let text = fragment.plain_text();
    if let Some(ctx) = &app.egui_ctx {
        ctx.copy_text(text.clone());
    }
    app.clipboard = Some(Copied {
        fragment,
        assets,
        text,
    });
}

/// Copy the selection, then delete it with everything under it (one
/// history step for the delete).
pub(super) fn cut(app: &mut ReasonCraftApp) {
    if !app.tabs.get(app.active).is_some_and(can_copy) {
        return;
    }
    copy(app);
    if let Some(tab) = app.tabs.get_mut(app.active) {
        delete(tab, RemoveMode::Branch);
    }
}

/// Paste at the selection (one history step). `text` is the system
/// clipboard's text when known (Ctrl+V); otherwise (the ribbon, Search
/// commands) it is read here.
pub(super) fn paste(app: &mut ReasonCraftApp, text: Option<String>) {
    let text = text.or_else(|| app.export.read_clipboard_text());
    let ours = app.clipboard.as_ref().is_some_and(|c| match &text {
        Some(text) => c.is_ours(text),
        None => true,
    });
    match text {
        Some(text) if !ours && !text.trim().is_empty() => {
            let Some(tab) = app.tabs.get(app.active) else {
                return;
            };
            let to = paste_target(tab);
            let what = Insertable::Text { text, source: None };
            insert::apply(app, what, Some(to), None);
        }
        _ => paste_copied(app),
    }
}

/// Paste a fresh copy of the copied item, with its images, and select it.
fn paste_copied(app: &mut ReasonCraftApp) {
    let (Some(copied), Some(tab)) = (&app.clipboard, app.tabs.get_mut(app.active)) else {
        return;
    };
    let to = paste_target(tab);
    if !can_attach(&tab.doc, to, copied.fragment.new_item()) {
        return;
    }
    let Some(item) = tab.edit(|d| d.paste_fragment(&copied.fragment, to)) else {
        return;
    };
    // Outside history, like an inserted image: undo leaves them unused.
    for key in copied.fragment.assets() {
        tab.assets.copy_from(&copied.assets, key);
    }
    tab.selection = Some(item);
    tab.reveal_pending = true;
}

#[cfg(test)]
mod tests;
