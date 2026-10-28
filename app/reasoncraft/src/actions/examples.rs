//! Opening example maps and templates.

use reasoncraft_core::Document;
use reasoncraft_core::examples::{Example, Template, example, template};

use crate::app::ReasonCraftApp;
use crate::doc_tab::DocTab;
use crate::file_ops;
use crate::i18n::ui_language;

/// Open an example, in the UI language: it is read alongside the
/// interface and its help.
pub(super) fn open_example(app: &mut ReasonCraftApp, which: Example) {
    place(app, example(which, ui_language()));
}

/// Open a template, in the language of new documents, like a blank map:
/// the user writes it.
pub(super) fn open_template(app: &mut ReasonCraftApp, which: Template) {
    let lang = app.settings.new_doc_language(app.os_locale);
    place(app, template(which, lang));
}

/// Open `doc` clean and without a file (the tab is named by its title):
/// closing it asks nothing, autosave leaves it alone, and Save asks where
/// to put it.
fn place(app: &mut ReasonCraftApp, doc: Document) {
    file_ops::place_tab(app, DocTab::from_document(doc));
}
