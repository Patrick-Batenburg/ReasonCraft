//! The backstage Document page: title, author and language of the open
//! document.

use eframe::egui::{self, RichText, TextEdit};

use crate::actions::Action;
use crate::app::ReasonCraftApp;
use crate::i18n::t;
use reasoncraft_core::meta::LANGUAGES;

/// The picker for a language: one choice per language, the current one
/// selected. Returns the language picked this frame, if it is a change.
pub(super) fn language_picker(ui: &mut egui::Ui, current: &str) -> Option<String> {
    let mut picked = None;
    ui.horizontal(|ui| {
        for lang in LANGUAGES {
            let on = current == lang;
            if ui
                .selectable_label(on, t(&format!("lang.{lang}")))
                .clicked()
                && !on
            {
                picked = Some(lang.to_owned());
            }
        }
    });
    picked
}

/// Title and author fields (live while typing, one history step when the
/// session ends) and the language picker.
pub(super) fn page(ui: &mut egui::Ui, app: &mut ReasonCraftApp) {
    let tk = app.tokens;
    ui.spacing_mut().item_spacing.y = 8.0;
    ui.label(
        RichText::new(t("document.title"))
            .strong()
            .size(18.0)
            .color(tk.box_text),
    );
    ui.add_space(8.0);
    let tab = app.active_tab();
    let ids = [
        tab.widget_id("document_title"),
        tab.widget_id("document_author"),
    ];
    let meta = &tab.doc.meta;
    let (mut title, mut author) = (meta.title.clone(), meta.author.clone());
    let current_language = meta.language.clone();
    let mut changed = false;
    for ((key, text), id) in [
        ("document.title_field", &mut title),
        ("document.author_field", &mut author),
    ]
    .into_iter()
    .zip(ids)
    {
        let label = ui.label(RichText::new(t(key)).color(tk.box_text));
        let response = ui
            .add(TextEdit::singleline(text).id(id).desired_width(280.0))
            .labelled_by(label.id);
        changed |= response.changed();
    }
    if changed {
        app.push(Action::SetDocMeta { title, author });
    }
    ui.add_space(8.0);
    ui.label(RichText::new(t("document.language")).color(tk.box_text));
    if let Some(lang) = language_picker(ui, &current_language) {
        app.push(Action::SetDocLanguage(lang));
    }
    ui.label(RichText::new(t("document.language_hint")).color(tk.text_muted));
}
