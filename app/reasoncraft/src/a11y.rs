//! Accessibility helpers: the names screen readers announce for controls.

use eframe::egui::{self, Id, Response, Role, WidgetInfo, accesskit};

use crate::i18n::{FluentArgs, t_args};

/// `response` announced by `name` instead of its visible text (a glyph
/// such as `Aa`, or a word that only its surroundings make clear). It
/// keeps the role, state and enabled state the widget gave itself. Every
/// glyph-only button is named through this.
pub fn labelled(response: Response, name: impl Into<String>) -> Response {
    let name = name.into();
    let role = response
        .ctx
        .accesskit_node_builder(response.id, |node| node.role())
        .filter(|role| *role != accesskit::Role::Unknown)
        .unwrap_or(Role::Button);
    let enabled = response.enabled();
    response.widget_info(|| WidgetInfo::labeled(role, enabled, &name));
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
