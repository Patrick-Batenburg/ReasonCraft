//! One snippet in the Snippets tab: drag handle, delete button, text,
//! "Drag to map" chip (while text is selected) and source line (type,
//! citation, link).

use eframe::egui::{
    self, Button, CursorIcon, Frame, Id, Margin, Popup, Rect, Response, RichText, Role, ScrollArea,
    Sense, Stroke, TextEdit, Ui, WidgetInfo,
    text::{CCursor, CCursorRange},
    text_edit::TextEditState,
};
use reasoncraft_core::{MAX_SNIPPET_CHARS, Snippet, SnippetSource, SourceType};

use super::drag_chip;
use crate::actions::Action;
use crate::canvas::plus_menu::source_button;
use crate::doc_tab::DocTab;
use crate::i18n::{FluentArgs, t, t_args};
use crate::shell::icon_button;
use crate::shell::icons::Icon;
use crate::shell::link_modal::{normalize_link, visible_link};
use crate::shell::source_icons::source_atom;
use crate::spell::{FieldStyle, SpellFields};
use crate::theme::Tokens;

/// The tallest a snippet's text gets before it scrolls inside the card.
const TEXT_MAX_H: f32 = 220.0;
const HANDLE: egui::Vec2 = egui::vec2(14.0, 20.0);

/// The widget id of `tab`'s text field for `snippet`.
pub fn text_id(tab: &DocTab, snippet: &Snippet) -> Id {
    tab.widget_id(("snippet_text", snippet.id))
}

/// What a drawn card reports back to the list.
pub(super) struct Card {
    pub rect: Rect,
    pub handle: Response,
    /// The handle is carrying the whole snippet to the map (Alt+drag),
    /// not reordering it.
    pub to_map: bool,
}

/// How a card shows this frame.
pub(super) struct State {
    /// Give its text field keyboard focus, with the cursor at the end.
    pub focus: bool,
    /// Show the length-limit notice.
    pub capped: bool,
}

/// Draw one snippet card, misspellings in its text underlined.
pub(super) fn show(
    ui: &mut Ui,
    tab: &DocTab,
    snippet: &Snippet,
    state: State,
    tk: &Tokens,
    spell: &mut SpellFields,
    out: &mut Vec<Action>,
) -> Card {
    let State { focus, capped } = state;
    let id = snippet.id;
    let frame = Frame::new()
        .fill(tk.box_fill)
        .stroke(Stroke::new(1.0, tk.attachment_line.gamma_multiply(0.5)))
        .corner_radius(6.0)
        .inner_margin(Margin::same(6));
    let mut handle = None;
    let resp = frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            handle = Some(drag_handle(ui, tk));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let label = t("snippet.delete");
                if icon_button(ui, Icon::Delete, &label, 20.0, true, tk, tk.box_fill).clicked() {
                    out.push(Action::RemoveSnippet(id));
                }
            });
        });
        let (focused, selection) = text_field(ui, tab, snippet, focus, spell, out);
        drag_chip::chip(ui, snippet, focused, selection, tk);
        if capped {
            ui.label(RichText::new(capped_notice()).color(tk.objection));
        }
        source_line(ui, tab, snippet, tk, out);
    });
    let handle = handle.expect("drawn");
    Card {
        rect: resp.response.rect,
        to_map: drag_chip::whole_snippet_drag(ui, &handle, snippet),
        handle,
    }
}

/// The notice under a snippet whose text was cut at the length limit.
pub fn capped_notice() -> String {
    let mut args = FluentArgs::new();
    args.set("max", MAX_SNIPPET_CHARS);
    t_args("snippet.capped", &args)
}

/// The ⋮⋮ grip, painted as two columns of dots.
fn drag_handle(ui: &mut Ui, tk: &Tokens) -> Response {
    let (rect, resp) = ui.allocate_exact_size(HANDLE, Sense::drag());
    let label = t("snippet.drag");
    resp.widget_info(|| WidgetInfo::labeled(Role::Button, true, &label));
    let colour = if resp.hovered() || resp.dragged() {
        tk.box_text
    } else {
        tk.text_muted
    };
    for col in [-2.5, 2.5] {
        for row in [-5.0, 0.0, 5.0] {
            let at = rect.center() + egui::vec2(col, row);
            ui.painter().circle_filled(at, 1.4, colour);
        }
    }
    if resp.dragged() {
        ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
    } else if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::Grab);
    }
    resp.on_hover_text(label)
}

