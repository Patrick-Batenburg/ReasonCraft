//! Every command the user can give, and the one place that carries them out.

use crate::app::ReasonCraftApp;

mod action;
mod classify;
mod dispatch;

mod box_group;
pub use action::{Action, Dir, Panel, UnsavedChoice};
mod enabled;
pub use box_group::{FakeOpener, LinkOpener, SystemOpener};
pub use enabled::{can_run, is_enabled};
mod why_disabled;
pub use why_disabled::{EXPLAINED as EXPLAINED_DISABLED, why_disabled};
mod argdown;
pub use argdown::report_text as import_report_text;
mod clipboard;
pub use clipboard::Copied;
mod comments;
mod editing;
mod essay;
mod evaluate;
mod examples;
mod export;
mod find;
mod help;
pub(crate) use export::close_after_exports;
pub use export::poll_exports;
mod images;
pub use images::{poll_images, wait_for_images};
mod meta;
pub use evaluate::{Rate, Rated, rate_action, rated};
mod insert;
pub use insert::{Insertable, target_allowed};
mod connect;
pub use connect::connect_item;
mod snippets;
mod sources;
mod structure;
mod view;
use structure::select_new;
pub use structure::{moves_nowhere, selected_card, sibling_list};

pub use crate::canvas::camera::{ZOOM_MAX, ZOOM_MIN};

/// Carry out `action`. Document edits go through [`crate::doc_tab::DocTab::edit`], which
/// records history only when the edit succeeds.
///
/// Every action first ends the running edit session, so it lands in
/// history before anything else happens, except live text for that same
/// kind of session; then it clears the status note and ends "Move to…"
/// in the active tab unless it keeps them (see `Action::traits`). A
/// finished image job is not the user's doing: it leaves all of that alone.
pub fn apply(app: &mut ReasonCraftApp, action: Action) {
    if !matches!(action, Action::ImagePrepared { .. }) {
        let traits = action.traits();
        if !traits.keeps_notes {
            app.status_note = None;
            app.export.notice = None;
        }
        if let Some(tab) = app.tabs.get_mut(app.active) {
            if !action.continues(tab.session()) {
                tab.commit_session();
            }
            if !traits.keeps_connect {
                tab.connect_mode = None;
            }
        }
    }
    dispatch::run(app, action);
    crate::file_ops::explain_missing_dialog(app);
    if let Some(tab) = app.tabs.get_mut(app.active)
        && std::mem::take(&mut tab.refused_too_deep)
    {
        let mut args = fluent_bundle::FluentArgs::new();
        args.set("max", reasoncraft_core::MAX_DEPTH);
        app.status_note = Some(crate::i18n::t_args("status.too_deep", &args));
    }
}

#[cfg(test)]
mod tests;
