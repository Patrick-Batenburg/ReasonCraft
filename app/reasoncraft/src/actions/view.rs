//! View preferences: theme, rating display, panels and the Text panel tab.
//! None of them change the document or end "Connect to…".

use super::{Action, Panel};
use crate::app::ReasonCraftApp;

pub(super) fn apply(app: &mut ReasonCraftApp, action: Action) {
    let s = &mut app.settings;
    match action {
        Action::SetTheme(choice) => s.theme = choice,
        Action::ToggleMarkers => s.show_markers = !s.show_markers,
        Action::ToggleShading => s.eval_shading = !s.eval_shading,
        Action::TogglePanel(Panel::Left) => app.left_panel = !app.left_panel,
        Action::TogglePanel(Panel::Right) => app.right_panel = !app.right_panel,
        Action::SetTextTab(tab) => s.text_tab = tab,
        _ => unreachable!("not a view action"),
    }
}
