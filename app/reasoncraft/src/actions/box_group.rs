//! The Home ribbon's Box group: a card's kind, and a box's link (edit it,
//! open it in the browser).

use std::cell::RefCell;
use std::rc::Rc;

use reasoncraft_core::{Item, NodeId};

use super::{Action, selected_card};
use crate::app::ReasonCraftApp;
use crate::doc_tab::DocTab;
use crate::file_ops::Modal;
use crate::i18n::{FluentArgs, t, t_args};
use crate::shell::link_modal::{LinkEdit, LinkError, normalize_link, safe_link, visible_link};

/// Opens a link in the user's browser or mail program.
pub trait LinkOpener {
    fn open(&self, url: &str) -> Result<(), String>;
}

/// The platform's handler for the link.
pub struct SystemOpener;

impl LinkOpener for SystemOpener {
    fn open(&self, url: &str) -> Result<(), String> {
        // Detached: `open::that` waits for the launcher (on Linux `xdg-open`
        // may wait for the browser), which would freeze the UI.
        open::that_detached(url).map_err(|e| e.to_string())
    }
}

/// Test opener: records every link instead of opening it, and fails with
/// `fail` when set. Clones share their state.
#[derive(Clone, Default)]
pub struct FakeOpener {
    pub opened: Rc<RefCell<Vec<String>>>,
    pub fail: Rc<RefCell<Option<String>>>,
}

impl LinkOpener for FakeOpener {
    fn open(&self, url: &str) -> Result<(), String> {
        self.opened.borrow_mut().push(url.to_owned());
        self.fail.borrow().clone().map_or(Ok(()), Err)
    }
}

/// The selected box, if a box is selected.
fn selected_box(tab: &DocTab) -> Option<NodeId> {
    match tab.selection? {
        Item::Box(b) => tab.doc.boxes.contains_key(&b).then_some(b),
        Item::Card(_) => None,
    }
}

/// A link to store: normalized, `None` for no link, `Err` when refused.
fn checked(link: Option<&str>) -> Result<Option<String>, LinkError> {
    link.map_or(Ok(None), normalize_link)
}

/// Box `b`'s stored link, if it is safe to open.
fn stored_safe_link(tab: &DocTab, b: NodeId) -> Option<String> {
    safe_link(tab.doc.boxes.get(&b)?.link.as_deref()?)
}

/// Whether a Box-group action applies now; `None` for other actions.
pub(super) fn is_enabled(tab: &DocTab, action: &Action) -> Option<bool> {
    Some(match action {
        Action::SetCardKind(_) => selected_card(tab).is_some(),
        Action::EditLink => selected_box(tab).is_some(),
        Action::SetLink(link) => selected_box(tab).is_some() && checked(link.as_deref()).is_ok(),
        Action::OpenLink(b) => stored_safe_link(tab, *b).is_some(),
        _ => return None,
    })
}

/// Carry out a Box-group action. Setting what is already there (the same
/// kind, the same link) changes nothing and records no history.
pub(super) fn apply(app: &mut ReasonCraftApp, action: Action) {
    let Some(tab) = app.tabs.get_mut(app.active) else {
        return;
    };
    match action {
        Action::SetCardKind(kind) => {
            if let Some(c) = selected_card(tab)
                && tab.doc.cards[&c].kind != kind
            {
                tab.edit(|d| d.set_card_kind(c, kind));
            }
        }
        Action::EditLink => {
            if let Some(b) = selected_box(tab) {
                app.link_edit = Some(LinkEdit::new(b, tab.doc.boxes[&b].link.as_deref()));
            }
        }
        Action::SetLink(link) => {
            // Checked here too, not only in the editor: a refused link
            // changes nothing and records no history.
            let Ok(link) = checked(link.as_deref()) else {
                return;
            };
            // The box the editor was opened for, else the selected box.
            let target = app.link_edit.take().map(|e| e.node);
            if let Some(b) = target.or_else(|| selected_box(tab))
                && tab.doc.boxes.get(&b).is_some_and(|n| n.link != link)
            {
                tab.edit(|d| d.set_link(b, link));
            }
        }
        Action::OpenLink(b) => {
            let Some(stored) = tab.doc.boxes.get(&b).and_then(|n| n.link.as_deref()) else {
                return;
            };
            // Only safe http, https and mailto links reach the OS: a file
            // may carry `file:` paths, UNC shares or script links.
            let Some(url) = safe_link(stored) else {
                let mut args = FluentArgs::new();
                args.set("url", visible_link(stored));
                app.modal = Some(Modal::Error {
                    title: t("link.open_failed"),
                    message: t_args("link.unsafe", &args),
                });
                return;
            };
            if let Err(detail) = app.link_opener.open(&url) {
                let mut args = FluentArgs::new();
                args.set("url", url);
                args.set("detail", detail);
                app.modal = Some(Modal::Error {
                    title: t("link.open_failed"),
                    message: t_args("link.open_failed_detail", &args),
                });
            }
        }
        _ => unreachable!("not a Box-group action"),
    }
}

#[cfg(test)]
mod tests;
