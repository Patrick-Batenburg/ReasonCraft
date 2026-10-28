//! Opening example maps, templates and recent files.

use reasoncraft_core::examples::{example, template};

use super::Action;
use crate::app::ReasonCraftApp;
use crate::doc_tab::DocTab;
use crate::file_ops;
use crate::i18n::ui_language;

/// Open an example (in the UI language: it is read alongside the
/// interface and its help), a template (in the language of new documents,
/// like a blank map: the user writes it) or a recent file.
pub(super) fn apply(app: &mut ReasonCraftApp, action: Action) {
    let doc = match action {
        Action::OpenExample(which) => example(which, ui_language()),
        Action::OpenTemplate(which) => {
            template(which, app.settings.new_doc_language(app.os_locale))
        }
        Action::OpenRecent(path) => return file_ops::open_recent(app, &path),
        _ => unreachable!("not an example action"),
    };
    // Clean and untitled: closing it asks nothing, autosave leaves it
    // alone, and Save asks where to put it.
    file_ops::place_tab(app, DocTab::from_document(doc));
}
