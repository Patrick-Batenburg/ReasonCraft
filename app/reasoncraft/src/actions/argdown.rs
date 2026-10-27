//! File → Import Argdown…: read an Argdown file into a new tab and report
//! what the map could not keep as written.
//!
//! The tab is untitled and unsaved: the file it came from is never
//! written, and only Save As gives the map a file of its own.

use std::path::Path;

use reasoncraft_core::{
    ArgdownError, ImportNote, ImportNoteKind, argdown::MAX_ARGDOWN_BYTES, from_argdown,
};

use crate::app::ReasonCraftApp;
use crate::doc_tab::DocTab;
use crate::file_ops::Modal;
use crate::i18n::{FluentArgs, t, t_args};

/// Why a file could not be imported.
#[derive(Debug)]
pub enum ImportError {
    Io(std::io::Error),
    Argdown(ArgdownError),
}

impl ImportError {
    /// The explanation shown to the user.
    pub fn message(&self) -> String {
        let mut args = FluentArgs::new();
        let key = match self {
            ImportError::Io(e) => {
                args.set("detail", e.to_string());
                "argdown.error.io"
            }
            ImportError::Argdown(ArgdownError::TooLarge) => "argdown.error.too_large",
            ImportError::Argdown(ArgdownError::TooManyBoxes) => "argdown.error.too_many_boxes",
            ImportError::Argdown(ArgdownError::TooDeep { line }) => {
                args.set("line", *line);
                "argdown.error.too_deep"
            }
            ImportError::Argdown(ArgdownError::Empty) => "argdown.error.empty",
        };
        t_args(key, &args)
    }
}

/// Ask for an Argdown file and import it.
pub fn import_with_dialog(app: &mut ReasonCraftApp) {
    if let Some(path) = app.dialogs.pick_import() {
        import_path(app, &path);
    }
}

/// Import `path` into a new tab, which becomes active; show the report
/// when the import changed anything, or why it failed (no tab changes).
pub fn import_path(app: &mut ReasonCraftApp, path: &Path) {
    let (mut doc, report) = match read(path) {
        Ok(result) => result,
        Err(err) => {
            let mut args = FluentArgs::new();
            let file = path.file_name().unwrap_or(path.as_os_str());
            args.set("file", file.to_string_lossy().into_owned());
            app.modal = Some(Modal::Error {
                title: t_args("error.import.title", &args),
                message: err.message(),
            });
            return;
        }
    };
    if doc.meta.title.trim().is_empty() {
        let stem = path.file_stem().unwrap_or_default();
        doc.meta.title = stem.to_string_lossy().into_owned();
    }
    let mut tab = DocTab::from_document(doc);
    // Never saved: closing it asks first, and autosave keeps it.
    tab.history.saved_marker = None;
    app.tabs.push(tab);
    app.active = app.tabs.len() - 1;
    if !report.notes.is_empty() {
        app.modal = Some(Modal::ImportReport(report.notes));
    }
}

fn read(
    path: &Path,
) -> Result<(reasoncraft_core::Document, reasoncraft_core::ImportReport), ImportError> {
    let size = std::fs::metadata(path).map_err(ImportError::Io)?.len();
    if size > MAX_ARGDOWN_BYTES as u64 {
        return Err(ImportError::Argdown(ArgdownError::TooLarge));
    }
    let bytes = std::fs::read(path).map_err(ImportError::Io)?;
    from_argdown(&String::from_utf8_lossy(&bytes)).map_err(ImportError::Argdown)
}

/// The text of one note kind.
pub fn note_text(kind: &ImportNoteKind) -> String {
    let mut args = FluentArgs::new();
    match kind {
        ImportNoteKind::UnknownReference(title) => args.set("title", title.clone()),
        ImportNoteKind::TextCut => args.set("max", reasoncraft_core::MAX_BOX_CHARS),
        _ => {}
    }
    t_args(&format!("argdown.note.{}", kind.key()), &args)
}

/// How many line numbers the report lists per kind of note.
const LINES_SHOWN: usize = 5;

/// The report's body: an introduction and one line per kind of note, with
/// how often it happened and the first few line numbers, so a long report
/// stays short.
pub fn report_text(notes: &[ImportNote]) -> String {
    let mut groups: Vec<(&ImportNoteKind, Vec<usize>)> = Vec::new();
    for note in notes {
        match groups.iter_mut().find(|(k, _)| **k == note.kind) {
            Some((_, lines)) => lines.push(note.line),
            None => groups.push((&note.kind, vec![note.line])),
        }
    }
    let mut body = t("argdown.report.message");
    for (kind, lines) in groups {
        let shown: Vec<String> = lines
            .iter()
            .take(LINES_SHOWN)
            .map(usize::to_string)
            .collect();
        let mut shown = shown.join(", ");
        if lines.len() > LINES_SHOWN {
            let mut args = FluentArgs::new();
            args.set("lines", shown);
            args.set("more", lines.len() - LINES_SHOWN);
            shown = t_args("argdown.report.more", &args);
        }
        let mut args = FluentArgs::new();
        args.set("text", note_text(kind));
        args.set("count", lines.len());
        args.set("lines", shown);
        body.push_str("\n• ");
        body.push_str(&t_args("argdown.report.group", &args));
    }
    body
}

#[cfg(test)]
mod tests;