/// The snippet's text, scrolling inside the card when long. Changes go out
/// live; losing focus ends the editing session. Returns whether the field
/// has focus, and then the selected text (see [`drag_chip::selected_text`]).
fn text_field(
    ui: &mut Ui,
    tab: &DocTab,
    snippet: &Snippet,
    focus: bool,
    spell: &mut SpellFields,
    out: &mut Vec<Action>,
) -> (bool, Option<String>) {
    let id = text_id(tab, snippet);
    if focus {
        let mut state = TextEditState::load(ui.ctx(), id).unwrap_or_default();
        let end = CCursor::new(snippet.text.chars().count());
        state.cursor.set_char_range(Some(CCursorRange::one(end)));
        state.store(ui.ctx(), id);
        ui.memory_mut(|m| m.request_focus(id));
    }
    let mut text = snippet.text.clone();
    let output = ScrollArea::vertical()
        .id_salt(("snippet_scroll", snippet.id))
        .max_height(TEXT_MAX_H)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            let mut layouter = spell.layouter(FieldStyle::default());
            let edit = TextEdit::multiline(&mut text)
                .id(id)
                .layouter(&mut layouter)
                .hint_text(t("snippet.hint"))
                .desired_width(f32::INFINITY)
                .desired_rows(2);
            edit.show(ui)
        })
        .inner;
    let resp = output.response.response;
    if focus {
        // A new snippet at the end of a long list comes into view.
        resp.scroll_to_me(None);
    }
    let label = t("snippet.text");
    resp.widget_info(|| WidgetInfo::labeled(Role::MultilineTextInput, true, &label));
    if resp.changed() {
        out.push(Action::EditSnippetText {
            id: snippet.id,
            text,
        });
    }
    if resp.lost_focus() {
        out.push(Action::StopEditing);
    }
    let focused = resp.has_focus();
    let selection = output
        .cursor_range
        .filter(|_| focused)
        .and_then(|r| drag_chip::selected_text(&snippet.text, r));
    (focused, selection)
}

/// Type picker and citation on one row, the link below it. Citation and
/// link typing shows live, in the snippet's editing session; a typed link
/// that is not valid is not stored and shows a hint while its field has focus.
fn source_line(ui: &mut Ui, tab: &DocTab, snippet: &Snippet, tk: &Tokens, out: &mut Vec<Action>) {
    let id = snippet.id;
    let current = snippet
        .source
        .clone()
        .unwrap_or_else(|| SnippetSource::new(SourceType::Source, "", None));
    ui.horizontal(|ui| {
        let st = &current.source_type;
        let name = t(&format!("source.{}", st.key()));
        let picker = ui.add(Button::new(source_atom(st, 16.0)));
        let label = t("snippet.source_type");
        picker.widget_info(|| WidgetInfo::labeled(Role::Button, true, &label));
        let picker = picker.on_hover_text(name);
        Popup::menu(&picker).show(|ui| {
            for st in SourceType::ALL {
                let selected = st == current.source_type;
                if ui.add(source_button(&st).selected(selected)).clicked() && !selected {
                    let source = SnippetSource {
                        source_type: st,
                        ..current.clone()
                    };
                    out.push(Action::SetSnippetSource {
                        id,
                        source: Some(source),
                    });
                }
            }
        });
        let mut citation = current.citation.clone();
        let field = tab.widget_id(("snippet_citation", id));
        let resp = line_field(ui, field, &mut citation, &t("snippet.citation"));
        if resp.changed() {
            out.push(Action::EditSnippetCitation { id, citation });
        }
        if resp.lost_focus() {
            out.push(Action::StopEditing);
        }
    });
    // While the field has focus it shows the typed text (kept in temp
    // memory, since an invalid link is not stored); otherwise the stored link.
    let field = tab.widget_id(("snippet_link", id));
    let stored = current
        .link
        .as_deref()
        .map(visible_link)
        .unwrap_or_default();
    let mut text = if ui.memory(|m| m.has_focus(field)) {
        ui.data(|d| d.get_temp::<String>(field)).unwrap_or(stored)
    } else {
        stored
    };
    let resp = line_field(ui, field, &mut text, &t("snippet.link"));
    let invalid = resp.has_focus() && normalize_link(&text).is_err();
    if resp.changed() {
        out.push(Action::EditSnippetLink {
            id,
            link: text.clone(),
        });
    }
    if resp.has_focus() {
        ui.data_mut(|d| d.insert_temp(field, text));
    } else {
        ui.data_mut(|d| d.remove::<String>(field));
    }
    if resp.lost_focus() {
        out.push(Action::StopEditing);
    }
    if invalid {
        ui.label(RichText::new(t("link.invalid")).small().color(tk.objection));
    }
}

/// A one-line field for `text`, with `label` as its hint and accessible name.
fn line_field(ui: &mut Ui, id: Id, text: &mut String, label: &str) -> Response {
    let edit = TextEdit::singleline(text)
        .id(id)
        .hint_text(label)
        .desired_width(ui.available_width());
    let resp = ui.add(edit);
    resp.widget_info(|| WidgetInfo::labeled(Role::TextInput, true, label));
    resp
}
