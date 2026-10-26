//! The title bar's Search commands box (Alt+Q): type part of a command's
//! name, pick it from the list under the box with the arrows and Enter (or
//! a click), and it runs as if given from the ribbon.

use eframe::egui::{
    self, Align2, Area, EventFilter, FontId, Frame, Id, Key, Margin, Modifiers, Order, Response,
    Role, Sense, Stroke, TextEdit, Ui, Vec2, WidgetInfo, pos2,
};

use super::selected_fill;
use crate::actions::{Action, is_enabled};
use crate::app::{ReasonCraftApp, RibbonTab};
use crate::canvas::input::{shortcut_for, shortcut_text};
use crate::commands::{Command, search};
use crate::i18n::{FluentArgs, t, t_args};
use crate::theme::Tokens;

/// At most this many results show.
const MAX_ROWS: usize = 12;
const FIELD_W: f32 = 260.0;
const LIST_W: f32 = 380.0;
const ROW_H: f32 = 26.0;

/// What the box remembers between frames (kept in egui memory: the box
/// belongs to the window, not to a document).
#[derive(Clone, Default)]
struct State {
    query: String,
    /// The highlighted result row.
    highlight: usize,
    /// The pointer was over the result list last frame (a click on a row
    /// takes the focus from the field before the click completes).
    list_hovered: bool,
    /// Enter was pressed on a greyed-out command: say why nothing ran.
    refused: bool,
}

/// The id of the text field.
pub fn field_id() -> Id {
    Id::unique("command_search")
}

/// Put the keyboard focus in the box (it shows from the next frame on).
pub fn focus(ctx: &egui::Context) {
    ctx.memory_mut(|m| m.request_focus(field_id()));
}

pub fn show(ui: &mut Ui, app: &mut ReasonCraftApp) {
    let tk = app.tokens;
    let id = field_id();
    let state_id = id.with("state");
    let mut st: State = ui.data(|d| d.get_temp(state_id)).unwrap_or_default();
    let has_focus = ui.memory(|m| m.has_focus(id));
    // Take the list keys before the field sees them (Enter would end its
    // focus). The field's event filter keeps the focus on Esc and the
    // arrows, so the map does not take them first.
    let (mut up, mut down, mut enter, mut esc) = (false, false, false, false);
    if has_focus {
        ui.input_mut(|i| {
            up = i.consume_key(Modifiers::NONE, Key::ArrowUp);
            down = i.consume_key(Modifiers::NONE, Key::ArrowDown);
            enter = i.consume_key(Modifiers::NONE, Key::Enter);
            esc = i.consume_key(Modifiers::NONE, Key::Escape);
        });
    }
    if esc {
        st = State::default();
        ui.memory_mut(|m| m.surrender_focus(id));
    }

    let field = Frame::new()
        .fill(tk.panel)
        .stroke(Stroke::new(1.0, tk.attachment_line.gamma_multiply(0.6)))
        .corner_radius(6)
        .inner_margin(Margin::symmetric(8, 4));
    let mut args = FluentArgs::new();
    let keys = shortcut_for(&Action::OpenCommandSearch).map(shortcut_text);
    args.set("keys", keys.unwrap_or_default());
    let hint = t_args("titlebar.search", &args);
    let resp = ui.add(
        TextEdit::singleline(&mut st.query)
            .id(id)
            .hint_text(&hint)
            .frame(field)
            .desired_width(FIELD_W)
            .event_filter(EventFilter {
                horizontal_arrows: true,
                vertical_arrows: true,
                escape: true,
                tab: false,
            }),
    );
    resp.widget_info(|| WidgetInfo::labeled(Role::TextInput, true, &hint));
    if resp.changed() {
        st.highlight = 0;
        st.refused = false;
    }

    let open = resp.has_focus() || st.list_hovered;
    let results = if open {
        search(&st.query, MAX_ROWS)
    } else {
        Vec::new()
    };
    if results.is_empty() {
        st.list_hovered = false;
        ui.data_mut(|d| d.insert_temp(state_id, st));
        return;
    }
    let last = results.len() - 1;
    st.highlight = st.highlight.min(last);
    if down {
        st.highlight = (st.highlight + 1).min(last);
        st.refused = false;
    }
    if up {
        st.highlight = st.highlight.saturating_sub(1);
        st.refused = false;
    }
    let mut run = enter.then_some(st.highlight);
    let enabled: Vec<bool> = results
        .iter()
        .map(|c| is_enabled(app.active_tab(), &c.action))
        .collect();
    if run.is_some_and(|i| !enabled[i]) {
        run = None;
        st.refused = true;
    }

    let list = Area::new(id.with("list"))
        .order(Order::Foreground)
        .fixed_pos(resp.rect.left_bottom() + Vec2::new(0.0, 4.0))
        .show(ui.ctx(), |ui| {
            Frame::popup(ui.style()).fill(tk.panel).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for (i, cmd) in results.iter().enumerate() {
                    let row = result_row(ui, &tk, cmd, enabled[i], i == st.highlight);
                    if row.hovered() && i != st.highlight {
                        st.highlight = i;
                        st.refused = false;
                    }
                    if row.clicked() {
                        run = Some(i);
                    }
                    if i == st.highlight && st.refused {
                        row.show_tooltip_text(t("command.disabled"));
                    } else if !enabled[i] {
                        row.on_hover_text(t("command.disabled"));
                    }
                }
            });
        });
    st.list_hovered = list.response.contains_pointer();

    if let Some(i) = run {
        // Back to the map from the backstage, like its own commands.
        if app.ribbon_tab == RibbonTab::File {
            app.ribbon_tab = RibbonTab::Home;
        }
        app.push(results[i].action.clone());
        st = State::default();
        ui.memory_mut(|m| m.surrender_focus(id));
    }
    ui.data_mut(|d| d.insert_temp(state_id, st));
}

/// One result: the label, its group, and the shortcut at the right.
fn result_row(ui: &mut Ui, tk: &Tokens, cmd: &Command, enabled: bool, on: bool) -> Response {
    let label = t(cmd.label);
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(LIST_W, ROW_H), sense);
    resp.widget_info(|| WidgetInfo::selected(Role::ListItem, enabled, on, &label));
    if !ui.is_rect_visible(rect) {
        return resp;
    }
    let painter = ui.painter();
    if on {
        painter.rect_filled(rect, 4.0, selected_fill(tk, tk.panel));
    }
    let colour = if enabled { tk.box_text } else { tk.text_muted };
    let font = FontId::proportional(13.0);
    let left = pos2(rect.left() + 8.0, rect.center().y);
    let shown = painter.text(left, Align2::LEFT_CENTER, &label, font.clone(), colour);
    let group_pos = pos2(shown.right() + 10.0, rect.center().y);
    let small = FontId::proportional(11.0);
    let group = t(cmd.group);
    painter.text(
        group_pos,
        Align2::LEFT_CENTER,
        group,
        small.clone(),
        tk.text_muted,
    );
    if let Some(s) = shortcut_for(&cmd.action) {
        let right = pos2(rect.right() - 8.0, rect.center().y);
        painter.text(
            right,
            Align2::RIGHT_CENTER,
            shortcut_text(s),
            small,
            tk.text_muted,
        );
    }
    resp
}
