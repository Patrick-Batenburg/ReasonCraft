//! Dragging snippet text onto the map. While some of a snippet's text is
//! selected, a "Drag to map" chip under it carries that text; Alt+dragging
//! the snippet's ⋮⋮ handle carries the whole text. Both drag
//! [`Insertable::Text`] with the snippet's source; the canvas shows where
//! it would land and inserts it on release.
//!
//! A text field's selection collapses when it loses focus, which pressing
//! the chip does, so the chip remembers the selection from the frames the
//! field had focus and keeps it while the chip is hovered or dragged.

use eframe::egui::{
    self, CursorIcon, DragAndDrop, FontId, Id, Response, Role, Sense, Stroke, StrokeKind, Ui,
    WidgetInfo, text::CCursorRange, vec2,
};
use reasoncraft_core::{Snippet, Uuid};

use crate::actions::Insertable;
use crate::i18n::{FluentArgs, t, t_args};
use crate::theme::Tokens;

/// How many characters of the text a chip (and the drag ghost) shows.
pub const EXCERPT_CHARS: usize = 40;
const PAD: egui::Vec2 = vec2(8.0, 3.0);
const FONT: f32 = 12.0;

/// The first [`EXCERPT_CHARS`] characters of `text` on one line (runs of
/// whitespace as one space), with "…" when there is more.
pub fn excerpt(text: &str) -> String {
    let mut flat = String::new();
    for word in text.split_whitespace() {
        if !flat.is_empty() {
            flat.push(' ');
        }
        flat.push_str(word);
        if flat.chars().count() > EXCERPT_CHARS {
            break;
        }
    }
    match flat.char_indices().nth(EXCERPT_CHARS) {
        Some((end, _)) => format!("{}…", flat[..end].trim_end()),
        None => flat,
    }
}

/// The text `range` selects in `text` (character indices, not bytes),
/// trimmed; `None` when nothing but whitespace is selected.
pub fn selected_text(text: &str, range: CCursorRange) -> Option<String> {
    let r = range.as_sorted_char_range();
    let chosen: String = text
        .chars()
        .skip(r.start.0)
        .take(r.end.0.saturating_sub(r.start.0))
        .collect();
    let chosen = chosen.trim();
    (!chosen.is_empty()).then(|| chosen.to_owned())
}

/// The chip's text and accessible name.
pub fn chip_label(text: &str) -> String {
    let mut args = FluentArgs::new();
    args.set("text", excerpt(text));
    t_args("snippet.drag_to_map", &args)
}

fn chip_id(snippet: Uuid) -> Id {
    Id::unique(("snippet_drag_chip", snippet))
}

/// The chip under `snippet`. `focused`: its text field has focus, with
/// `selection` selected (see [`selected_text`]).
pub(super) fn chip(
    ui: &mut Ui,
    snippet: &Snippet,
    focused: bool,
    selection: Option<String>,
    tk: &Tokens,
) {
    let id = chip_id(snippet.id);
    let ctx = ui.ctx().clone();
    let held = ctx.is_being_dragged(id)
        || ctx
            .read_response(id)
            .is_some_and(|r| r.hovered() || r.dragged());
    let text = if focused {
        match &selection {
            Some(s) => {
                ctx.data_mut(|d| d.insert_temp(id, s.clone()));
            }
            None => ctx.data_mut(|d| d.remove::<String>(id)),
        }
        selection
    } else if held {
        ctx.data(|d| d.get_temp::<String>(id))
    } else {
        ctx.data_mut(|d| d.remove::<String>(id));
        None
    };
    let Some(text) = text else {
        return;
    };
    let label = chip_label(&text);
    let wrap = (ui.available_width() - 2.0 * PAD.x).max(40.0);
    let galley = ui
        .painter()
        .layout(label.clone(), FontId::proportional(FONT), tk.box_text, wrap);
    let (rect, _) = ui.allocate_exact_size(galley.size() + 2.0 * PAD, Sense::hover());
    let resp = ui.interact(rect, id, Sense::drag());
    resp.widget_info(|| WidgetInfo::labeled(Role::Button, true, &label));
    let active = resp.hovered() || resp.dragged();
    let border = if active {
        tk.accent
    } else {
        tk.attachment_line
    };
    let painter = ui.painter();
    painter.rect(
        rect,
        9.0,
        tk.panel,
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );
    painter.galley(rect.min + PAD, galley, tk.box_text);
    if resp.dragged() {
        ctx.set_cursor_icon(CursorIcon::Grabbing);
    }
    resp.dnd_set_drag_payload(Insertable::Text {
        text,
        source: snippet.source.clone(),
    });
    resp.on_hover_text(t("snippet.tip.drag_to_map"));
}

fn whole_drag_id() -> Id {
    Id::unique("snippet_whole_drag")
}

/// Alt+drag on `snippet`'s handle carries its whole text to the map
/// (unless blank). Returns whether `handle` is doing that, so the list
/// does not also reorder.
pub(super) fn whole_snippet_drag(ui: &Ui, handle: &Response, snippet: &Snippet) -> bool {
    let ctx = ui.ctx();
    let key = whole_drag_id();
    if handle.drag_started() {
        let text = snippet.text.trim();
        if ui.input(|i| i.modifiers.alt) && !text.is_empty() {
            DragAndDrop::set_payload(
                ctx,
                Insertable::Text {
                    text: text.to_owned(),
                    source: snippet.source.clone(),
                },
            );
            ctx.data_mut(|d| d.insert_temp(key, snippet.id));
        } else if ctx.data(|d| d.get_temp::<Uuid>(key)) == Some(snippet.id) {
            ctx.data_mut(|d| d.remove::<Uuid>(key));
        }
    }
    let mine = ctx.data(|d| d.get_temp::<Uuid>(key)) == Some(snippet.id);
    if mine && !handle.dragged() && !handle.drag_stopped() {
        ctx.data_mut(|d| d.remove::<Uuid>(key));
        return false;
    }
    mine
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::text::CCursor;

    fn range(a: usize, b: usize) -> CCursorRange {
        CCursorRange::two(CCursor::new(a), CCursor::new(b))
    }

    #[test]
    fn selection_counts_characters_not_bytes() {
        let text = "één coördinatie, auto's";
        assert_eq!(
            selected_text(text, range(4, 15)).as_deref(),
            Some("coördinatie")
        );
        assert_eq!(
            selected_text(text, range(15, 4)).as_deref(),
            Some("coördinatie")
        );
        assert_eq!(selected_text(text, range(0, 3)).as_deref(), Some("één"));
        // Out of range is clamped.
        assert_eq!(
            selected_text(text, range(17, 99)).as_deref(),
            Some("auto's")
        );
    }

    #[test]
    fn blank_selection_is_none() {
        assert_eq!(selected_text("a  \n\t b", range(1, 5)), None);
        assert_eq!(selected_text("abc", range(2, 2)), None);
        assert_eq!(
            selected_text("a  word \n b", range(1, 9)).as_deref(),
            Some("word")
        );
    }

    #[test]
    fn excerpt_is_short_and_on_one_line() {
        assert_eq!(excerpt("  short\n text "), "short text");
        let long = "word ".repeat(30);
        let e = excerpt(&long);
        assert!(e.ends_with('…'));
        assert!(e.chars().count() <= EXCERPT_CHARS + 1);
        assert_eq!(excerpt(&"é".repeat(50)), format!("{}…", "é".repeat(40)));
    }
}
