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
use super::images::content_rect;
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
/// as [`Action::EditText`] for this box; losing focus (a click elsewhere) commits.
pub(super) fn show(
    ui: &mut egui::Ui,
    tab: &mut DocTab,
    layout: &Layout,
    tk: &Tokens,
    pending: &mut Vec<Action>,
) {
    let Some(b) = tab.editing() else { return };
    let (Some(&world), Some(node)) = (layout.boxes.get(&b), tab.doc.boxes.get(&b)) else {
        return;
    };
    // The text sits below the box's image, if it has one.
    let world = content_rect(world, node);
    let mut area = world.shrink(BOX_TEXT_PAD);
    let hint = match &node.kind {
        BoxKind::Source { source_type } => {
            area.min.y = world.top() + SOURCE_HEADER;
            t(&format!("source.prompt.{}", source_type.key()))
        }
        _ => t("editor.hint"),
    };
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
        .hint_text(hint)
        .frame(Frame::NONE)
        .margin(Margin::ZERO)
        .desired_width(rect.width())
        .desired_rows(1)
        .min_size(rect.size())
        .align(Align2::CENTER_CENTER)
        .lock_focus(true);
    let response = ui.put(rect, editor);
    if response.changed() {
        pending.push(Action::EditText {
            node: b,
            text: tab.edit_text.clone(),
        });
    }
    if response.lost_focus() {
        pending.push(Action::StopEditing);
    }
}

#[cfg(test)]
mod tests {
    use eframe::egui;
    use reasoncraft_core::layout::{BOX_TEXT_PAD, LayoutParams};
    use reasoncraft_core::{BoxKind, SourceType};

    use crate::canvas::measure::galley;
    use crate::i18n::t;

    /// An empty source has one row for text, so its prompt must fit on one
    /// row of a default-width source, or it would spill out of the box.
    #[test]
    fn source_prompts_fit_one_row() {
        let ctx = egui::Context::default();
        let width = LayoutParams::default().source_width - 2.0 * BOX_TEXT_PAD;
        let mut wrapped = Vec::new();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            for st in SourceType::ALL {
                let prompt = t(&format!("source.prompt.{}", st.key()));
                let kind = BoxKind::Source {
                    source_type: st.clone(),
                };
                if galley(ui.ctx(), &prompt, width, &kind).rows.len() > 1 {
                    let w = galley(ui.ctx(), &prompt, f32::INFINITY, &kind).size().x;
                    wrapped.push(format!("`{prompt}` ({w:.0} px)"));
                }
            }
        });
        out.textures_delta.clear();
        assert!(wrapped.is_empty(), "wider than {width} px: {wrapped:?}");
    }
}
