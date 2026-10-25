//! The Snippets tab: pieces of source text kept beside the map, each
//! tagged with where it came from, in an order the user drags into place.
//!
//! "Paste as snippet" asks the window for a paste (`RequestPaste`); the
//! clipboard text then arrives as a paste event on the next frame and
//! becomes a new snippet. Ctrl+V does the same while the pointer is over
//! the tab and no text field has focus; with a snippet's text field
//! focused, Ctrl+V pastes into that snippet as usual.

use eframe::egui::{Event, Id, Rect, RichText, ScrollArea, Stroke, Ui, ViewportCommand};
use reasoncraft_core::Uuid;

use super::snippet_card::{self, Card};
use crate::actions::Action;
use crate::doc_tab::DocTab;
use crate::i18n::t;
use crate::theme::Tokens;

/// How many frames a "Paste as snippet" click waits for the clipboard text.
const PASTE_WAIT_PASSES: u64 = 3;

/// Draw the Snippets tab for `tab` and queue the user's commands in `out`.
pub fn show(ui: &mut Ui, tab: &mut DocTab, tk: &Tokens, out: &mut Vec<Action>) {
    let area = ui.max_rect();
    paste_as_snippet(ui, area, out);
    toolbar(ui, out);
    ui.add_space(6.0);
    if tab.doc.snippets.is_empty() {
        ui.label(RichText::new(t("snippet.empty")).color(tk.text_muted));
        ui.take_available_space();
        return;
    }
    let focus = tab.snippet_focus.take();
    let tab = &*tab;
    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let mut cards: Vec<(Uuid, Card)> = Vec::with_capacity(tab.doc.snippets.len());
            for snippet in &tab.doc.snippets {
                let focus = focus == Some(snippet.id);
                let capped = tab.snippet_capped == Some(snippet.id);
                let card = snippet_card::show(ui, snippet, focus, capped, tk, out);
                cards.push((snippet.id, card));
                ui.add_space(6.0);
            }
            reorder(ui, &cards, tk, out);
        });
}

fn toolbar(ui: &mut Ui, out: &mut Vec<Action>) {
    ui.horizontal_wrapped(|ui| {
        if ui
            .button(t("snippet.new"))
            .on_hover_text(t("snippet.tip.new"))
            .clicked()
        {
            out.push(Action::AddSnippet(String::new()));
        }
        if ui
            .button(t("snippet.paste"))
            .on_hover_text(t("snippet.tip.paste"))
            .clicked()
        {
            let pass = ui.ctx().cumulative_pass_nr();
            ui.data_mut(|d| d.insert_temp(paste_wait_id(), pass));
            ui.ctx().send_viewport_cmd(ViewportCommand::RequestPaste);
            ui.ctx().request_repaint();
        }
    });
}

fn paste_wait_id() -> Id {
    Id::unique("snippet_paste_requested_at")
}

/// Turn pasted text into a new snippet: right after "Paste as snippet",
/// or for Ctrl+V while the pointer is over `area` and no text field has
/// focus. Blank text is ignored.
fn paste_as_snippet(ui: &mut Ui, area: Rect, out: &mut Vec<Action>) {
    let ctx = ui.ctx().clone();
    let pass = ctx.cumulative_pass_nr();
    let requested = ctx
        .data(|d| d.get_temp::<u64>(paste_wait_id()))
        .filter(|at| pass <= at + PASTE_WAIT_PASSES);
    let hovered = ui.rect_contains_pointer(area) && !ctx.text_edit_focused();
    if requested.is_none() && !hovered {
        ctx.data_mut(|d| d.remove::<u64>(paste_wait_id()));
        return;
    }
    let mut pasted = None;
    ctx.input_mut(|i| {
        i.events.retain(|e| match e {
            Event::Paste(text) if pasted.is_none() && !text.trim().is_empty() => {
                pasted = Some(text.clone());
                false
            }
            _ => true,
        });
    });
    if let Some(text) = pasted {
        ctx.data_mut(|d| d.remove::<u64>(paste_wait_id()));
        out.push(Action::AddSnippet(text));
    }
}

/// While a card's handle is dragged, mark where it would land; on release,
/// move it there. The place counts the other cards above the pointer.
fn reorder(ui: &Ui, cards: &[(Uuid, Card)], tk: &Tokens, out: &mut Vec<Action>) {
    let Some((dragged, card)) = cards
        .iter()
        .find(|(_, c)| c.handle.dragged() || c.handle.drag_stopped())
    else {
        return;
    };
    let Some(pointer) = ui.ctx().pointer_latest_pos() else {
        return;
    };
    let others: Vec<Rect> = cards
        .iter()
        .filter(|(id, _)| id != dragged)
        .map(|(_, c)| c.rect)
        .collect();
    let index = others.iter().filter(|r| r.center().y < pointer.y).count();
    if card.handle.drag_stopped() {
        out.push(Action::MoveSnippet {
            id: *dragged,
            index,
        });
        return;
    }
    let y = match (index.checked_sub(1).map(|i| others[i]), others.get(index)) {
        (Some(above), Some(below)) => (above.bottom() + below.top()) / 2.0,
        (Some(above), None) => above.bottom() + 3.0,
        (None, Some(below)) => below.top() - 3.0,
        (None, None) => return,
    };
    let x = card.rect.x_range();
    ui.painter().hline(x, y, Stroke::new(2.5, tk.accent));
}
