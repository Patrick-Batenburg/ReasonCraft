//! Side panels keep the width the user gave them. A small window squeezes
//! a panel (see [`super::panel_width_range`]); egui then stores the
//! squeezed width as the panel's own, so a larger window would not give
//! the width back. The chosen width is kept here (with the window state,
//! so across restarts too), and the panel's stored
//! width is set from it every frame.

use eframe::egui::{self, Rangef, Rect, containers::panel::PanelState, pos2, vec2};

/// Which side a panel is on.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Side {
    Left,
    Right,
}

/// Before the panel `salt` on `side` is shown in `ui`: remember the width
/// the user chose (dragging the panel's edge) and give the panel that
/// width, within `range`. Until the user drags, the chosen width is the
/// one the panel has, or `default` while it is squeezed to the most
/// `range` allows.
pub(super) fn keep_chosen(ui: &egui::Ui, salt: &str, side: Side, range: Rangef, default: f32) {
    let ctx = ui.ctx();
    let panel = ui.scope_id().with(salt);
    let Some(state) = PanelState::load(ctx, panel) else {
        return;
    };
    let width = state.size().x;
    // The edge's handle, as egui names it (see `show_side_panels`).
    let handle = ctx.read_response(panel.with("__resize"));
    let dragged = handle
        .as_ref()
        .is_some_and(|r| r.dragged() || r.drag_stopped());
    // egui stores the dragged width in the frame the drag ends, after
    // this runs: the width is taken up in the frame after.
    let drag_id = panel.with("was_dragged");
    let was_dragged = ctx.data_mut(|d| d.remove_temp::<bool>(drag_id)).is_some();
    if dragged {
        ctx.data_mut(|d| d.insert_temp(drag_id, true));
        return;
    }
    let chosen_id = panel.with("chosen_width");
    let squeezed = width >= range.max - 0.5;
    let chosen = match ctx.data_mut(|d| d.get_persisted::<f32>(chosen_id)) {
        Some(chosen) if !was_dragged => chosen,
        None if squeezed => width.max(default),
        _ => width,
    };
    // Kept with the window state, so a restart in a small window does
    // not forget it.
    ctx.data_mut(|d| d.insert_persisted(chosen_id, chosen));
    let want = range.clamp(chosen);
    // Exactly: egui widens a panel to contents a fraction too wide and
    // stores that width; laid out at it, the text may wrap otherwise and
    // be a different fraction too wide, and so on, frame after frame.
    if want == width {
        return;
    }
    let r = state.outer_rect;
    let outer_rect = match side {
        Side::Left => Rect::from_min_size(r.min, vec2(want, r.height())),
        Side::Right => Rect::from_min_max(pos2(r.max.x - want, r.min.y), r.max),
    };
    ctx.data_mut(|d| d.insert_persisted(panel, PanelState { outer_rect }));
}
