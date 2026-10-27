//! The backstage New page: a blank map, the templates and the examples.

use eframe::egui::{self, RichText};
use reasoncraft_core::examples::{Example, Template};

use super::choice::choice;
use crate::actions::Action;
use crate::app::{ReasonCraftApp, RibbonTab};
use crate::i18n::t;
use crate::shell::icons::Icon;
use crate::shell::template_key;

/// The page. True when a choice was made: the map shows again.
pub(super) fn page(ui: &mut egui::Ui, app: &mut ReasonCraftApp) -> bool {
    let tk = app.tokens;
    ui.spacing_mut().item_spacing.y = 4.0;
    let heading = |ui: &mut egui::Ui, key: &str, size: f32| {
        ui.add_space(8.0);
        ui.label(RichText::new(t(key)).strong().size(size).color(tk.box_text));
    };
    heading(ui, "backstage.new.title", 18.0);
    let mut chosen = None;
    let mut offer = |ui: &mut egui::Ui, icon, key: &str, action: Action| {
        let name = t(key);
        if choice(ui, &tk, icon, &name, &t(&format!("{key}.hint")), &name).clicked() {
            chosen = Some(action);
        }
    };
    offer(ui, Icon::New, "backstage.new.blank", Action::New);
    heading(ui, "backstage.templates", 15.0);
    for which in Template::ALL {
        let (key, icon) = template_key(which);
        offer(ui, icon, key, Action::OpenTemplate(which));
    }
    heading(ui, "backstage.examples", 15.0);
    offer(
        ui,
        Icon::Reason,
        "example.reasoning",
        Action::OpenExample(Example::Reasoning),
    );
    offer(
        ui,
        Icon::Category,
        "example.grouping",
        Action::OpenExample(Example::Grouping),
    );
    let Some(action) = chosen else {
        return false;
    };
    app.ribbon_tab = RibbonTab::Home;
    app.push(action);
    true
}
