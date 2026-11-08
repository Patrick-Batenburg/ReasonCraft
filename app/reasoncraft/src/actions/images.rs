//! Putting an image in the selected box, and taking it out again.
//!
//! The picked file is read and prepared on a worker thread (see
//! [`crate::canvas::images::ImageJobs`]); the frame loop hands the result
//! back as [`Action::ImagePrepared`]. The bytes go to the tab's
//! [`reasoncraft_core::Assets`], outside history; the document only gains
//! an [`ImageRef`], so undo snapshots stay small.

use std::time::{Duration, Instant};

use reasoncraft_core::{ImageRef, Item, NodeId};

use super::{Action, UnsavedChoice};
use crate::app::ReasonCraftApp;
use crate::canvas::images::{ImageFailure, ImageResult, PreparedImage};
use crate::doc_tab::DocTab;
use crate::file_ops::{self, Modal};
use crate::i18n::t;
use crate::jobs::JobId;

/// The selected box, when it is a kind that can hold an image (not a note).
fn image_box(tab: &DocTab) -> Option<NodeId> {
    let Some(Item::Box(b)) = tab.selection else {
        return None;
    };
    holds_images(tab, b).then_some(b)
}

/// Whether box `b` exists and is a kind that can hold an image.
fn holds_images(tab: &DocTab, b: NodeId) -> bool {
    tab.doc.boxes.get(&b).is_some_and(|n| n.kind.takes_image())
}

fn has_image(tab: &DocTab, b: NodeId) -> bool {
    tab.doc.boxes.get(&b).is_some_and(|n| n.image.is_some())
}

/// Whether an image action applies now; `None` for other actions.
pub(super) fn is_enabled(tab: &DocTab, action: &Action) -> Option<bool> {
    Some(match action {
        Action::InsertImage => image_box(tab).is_some(),
        Action::RemoveImage => image_box(tab).is_some_and(|b| has_image(tab, b)),
        _ => return None,
    })
}

/// Pick an image file for the active tab's selected box and start
/// preparing it; a newer insert for the same box replaces the older one.
pub(super) fn insert(app: &mut ReasonCraftApp) {
    let Some(b) = app.tabs.get(app.active).and_then(image_box) else {
        return;
    };
    let Some(path) = app.dialogs.pick_image() else {
        return;
    };
    let job = app.image_jobs.start(b, path);
    app.tabs[app.active].pending_images.insert(b, job);
}

/// Take the image out of the selected box (no step when it has none).
pub(super) fn remove(tab: &mut DocTab) {
    if let Some(b) = image_box(tab)
        && has_image(tab, b)
    {
        tab.edit(|d| d.set_image(b, None));
    }
}

/// The tab waiting for `job` to fill box `node`, if any.
fn waiting_tab(app: &ReasonCraftApp, job: JobId, node: NodeId) -> Option<usize> {
    app.tabs
        .iter()
        .position(|t| t.pending_images.get(&node) == Some(&job))
}

/// Put the image `job` prepared in box `node` of the tab waiting for it,
/// as one history step. Dropped when no tab waits for it (a newer insert
/// replaced it, or the tab closed); discarded with a note when the box is
/// gone or can no longer hold an image. A failure shows its message.
/// When a "Save" in the close prompt waited for this tab's last image, the
/// save (and the close) goes ahead now.
pub(super) fn prepared(app: &mut ReasonCraftApp, job: JobId, node: NodeId, result: ImageResult) {
    let Some(i) = waiting_tab(app, job, node) else {
        return;
    };
    app.tabs[i].pending_images.remove(&node);
    let failure = match result {
        Ok(image) => {
            place(app, i, node, image);
            None
        }
        Err(failure) => Some(failure),
    };
    let key = app.tabs[i].recovery_key;
    let save_now = matches!(app.modal, Some(Modal::Unsaved { tab, waiting: true, .. }) if tab == key)
        && app.tabs[i].pending_images.is_empty();
    if save_now {
        file_ops::resolve_unsaved(app, UnsavedChoice::Save);
    }
    if let Some(failure) = failure {
        show_failure(app, &failure);
    }
}

