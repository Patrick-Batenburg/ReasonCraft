//! The offer, on start, to restore documents a crashed session left behind.

use reasoncraft_core::file::{self, FileError};

use super::{RecoveredDoc, session};
use crate::app::ReasonCraftApp;
use crate::doc_tab::DocTab;
use crate::file_ops::{self, FileOp, Modal};

/// The answer to the recovery prompt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveryChoice {
    /// Open every document in a tab of its own.
    RestoreAll,
    /// Delete every copy.
    DiscardAll,
    /// Keep the copies; the next start asks again.
    Later,
}

/// Show the recovery prompt when crashed sessions left documents behind.
pub(crate) fn offer(app: &mut ReasonCraftApp) {
    let Some(store) = &app.recovery else {
        return;
    };
    let docs = store.orphaned();
    if !docs.is_empty() {
        app.modal = Some(Modal::Recover(docs));
    }
}

/// Put a showing recovery prompt aside, so another message can show first.
pub(crate) fn defer_offer(app: &mut ReasonCraftApp) {
    if let Some(Modal::Recover(docs)) = app.modal.take() {
        app.recovery_offer = Some(docs);
    }
}

/// Show the recovery prompt put aside, once no other message is open.
pub(crate) fn resume_offer(app: &mut ReasonCraftApp) {
    if app.modal.is_none()
        && let Some(docs) = app.recovery_offer.take()
    {
        app.modal = Some(Modal::Recover(docs));
    }
}

/// The answer to the recovery prompt.
pub(crate) fn resolve(app: &mut ReasonCraftApp, choice: RecoveryChoice) {
    let Some(Modal::Recover(docs)) = app.modal.take() else {
        return;
    };
    match choice {
        RecoveryChoice::RestoreAll => {
            let mut first_error = None;
            for doc in &docs {
                if let Err(err) = restore(app, doc) {
                    first_error.get_or_insert((doc, err));
                }
            }
            // A copy that cannot be read is kept; the next start offers it again.
            if let Some((doc, err)) = first_error {
                file_ops::show_error(app, FileOp::Open, &doc.package, &err);
            }
        }
        RecoveryChoice::DiscardAll => {
            if let Some(store) = &app.recovery {
                for doc in &docs {
                    store.discard(doc);
                }
            }
        }
        RecoveryChoice::Later => {}
    }
}

/// Open `doc` in a tab: unsaved, under its original path, marked as
/// recovered. It is autosaved under this session at once, then the
/// orphaned copy is deleted.
fn restore(app: &mut ReasonCraftApp, doc: &RecoveredDoc) -> Result<(), FileError> {
    let mut tab = DocTab::from_package(file::open_package(&doc.package)?);
    tab.path = doc.meta.original.clone();
    tab.disk_mtime = doc.meta.original_mtime;
    tab.history.saved_marker = None;
    tab.recovered_from = Some(doc.meta.clone());
    let i = file_ops::place_tab(app, tab);
    if let Some(store) = &app.recovery {
        session::autosave_tab(store, &mut app.tabs[i]);
        store.discard(doc);
    }
    Ok(())
}
