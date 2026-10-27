//! Help pages in the Text panel, F1's topic, and the About page.

use super::Action;
use crate::app::{ReasonCraftApp, RibbonTab};
use crate::help::{HelpPage, START};
use crate::settings::TextTab;
use crate::shell::panes::Pane;

pub(super) fn apply(app: &mut ReasonCraftApp, action: Action) {
    match action {
        Action::OpenHelp(page) => show(app, page),
        Action::ContextHelp => show(app, HelpPage::Topic(context_topic(app))),
        Action::HelpBack => app.help.back(),
        Action::ShowAbout => {
            app.ribbon_tab = RibbonTab::File;
            if let Some(ctx) = &app.egui_ctx {
                crate::shell::backstage::open_about(ctx);
            }
        }
        _ => unreachable!("not a help action"),
    }
}

/// Show `page` in the Help tab of the Text panel, opening the panel (and
/// leaving the backstage, which hides it).
fn show(app: &mut ReasonCraftApp, page: HelpPage) {
    if app.ribbon_tab == RibbonTab::File {
        app.ribbon_tab = RibbonTab::Home;
    }
    app.right_panel = true;
    app.settings.text_tab = TextTab::Help;
    app.help.open(page);
}

/// The topic for the part of the window with the keyboard focus: the
/// topic of the Text panel tab shown, when the focus is in that panel;
/// otherwise Getting started.
fn context_topic(app: &ReasonCraftApp) -> &'static str {
    let focused = app.egui_ctx.as_ref().and_then(|ctx| app.panes.focused(ctx));
    if focused != Some(Pane::Text) {
        return START;
    }
    match app.settings.text_tab {
        TextTab::Check => "checks",
        TextTab::Outline | TextTab::Essay => "outline-and-essay",
        TextTab::Snippets => "snippets",
        TextTab::Help => START,
    }
}