/// Put `image` in box `node` of tab `i`, unless the box can no longer
/// hold one (then a note says it was discarded).
fn place(app: &mut ReasonCraftApp, i: usize, node: NodeId, image: PreparedImage) {
    let tab = &mut app.tabs[i];
    if !holds_images(tab, node) {
        app.status_note = Some(t("image.discarded"));
        return;
    }
    // The frame loop waits for the tab's edit session to end; a direct
    // call still keeps the edit and the image as separate steps.
    tab.commit_session();
    // Only ever "png": the bytes were re-encoded as PNG.
    let asset = tab.assets.insert("png", image.png);
    let aspect = image.aspect;
    tab.edit(|d| d.set_image(node, Some(ImageRef::new(asset, aspect))));
}

/// An error message for `failure`, or a status note while a dialog is
/// open (a message, the export dialog or the link editor).
fn show_failure(app: &mut ReasonCraftApp, failure: &ImageFailure) {
    if app.dialog_open() {
        app.status_note = Some(format!("{}: {}", failure.title(), failure.message()));
        return;
    }
    app.modal = Some(Modal::Error {
        title: failure.title(),
        message: failure.message(),
    });
}

/// Picks up the images that finished preparing and queues each as
/// [`Action::ImagePrepared`]. A result for a tab with a running edit
/// session waits until the session ends; one nobody waits for is dropped.
pub fn poll_images(app: &mut ReasonCraftApp) {
    let mut done = std::mem::take(&mut app.image_jobs.waiting);
    done.extend(app.image_jobs.finished());
    for p in done {
        let Some(i) = waiting_tab(app, p.job, p.node) else {
            continue;
        };
        if app.tabs[i].session().is_some() {
            app.image_jobs.waiting.push(p);
        } else {
            app.pending.push(Action::ImagePrepared {
                job: p.job,
                node: p.node,
                result: p.result,
            });
        }
    }
}

/// Applies the image results for tab `i` that are ready now, also those
/// held for its edit session: a close commits the session first, so a
/// finished image is never lost with the tab. Other results stay queued.
pub(super) fn flush_tab(app: &mut ReasonCraftApp, i: usize) {
    let mut done = std::mem::take(&mut app.image_jobs.waiting);
    done.extend(app.image_jobs.finished());
    for p in done {
        if waiting_tab(app, p.job, p.node) == Some(i) {
            prepared(app, p.job, p.node, p.result);
        } else {
            app.image_jobs.waiting.push(p);
        }
    }
}

/// "Save" in the close prompt: applies the tab's finished images, and when
/// some are still being prepared, keeps the prompt open in its waiting
/// state and returns true; [`prepared`] saves once the last one arrives.
pub(super) fn wait_before_save(app: &mut ReasonCraftApp) -> bool {
    let Some(Modal::Unsaved { tab, .. }) = app.modal else {
        return false;
    };
    let Some(i) = file_ops::tab_index(app, tab) else {
        return false;
    };
    flush_tab(app, i);
    if app.tabs[i].pending_images.is_empty() {
        return false;
    }
    if let Some(Modal::Unsaved { waiting, .. }) = &mut app.modal {
        *waiting = true;
    }
    true
}

/// Waits for every image being prepared and applies the results: for
/// tests and scripted runs, which have no frame loop.
pub fn wait_for_images(app: &mut ReasonCraftApp) {
    let start = Instant::now();
    loop {
        poll_images(app);
        if !app.image_jobs.busy() {
            break;
        }
        assert!(
            start.elapsed() < Duration::from_secs(60),
            "image preparation hangs"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
    for action in std::mem::take(&mut app.pending) {
        super::apply(app, action);
    }
}
