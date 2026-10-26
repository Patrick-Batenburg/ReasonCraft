//! The spelling panel (F7, Home → Spelling): docked above the Text panel,
//! non-modal. It shows the current misspelling in context, up to six
//! suggestions and a field with the replacement, and the buttons Change,
//! Change all, Ignore, Ignore all, Add to dictionary and Close.

use eframe::egui::{
    Button, FontId, Frame, Id, Margin, RichText, Role, Stroke, TextEdit, Ui, WidgetInfo,
    text::{LayoutJob, TextFormat},
};

use reasoncraft_core::FindHit;

use crate::actions::Action;
use crate::doc_tab::DocTab;
use crate::i18n::t;
use crate::spell::{SpellPass, SpellService, UNDERLINE_WIDTH, context};
use crate::theme::Tokens;

/// Show `tab`'s spelling pass, if one is open.
pub fn show(
    ui: &mut Ui,
    tab: &mut DocTab,
    spell: &mut SpellService,
    tk: &Tokens,
    out: &mut Vec<Action>,
) {
    let lang = tab.doc.meta.language.clone();
    let Some(pass) = &tab.spelling else {
        return;
    };
    // The context, read before the pass is borrowed for its field.
    let shown = pass.current.as_ref().and_then(|h| {
        let text = &tab.doc.boxes.get(&h.node)?.text;
        text.get(h.range.clone())?;
        Some(context(text, h.range.clone()))
    });
    let field_id = tab.widget_id("spelling_replacement");
    let Some(pass) = &mut tab.spelling else {
        return;
    };
    Frame::new()
        .fill(tk.box_fill)
        .stroke(Stroke::new(1.0, tk.attachment_line))
        .corner_radius(6.0)
        .inner_margin(Margin::same(8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(t("spell.title")).strong().color(tk.box_text));
            let note = |ui: &mut Ui, text: String| {
                ui.label(RichText::new(text).color(tk.text_muted));
            };
            let shown = shown.filter(|_| !pass.done);
            if pass.done {
                note(ui, t("spell.done"));
            } else if let Some(error) = spell.failure(&lang) {
                note(ui, error.to_owned());
            } else if pass.current.is_none() {
                note(ui, t("spell.loading"));
            }
            if let Some(shown) = &shown {
                details(ui, pass, shown, field_id, tk);
            }
            ui.horizontal_wrapped(|ui| {
                if let (Some((_, word, _)), Some(hit)) = (shown, pass.current.clone()) {
                    let with = pass.replacement.clone();
                    buttons(ui, hit, word, with, out);
                }
                if ui.button(t("spell.close")).clicked() {
                    out.push(Action::CloseSpelling);
                }
            });
        });
    ui.add_space(8.0);
}

/// The buttons for the misspelling `hit` of `word`, but Close.
fn buttons(ui: &mut Ui, hit: FindHit, word: String, with: String, out: &mut Vec<Action>) {
    if ui.add(Button::new(t("spell.change"))).clicked() {
        out.push(Action::SpellChange {
            hit,
            with: with.clone(),
        });
    }
    if ui.add(Button::new(t("spell.change_all"))).clicked() {
        out.push(Action::SpellChangeAll {
            word: word.clone(),
            with,
        });
    }
    if ui.add(Button::new(t("spell.ignore"))).clicked() {
        out.push(Action::SpellIgnore);
    }
    if ui.add(Button::new(t("spell.ignore_all"))).clicked() {
        out.push(Action::SpellIgnoreAll(word.clone()));
    }
    if ui.add(Button::new(t("spell.add"))).clicked() {
        out.push(Action::SpellAddWord(word));
    }
}

/// The word in context, the suggestions (a click picks one) and the
/// replacement field.
fn details(
    ui: &mut Ui,
    pass: &mut SpellPass,
    (head, word, tail): &(String, String, String),
    field_id: Id,
    tk: &Tokens,
) {
    let mut job = LayoutJob::default();
    let plain = TextFormat::simple(FontId::proportional(13.0), tk.text_muted);
    job.append(head, 0.0, plain.clone());
    let marked = TextFormat {
        color: tk.box_text,
        underline: Stroke::new(UNDERLINE_WIDTH, tk.spell_error),
        ..plain.clone()
    };
    job.append(word, 0.0, marked);
    job.append(tail, 0.0, plain);
    job.wrap.max_width = ui.available_width();
    ui.label(job);

    if pass.suggestions.is_empty() {
        ui.label(RichText::new(t("spell.no_suggestions")).color(tk.text_muted));
    } else {
        ui.horizontal_wrapped(|ui| {
            for s in &pass.suggestions {
                if ui
                    .selectable_label(pass.replacement == *s, s.as_str())
                    .clicked()
                {
                    pass.replacement = s.clone();
                }
            }
        });
    }
    let label = t("spell.change_to");
    let resp = ui.add(
        TextEdit::singleline(&mut pass.replacement)
            .id(field_id)
            .hint_text(&label)
            .desired_width(f32::INFINITY),
    );
    resp.widget_info(|| WidgetInfo::labeled(Role::TextInput, true, &label));
}
