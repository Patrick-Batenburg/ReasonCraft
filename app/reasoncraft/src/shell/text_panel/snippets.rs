//! The Snippets tab: pieces of source text kept beside the map, each
//! tagged with where it came from, in an order the user drags into place.
//!
//! "Paste as snippet" asks the window for a paste (`RequestPaste`); the
//! clipboard text then arrives as a paste event on the next frame and
//! becomes a new snippet. Ctrl+V (Cmd+V on macOS) does the same while the pointer is over
//! the tab and no text field has focus; with a snippet's text field
//! focused, Ctrl+V pastes into that snippet as usual.

use eframe::egui::{
    Context, Event, FontId, Id, LayerId, Order, Pos2, Rect, RichText, Stroke, StrokeKind, Ui,
    ViewportCommand, vec2,
};
use reasoncraft_core::{Snippet, Uuid};

use super::drag_chip::excerpt;
use super::snippet_card::{self, Card};
use super::virtual_rows::{self, Rows};
use crate::actions::Action;
use crate::canvas::input::{Os, paste_shortcut, shortcut_text};
use crate::doc_tab::DocTab;
use crate::i18n::{FluentArgs, t, t_args};
use crate::spell::SpellFields;
use crate::theme::Tokens;

/// How many frames a "Paste as snippet" click waits for the clipboard text.
const PASTE_WAIT_PASSES: u64 = 3;

/// Draw the Snippets tab for `tab` and queue the user's commands in `out`.
pub fn show(
    ui: &mut Ui,
    tab: &mut DocTab,
    os: Os,
    tk: &Tokens,
    spell: &mut SpellFields,
    out: &mut Vec<Action>,
) {
    let area = ui.max_rect();
    paste_as_snippet(ui, area, out);
    let mut keys = FluentArgs::new();
    keys.set("paste", shortcut_text(paste_shortcut(), os));
    toolbar(ui, &keys, out);
    ui.add_space(6.0);
    if tab.doc.snippets.is_empty() {
        ui.label(RichText::new(t_args("snippet.empty", &keys)).color(tk.text_muted));
        ui.take_available_space();
        return;
    }
    let focus = tab.snippet_focus.take();
    let mut heights = std::mem::take(&mut tab.snippet_heights);
    if focus.is_some() {
        heights.reveal = focus;
    }
    let tab_ref = &*tab;
    let snippets = &tab_ref.doc.snippets;
    let keys: Vec<Uuid> = snippets.iter().map(|s| s.id).collect();
    // The card being dragged stays laid out, and so does one holding the
    // keyboard focus in its text (a new snippet at the end of a long
    // list, before the list has measured the rows above it).
    let dragging = ui.data(|d| d.get_temp::<Uuid>(dragging_id()));
    let focused = ui.memory(|m| m.focused());
    let typing = || {
        let focused = focused?;
        let s = snippets
            .iter()
            .find(|s| snippet_card::text_id(tab_ref, s) == focused)?;
        Some(s.id)
    };
    let pinned = (dragging.or_else(typing)).and_then(|id| keys.iter().position(|&k| k == id));
    let mut cards: Vec<(usize, Card)> = Vec::new();
    let draw = |ui: &mut Ui, i: usize| {
        let snippet = &snippets[i];
        if i > 0 {
            ui.add_space(6.0);
        }
        let focus = focus == Some(snippet.id);
        let capped = tab_ref.snippet_capped == Some(snippet.id);
        let state = snippet_card::State { focus, capped };
        let card = snippet_card::show(ui, tab_ref, snippet, state, tk, spell, out);
        cards.push((i, card));
    };
    let rows = Rows {
        keys: &keys,
        estimate: ROW_ESTIMATE,
        revision: tab_ref.revision(),
        pinned,
        draw,
    };
    let rects = virtual_rows::show(ui, &mut heights, rows);
    let dragged = cards
        .into_iter()
        .find(|(_, c)| c.handle.dragged() || c.handle.drag_stopped());
    ui.data_mut(|d| match &dragged {
        Some((i, c)) if c.handle.dragged() => {
            d.insert_temp(dragging_id(), snippets[*i].id);
        }
        _ => d.remove::<Uuid>(dragging_id()),
    });
    if let Some((i, card)) = dragged.filter(|(_, c)| !c.to_map) {
        reorder(ui, area, snippets, &rects, (i, &card), tk, out);
    }
    tab.snippet_heights = heights;
}

/// The height assumed for a snippet card not laid out yet: two lines of
/// text with the handle and source rows.
const ROW_ESTIMATE: f32 = 120.0;

/// Where the snippet whose card is being dragged is kept between frames.
fn dragging_id() -> Id {
    Id::unique("snippet_dragging")
}

fn toolbar(ui: &mut Ui, keys: &FluentArgs, out: &mut Vec<Action>) {
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
            .on_hover_text(t_args("snippet.tip.paste", keys))
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

/// While card `dragged` (the index of its snippet, and what it reported)
/// is dragged by its handle, show a ghost of it at the pointer and, over
/// the Snippets tab (`area`), mark where it would land; on release there,
/// move it there. The place counts the other cards (at `rects`, one per
/// snippet) above the pointer.
fn reorder(
    ui: &Ui,
    area: Rect,
    snippets: &[Snippet],
    rects: &[Rect],
    (dragged, card): (usize, &Card),
    tk: &Tokens,
    out: &mut Vec<Action>,
) {
    let Some(pointer) = ui.ctx().pointer_latest_pos() else {
        return;
    };
    let others: Vec<Rect> = rects
        .iter()
        .enumerate()
        .filter(|&(i, _)| i != dragged)
        .map(|(_, r)| *r)
        .collect();
    let index = others.iter().filter(|r| r.center().y < pointer.y).count();
    // Released outside the Snippets tab (over the map, say): nothing moves.
    let inside = area.contains(pointer);
    if card.handle.drag_stopped() {
        if inside {
            let id = snippets[dragged].id;
            out.push(Action::MoveSnippet { id, index });
        }
        return;
    }
    let width = card.rect.width().min(GHOST_MAX_W);
    paint_ghost(ui.ctx(), &snippets[dragged].text, pointer, width, tk);
    if !inside {
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

/// The widest a dragged card's ghost gets, in px.
const GHOST_MAX_W: f32 = 240.0;
const GHOST_PAD: f32 = 8.0;

/// A dragged snippet's ghost: a card at most `width` wide with the start of
/// its `text`, just below and right of `pointer`, above everything else.
fn paint_ghost(ctx: &Context, text: &str, pointer: Pos2, width: f32, tk: &Tokens) {
    let painter = ctx.layer_painter(LayerId::new(Order::Tooltip, Id::unique("snippet_ghost")));
    let wrap = (width - 2.0 * GHOST_PAD).max(40.0);
    let galley = painter.layout(excerpt(text), FontId::proportional(13.0), tk.box_text, wrap);
    let size = galley.size() + vec2(2.0, 2.0) * GHOST_PAD;
    let rect = Rect::from_min_size(pointer + vec2(14.0, 12.0), size);
    let shadow = ctx.global_style().visuals.popup_shadow;
    painter.add(shadow.as_shape(rect, 6.0));
    painter.rect(
        rect,
        6.0,
        tk.panel,
        Stroke::new(1.5, tk.accent),
        StrokeKind::Outside,
    );
    painter.galley(rect.min + vec2(GHOST_PAD, GHOST_PAD), galley, tk.box_text);
}
