//! Accessibility helpers: the names screen readers announce for controls.

use eframe::egui::{self, Id, Response, Role, WidgetInfo, accesskit};

use crate::i18n::{FluentArgs, t_args};

/// `response` announced by `name` instead of its visible text (a glyph
/// such as `Aa`, or a word that only its surroundings make clear). It
/// keeps the role, state and enabled state the widget gave itself. Every
/// glyph-only button is named through this.
pub fn labelled(response: Response, name: impl Into<String>) -> Response {
    let name = name.into();
    let (role, toggled) = response
        .ctx
        .accesskit_node_builder(response.id, |node| (node.role(), node.toggled()))
        .unwrap_or((Role::Unknown, None));
    let role = if role == Role::Unknown {
        Role::Button
    } else {
        role
    };
    // On or off as the widget said (a mixed check box stays mixed).
    let selected = match toggled {
        Some(accesskit::Toggled::True) => Some(true),
        Some(accesskit::Toggled::False) => Some(false),
        _ => None,
    };
    let enabled = response.enabled();
    response.widget_info(|| {
        let mut info = WidgetInfo::labeled(role, enabled, &name);
        info.selected = selected;
        info
    });
    response
}

/// Name the widget `id` that egui made itself (a panel's resize handle),
/// for which there is no [`Response`] to hand to [`labelled`]. Does
/// nothing when the widget is not shown this frame.
pub fn name_node(ctx: &egui::Context, id: Id, name: &str) {
    if ctx.viewport(|v| v.this_pass.widgets.get(id).is_some()) {
        ctx.accesskit_node_builder(id, |node| node.set_label(name));
    }
}

/// "`group`: `name`": the name of a control whose visible text is only
/// clear from the group it is in (Theme: System, Card: Nil).
pub fn in_group(group: &str, name: &str) -> String {
    let mut args = FluentArgs::new();
    args.set("group", group);
    args.set("name", name);
    t_args("a11y.in_group", &args)
}

/// Where to outline the control that has the keyboard focus: most of the
/// app's controls are drawn by hand and get no focus look from egui. A
/// text field (its caret) and the map (its selection) show focus their own
/// way, so they get none; nor does anything when no control has focus.
pub fn focus_ring_rect(ctx: &egui::Context) -> Option<egui::Rect> {
    let id = ctx.memory(|m| m.focused())?;
    if id == crate::canvas::canvas_id() || ctx.text_edit_focused() {
        return None;
    }
    let rect = ctx.read_response(id)?.interact_rect;
    rect.is_positive().then_some(rect)
}

/// Outline the focused control (see [`focus_ring_rect`]) in `colour`,
/// above everything else.
pub fn paint_focus_ring(ctx: &egui::Context, colour: egui::Color32) {
    if let Some(rect) = focus_ring_rect(ctx) {
        let layer = egui::LayerId::new(egui::Order::Tooltip, Id::unique("focus_ring"));
        let stroke = egui::Stroke::new(2.0, colour);
        ctx.layer_painter(layer)
            .rect_stroke(rect, 4.0, stroke, egui::StrokeKind::Inside);
    }
}

/// Name the dialog `ui` is drawn in after its `heading`, so a screen
/// reader announces it when the focus moves into it.
pub fn name_dialog(ui: &egui::Ui, heading: &str) {
    let dialog = ui
        .stack()
        .iter()
        .find(|s| matches!(s.kind(), Some(egui::UiKind::Modal | egui::UiKind::Popup)))
        .map(|s| s.unique_id);
    if let Some(id) = dialog {
        ui.ctx()
            .accesskit_node_builder(id, |node| node.set_label(heading));
    }
}

/// Whether dialog `id` showing `heading` is new this frame: it was not
/// shown last frame, or it showed another heading. Its default control
/// then takes the keyboard focus.
pub fn dialog_opened(ctx: &egui::Context, id: Id, heading: &str) -> bool {
    let pass = ctx.cumulative_pass_nr();
    let key = id.with("dialog_opened");
    let last: Option<(u64, String)> = ctx.data(|d| d.get_temp(key));
    ctx.data_mut(|d| d.insert_temp(key, (pass, heading.to_owned())));
    !last.is_some_and(|(seen, shown)| seen + 1 >= pass && shown == heading)
}

/// A polite live region holding what `add` shows: a screen reader reads
/// it out when it changes, without moving the focus (find counts, status
/// notes, the Move to… hint). The region is there every frame, empty or
/// not, so a note that appears is read too.
pub fn live_region<R>(
    ui: &mut egui::Ui,
    id_salt: &'static str,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let builder = egui::UiBuilder::new()
        .id_salt(id_salt)
        .accessibility_role(Role::Status);
    let shown = ui.scope_builder(builder, add);
    ui.ctx().accesskit_node_builder(shown.response.id, |node| {
        node.set_live(accesskit::Live::Polite);
    });
    shown.inner
}
