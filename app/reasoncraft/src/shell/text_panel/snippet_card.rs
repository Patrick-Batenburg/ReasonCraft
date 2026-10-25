//! One snippet in the Snippets tab: drag handle, delete button, text and
//! source line (type, citation, link).

use eframe::egui::{
    self, Button, CursorIcon, Frame, Id, Margin, Popup, Rect, Response, RichText, Role, ScrollArea,
    Sense, Stroke, TextEdit, Ui, WidgetInfo,
    text::{CCursor, CCursorRange},
    text_edit::TextEditState,
};
use reasoncraft_core::{Snippet, SnippetSource, SourceType};

use crate::actions::Action;
use crate::canvas::plus_menu::source_button;
use crate::i18n::t;
use crate::shell::icon_button;
use crate::shell::icons::Icon;
use crate::shell::link_modal::{normalize_link, visible_link};
use crate::shell::source_icons::source_atom;
use crate::theme::Tokens;

/// The tallest a snippet's text gets before it scrolls inside the card.
const TEXT_MAX_H: f32 = 220.0;
const HANDLE: egui::Vec2 = egui::vec2(14.0, 20.0);

/// The widget id of snippet `id`'s text field.
pub fn text_id(snippet: &Snippet) -> Id {
    Id::unique(("snippet_text", snippet.id))
}

/// What a drawn card reports back to the list.
pub(super) struct Card {
    pub rect: Rect,
    pub handle: Response,
}

/// Draw one snippet card. `focus`: give its text field keyboard focus,
/// with the cursor at the end. `capped`: show the length-limit notice.
pub(super) fn show(
    ui: &mut Ui,
    snippet: &Snippet,
    focus: bool,
    capped: bool,
    tk: &Tokens,
    out: &mut Vec<Action>,
) -> Card {
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
        text_field(ui, snippet, focus, out);
        if capped {
            ui.label(RichText::new(t("snippet.capped")).color(tk.objection));
        }
        source_line(ui, snippet, tk, out);
    });
    Card {
        rect: resp.response.rect,
        handle: handle.expect("drawn"),
    }
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
/// live; losing focus ends the editing session.
fn text_field(ui: &mut Ui, snippet: &Snippet, focus: bool, out: &mut Vec<Action>) {
    let id = text_id(snippet);
    if focus {
        let mut state = TextEditState::load(ui.ctx(), id).unwrap_or_default();
        let end = CCursor::new(snippet.text.chars().count());
        state.cursor.set_char_range(Some(CCursorRange::one(end)));
        state.store(ui.ctx(), id);
        ui.memory_mut(|m| m.request_focus(id));
    }
    let mut text = snippet.text.clone();
    let resp = ScrollArea::vertical()
        .id_salt(("snippet_scroll", snippet.id))
        .max_height(TEXT_MAX_H)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            let edit = TextEdit::multiline(&mut text)
                .id(id)
                .hint_text(t("snippet.hint"))
                .desired_width(f32::INFINITY)
                .desired_rows(2);
            ui.add(edit)
        })
        .inner;
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
}

/// Type picker and citation on one row, the link below it.
fn source_line(ui: &mut Ui, snippet: &Snippet, tk: &Tokens, out: &mut Vec<Action>) {
    let id = snippet.id;
    let current = snippet.source.clone().unwrap_or_else(|| SnippetSource {
        source_type: SourceType::Source,
        citation: String::new(),
        link: None,
    });
    let set = |change: &dyn Fn(&mut SnippetSource)| {
        let mut s = current.clone();
        change(&mut s);
        Action::SetSnippetSource {
            id,
            source: Some(s),
        }
    };
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
                    out.push(set(&|s| s.source_type = st.clone()));
                }
            }
        });
        let field = Id::unique(("snippet_citation", id));
        let label = t("snippet.citation");
        if let Some(text) = draft_field(ui, field, &current.citation, &label) {
            out.push(set(&|s| s.citation = text.clone()));
        }
    });
    let field = Id::unique(("snippet_link", id));
    let bad_id = field.with("invalid");
    let shown = current
        .link
        .as_deref()
        .map(visible_link)
        .unwrap_or_default();
    if let Some(text) = draft_field(ui, field, &shown, &t("snippet.link")) {
        match normalize_link(&text) {
            Ok(link) => {
                ui.data_mut(|d| d.remove::<bool>(bad_id));
                out.push(set(&|s| s.link = link.clone()));
            }
            Err(_) => {
                // Keep the typed text showing, to be fixed.
                ui.data_mut(|d| {
                    d.insert_temp(field, text);
                    d.insert_temp(bad_id, true);
                });
            }
        }
    }
    if ui.data(|d| d.get_temp::<bool>(bad_id)).unwrap_or(false) {
        ui.label(RichText::new(t("link.invalid")).small().color(tk.objection));
    }
}

/// A one-line field showing `current` (with `label` as its hint and
/// accessible name). Typing is kept as a draft until the field loses
/// focus; the draft is returned then if it differs from `current`.
fn draft_field(ui: &mut Ui, id: Id, current: &str, label: &str) -> Option<String> {
    let mut text = ui
        .data(|d| d.get_temp::<String>(id))
        .unwrap_or_else(|| current.to_owned());
    let edit = TextEdit::singleline(&mut text)
        .id(id.with("field"))
        .hint_text(label)
        .desired_width(ui.available_width());
    let resp = ui.add(edit);
    resp.widget_info(|| WidgetInfo::labeled(Role::TextInput, true, label));
    if resp.changed() {
        ui.data_mut(|d| d.remove::<bool>(id.with("invalid")));
    }
    if resp.lost_focus() {
        ui.data_mut(|d| d.remove::<String>(id));
        return (text != current).then_some(text);
    }
    if resp.has_focus() {
        ui.data_mut(|d| d.insert_temp(id, text));
    }
    None
}
