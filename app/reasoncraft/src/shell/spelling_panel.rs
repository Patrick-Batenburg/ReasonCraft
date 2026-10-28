//! The spelling panel (F7, Home → Spelling): docked above the Text panel,
//! non-modal. It shows the current misspelling in context, up to six
//! suggestions and a field with the replacement, and the buttons Change,
//! Change all, Ignore, Ignore all, Add to dictionary and Close.

use eframe::egui::{
    self, Button, FontId, Frame, Id, Margin, Rect, RichText, Role, Stroke, TextEdit, Ui,
    WidgetInfo,
    text::{LayoutJob, TextFormat},
};

use reasoncraft_core::FindHit;

use crate::a11y::labelled;
use crate::actions::Action;
use crate::doc_tab::DocTab;
use crate::i18n::{FluentArgs, t, t_args};
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
    // The context, read before the pass is borrowed for its field; only
    // while the box still holds the word at the hit.
    let shown = pass.current_word(&tab.doc).and_then(|_| {
        let h = pass.current.as_ref()?;
        Some(context(&tab.doc.boxes.get(&h.node)?.text, h.range.clone()))
    });
    let field_id = tab.widget_id("spelling_replacement");
    let Some(pass) = &mut tab.spelling else {
        return;
    };
    let shown = Frame::new()
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
            } else if spell.is_loading(&lang) {
                note(ui, t("spell.loading"));
            } else if pass.current.is_none() {
                note(ui, t("spell.checking"));
            }
            if let Some(shown) = &shown {
                details(ui, pass, shown, field_id, tk);
            }
            ui.horizontal_wrapped(|ui| {
                if let (Some(_), Some(hit)) = (&shown, pass.current.clone()) {
                    let with = pass.replacement.clone();
                    buttons(ui, hit, pass.word.clone(), with, out);
                }
                let close = ui.button(t("spell.close"));
                if labelled(close, t("spell.close_name")).clicked() {
                    out.push(Action::CloseSpelling);
                }
            });
        });
    let rect = shown.response.rect;
    ui.ctx().data_mut(|d| d.insert_temp(box_rect_id(), rect));
    ui.add_space(8.0);
}

/// Where the spelling box was drawn last (egui memory).
fn box_rect_id() -> Id {
    Id::unique("spelling_box_rect")
}

/// The control that had the keyboard focus at the end of the last frame.
fn last_focus_id() -> Id {
    Id::unique("spelling_last_focus")
}

/// Notes the focused control at the end of a frame, for [`esc_closes`]
/// (egui drops the focus as soon as Esc is pressed).
pub fn note_focus(ctx: &egui::Context) {
    let focused = ctx.memory(|m| m.focused());
    ctx.data_mut(|d| d.insert_temp(last_focus_id(), focused));
}

/// The control that had the focus at the end of last frame, as
/// [`note_focus`] noted it.
pub fn last_focus(ctx: &egui::Context) -> Option<Id> {
    ctx.data(|d| d.get_temp::<Option<Id>>(last_focus_id()))
        .flatten()
}

/// Whether Esc closes `tab`'s spelling box: it is open and, before the
/// Esc, the focus was inside it, on the map or nowhere (not in another
/// field, such as an essay paragraph, whose Esc is its own).
pub fn esc_closes(ctx: &egui::Context, tab: &DocTab) -> bool {
    if tab.spelling.is_none() {
        return false;
    }
    let Some(focus) = last_focus(ctx) else {
        return true;
    };
    if focus == crate::canvas::canvas_id() {
        return true;
    }
    let inside = |area: Rect| {
        ctx.read_response(focus)
            .is_some_and(|r| area.contains(r.rect.center()))
    };
    ctx.data(|d| d.get_temp::<Rect>(box_rect_id()))
        .is_some_and(inside)
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
    // The word is marked by colour and underline only: say it in words,
    // and read it out as the pass moves on.
    crate::a11y::live_region(ui, "spell_word", |ui| {
        let shown = ui.label(job);
        let mut args = FluentArgs::new();
        args.set("word", word.as_str());
        let said = format!(
            "{} {head}{word}{tail}",
            t_args("spell.not_in_dictionary", &args)
        );
        ui.ctx()
            .accesskit_node_builder(shown.id, |node| node.set_value(said));
    });

    match &pass.suggestions {
        None => {
            ui.label(RichText::new(t("spell.suggesting")).color(tk.text_muted));
        }
        Some(found) if found.is_empty() => {
            ui.label(RichText::new(t("spell.no_suggestions")).color(tk.text_muted));
        }
        Some(found) => {
            ui.horizontal_wrapped(|ui| {
                for s in found {
                    if ui
                        .selectable_label(pass.replacement == *s, s.as_str())
                        .clicked()
                    {
                        pass.replacement = s.clone();
                    }
                }
            });
        }
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
