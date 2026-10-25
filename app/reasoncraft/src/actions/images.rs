//! Putting an image in the selected box, and taking it out again.
//!
//! The bytes go to the tab's [`reasoncraft_core::Assets`], outside history;
//! the document only gains an [`ImageRef`], so undo snapshots stay small.

use std::path::Path;

use reasoncraft_core::{BoxKind, ImageRef, Item, NodeId};

use super::Action;
use crate::app::ReasonCraftApp;
use crate::canvas::images::{ImageError, MAX_INPUT_BYTES, prepare_image};
use crate::doc_tab::DocTab;
use crate::file_ops::Modal;
use crate::i18n::{FluentArgs, t, t_args};

/// The selected box, when it is a kind that can hold an image (not a note).
fn image_box(tab: &DocTab) -> Option<NodeId> {
    let Some(Item::Box(b)) = tab.selection else {
        return None;
    };
    let node = tab.doc.boxes.get(&b)?;
    (node.kind != BoxKind::Note).then_some(b)
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

/// Why a picked file could not be used.
enum Failure {
    Read(std::io::Error),
    Image(ImageError),
}

/// Read and prepare the image at `path`. A file over the size limit is
/// refused without reading it.
fn load(path: &Path) -> Result<(Vec<u8>, f32), Failure> {
    let len = std::fs::metadata(path).map_err(Failure::Read)?.len();
    if len > MAX_INPUT_BYTES {
        return Err(Failure::Image(ImageError::TooLarge));
    }
    let bytes = std::fs::read(path).map_err(Failure::Read)?;
    prepare_image(&bytes).map_err(Failure::Image)
}

/// Carry out an image action.
pub(super) fn apply(app: &mut ReasonCraftApp, action: Action) {
    let Some(tab) = app.tabs.get(app.active) else {
        return;
    };
    let Some(b) = image_box(tab) else {
        return;
    };
    match action {
        Action::InsertImage => {
            let Some(path) = app.dialogs.pick_image() else {
                return;
            };
            match load(&path) {
                Ok((png, aspect)) => {
                    let tab = &mut app.tabs[app.active];
                    // Only ever "png": the bytes were re-encoded as PNG.
                    let asset = tab.assets.insert("png", png);
                    tab.edit(|d| d.set_image(b, Some(ImageRef { asset, aspect })));
                }
                Err(failure) => {
                    let mut args = FluentArgs::new();
                    let file = path.file_name().unwrap_or(path.as_os_str());
                    args.set("file", file.to_string_lossy().into_owned());
                    let message = match failure {
                        Failure::Image(e) => t(e.key()),
                        Failure::Read(e) => {
                            args.set("detail", e.to_string());
                            t_args("image.error.read", &args)
                        }
                    };
                    app.modal = Some(Modal::Error {
                        title: t_args("image.error.title", &args),
                        message,
                    });
                }
            }
        }
        Action::RemoveImage => {
            let tab = &mut app.tabs[app.active];
            if has_image(tab, b) {
                tab.edit(|d| d.set_image(b, None));
            }
        }
        _ => unreachable!("not an image action"),
    }
}
