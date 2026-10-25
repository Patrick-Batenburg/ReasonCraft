//! Inline text editing: a text field laid over the edited box, in screen
//! space, with the box's font scaled by the zoom.

use eframe::egui::{
    self, Align2, FontId, Frame, Id, Margin, TextEdit,
    text::{CCursor, CCursorRange},
    text_edit::TextEditState,
};
use reasoncraft_core::layout::BOX_TEXT_PAD;
use reasoncraft_core::{BoxKind, Layout, NodeId};

use super::boxes::SOURCE_HEADER;
use super::measure::font_for;
use super::paint::screen_font;
use crate::actions::Action;
use crate::doc_tab::DocTab;
use crate::i18n::t;
use crate::theme::Tokens;

/// The widget id of the inline editor for box `b`.
pub fn editor_id(b: NodeId) -> Id {
    Id::unique(("inline_editor", b))
}

/// Show the editor over the box being edited, if any. Text changes go out
/// as [`Action::EditText`]; losing focus (a click elsewhere) commits.
pub(super) fn show(
    ui: &mut egui::Ui,
    tab: &mut DocTab,
    layout: &Layout,
    tk: &Tokens,
    pending: &mut Vec<Action>,
) {
    let Some(b) = tab.editing else { return };
    let (Some(&world), Some(node)) = (layout.boxes.get(&b), tab.doc.boxes.get(&b)) else {
        return;
    };
    let mut area = world.shrink(BOX_TEXT_PAD);
    if matches!(node.kind, BoxKind::Source { .. }) {
        area.min.y = world.top() + SOURCE_HEADER;
    }
    let rect = tab.camera.world_rect_to_screen(area);
    let zoom = tab.camera.zoom;
    let size = font_for(&node.kind).size;
    let font = screen_font(size, zoom).unwrap_or_else(|| FontId::proportional(size * zoom));
    let colour = if node.kind == BoxKind::Note {
        tk.note_text
    } else {
        tk.box_text
    };

    let id = editor_id(b);
    if tab.edit_focus {
        // Start with the cursor after the text (and any character just typed).
        let mut state = TextEditState::load(ui.ctx(), id).unwrap_or_default();
        let end = CCursor::new(tab.edit_text.chars().count());
        state.cursor.set_char_range(Some(CCursorRange::one(end)));
        state.store(ui.ctx(), id);
        ui.memory_mut(|m| m.request_focus(id));
        tab.edit_focus = false;
    }
    let editor = TextEdit::multiline(&mut tab.edit_text)
        .id(id)
        .font(font)
        .text_color(colour)
        .hint_text(t("editor.hint"))
        .frame(Frame::NONE)
        .margin(Margin::ZERO)
        .desired_width(rect.width())
        .desired_rows(1)
        .min_size(rect.size())
        .align(Align2::CENTER_CENTER)
        .lock_focus(true);
    let response = ui.put(rect, editor);
    if response.changed() {
        pending.push(Action::EditText(tab.edit_text.clone()));
    }
    if response.lost_focus() {
        pending.push(Action::StopEditing);
    }
}
