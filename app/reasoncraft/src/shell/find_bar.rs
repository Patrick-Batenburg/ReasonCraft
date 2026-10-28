//! The find bar above the canvas (Ctrl+F; Ctrl+H adds the replacement
//! row): the query, match case and whole word, the hit counter, Previous
//! and Next, and in replace mode the replacement, Replace and Replace all.
//!
//! In the query field Enter goes to the next hit and Shift+Enter to the
//! previous one; in the replacement field Enter replaces. Esc closes the
//! bar and gives the keys back to the map.

use eframe::egui::{
    self, Button, Frame, Id, Key, Margin, Modifiers, RichText, Role, TextEdit, Ui, WidgetInfo,
};
use reasoncraft_core::FindOptions;

use super::icon_button;
use super::icons::Icon;
use crate::a11y::labelled;
use crate::actions::Action;
use crate::app::ReasonCraftApp;
use crate::doc_tab::DocTab;
use crate::i18n::{FluentArgs, t, t_args};
use crate::theme::Tokens;

const FIELD_W: f32 = 240.0;

/// Show the active tab's find bar, if it is open.
pub fn show(ui: &mut Ui, app: &mut ReasonCraftApp) {
    let tk = app.tokens;
    let tab = &mut app.tabs[app.active];
    if !tab.find.open {
        return;
    }
    let mut out = Vec::new();
    egui::Panel::top("find_bar")
        .frame(
            Frame::new()
                .fill(tk.panel)
                .inner_margin(Margin::symmetric(10, 4)),
        )
        .show(ui, |ui| bar(ui, tab, &tk, &mut out));
    app.pending.extend(out);
}

/// The keys the bar's fields take for themselves this frame.
#[derive(Default)]
struct Keys {
    enter: bool,
    shift_enter: bool,
    esc: bool,
}

/// Take Enter, Shift+Enter and Esc before `field` sees them (a
/// single-line field gives up its focus on Enter).
fn take_keys(ui: &Ui, field: Id) -> Keys {
    let mut k = Keys::default();
    if ui.memory(|m| m.has_focus(field)) {
        ui.input_mut(|i| {
            // Shift first: the plain pattern also matches Shift+Enter.
            k.shift_enter = i.consume_key(Modifiers::SHIFT, Key::Enter);
            k.enter = i.consume_key(Modifiers::NONE, Key::Enter);
            k.esc = i.consume_key(Modifiers::NONE, Key::Escape);
        });
    }
    k
}

fn bar(ui: &mut Ui, tab: &mut DocTab, tk: &Tokens, out: &mut Vec<Action>) {
    let query_id = tab.widget_id("find_query");
    let replace_id = tab.widget_id("find_replacement");
    let in_query = take_keys(ui, query_id);
    let in_replace = take_keys(ui, replace_id);
    if in_query.esc || in_replace.esc {
        ui.memory_mut(|m| m.surrender_focus(query_id));
        ui.memory_mut(|m| m.surrender_focus(replace_id));
        out.push(Action::CloseFind);
        return;
    }
    if in_query.enter {
        out.push(Action::FindNext);
    }
    if in_query.shift_enter || in_replace.shift_enter {
        out.push(Action::FindPrev);
    }
    if in_replace.enter {
        out.push(Action::ReplaceCurrent);
    }
    if std::mem::take(&mut tab.find.focus) {
        ui.memory_mut(|m| m.request_focus(query_id));
    }

    let total = tab.find_hits().len();
    let position = tab.find_position();
    let found = total > 0;
    ui.horizontal(|ui| {
        let hint = t("find.query");
        let mut query = tab.find.query.clone();
        let resp = ui.add(
            TextEdit::singleline(&mut query)
                .id(query_id)
                .hint_text(&hint)
                .desired_width(FIELD_W),
        );
        resp.widget_info(|| WidgetInfo::labeled(Role::TextInput, true, &hint));
        if resp.changed() {
            out.push(Action::SetFindQuery(query));
        }
        let opts = tab.find.opts;
        // The glyph (Aa, W) shows; the tooltip text is the name.
        let toggle = |ui: &mut Ui, on: bool, key: &str, tip: &str| {
            let resp = ui.add(Button::selectable(on, t(key)));
            labelled(resp, t(tip)).on_hover_text(t(tip)).clicked()
        };
        if toggle(
            ui,
            opts.match_case,
            "find.match_case",
            "find.match_case.tip",
        ) {
            out.push(Action::SetFindOptions(FindOptions {
                match_case: !opts.match_case,
                ..opts
            }));
        }
        if toggle(
            ui,
            opts.whole_word,
            "find.whole_word",
            "find.whole_word.tip",
        ) {
            out.push(Action::SetFindOptions(FindOptions {
                whole_word: !opts.whole_word,
                ..opts
            }));
        }
        // The count is read out as it changes.
        crate::a11y::live_region(ui, "find_count", |ui| {
            if !tab.find.query.is_empty() {
                let text = if found {
                    let mut args = FluentArgs::new();
                    args.set("current", position.map_or(0, |i| i + 1));
                    args.set("total", total);
                    t_args("find.count", &args)
                } else {
                    t("find.none")
                };
                ui.label(RichText::new(text).size(12.0).color(tk.text_muted));
            }
        });
        if ui.add_enabled(found, Button::new(t("find.prev"))).clicked() {
            out.push(Action::FindPrev);
        }
        if ui.add_enabled(found, Button::new(t("find.next"))).clicked() {
            out.push(Action::FindNext);
        }
        if icon_button(ui, Icon::Close, &t("find.close"), 22.0, true, tk, tk.panel).clicked() {
            out.push(Action::CloseFind);
        }
    });
    if !tab.find.replace {
        return;
    }
    ui.horizontal(|ui| {
        let hint = t("find.replacement");
        let mut text = tab.find.replacement.clone();
        let resp = ui.add(
            TextEdit::singleline(&mut text)
                .id(replace_id)
                .hint_text(&hint)
                .desired_width(FIELD_W),
        );
        resp.widget_info(|| WidgetInfo::labeled(Role::TextInput, true, &hint));
        if resp.changed() {
            out.push(Action::SetReplaceText(text));
        }
        if ui
            .add_enabled(found, Button::new(t("find.replace")))
            .clicked()
        {
            out.push(Action::ReplaceCurrent);
        }
        if ui
            .add_enabled(found, Button::new(t("find.replace_all")))
            .clicked()
        {
            out.push(Action::ReplaceAll);
        }
    });
}
