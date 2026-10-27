//! Every command the user can give, and the one place that carries them out.

use reasoncraft_core::{BoxKind, CardKind, Item, NewLoose, Parent, RemoveMode};

use crate::app::ReasonCraftApp;
use crate::doc_tab::DocTab;
use crate::file_ops;
use crate::i18n::t;

mod action;
mod classify;
use classify::{continues_session, keeps_notes};
mod box_group;
pub use action::{Action, Dir, Panel, UnsavedChoice};
mod enabled;
pub use box_group::{FakeOpener, LinkOpener, SystemOpener};
pub use enabled::is_enabled;
mod argdown;
pub use argdown::report_text as import_report_text;
mod comments;
mod editing;
mod essay;
mod evaluate;
mod export;
mod find;
pub(crate) use export::close_after_exports;
pub use export::poll_exports;
pub(crate) use find::cap_box;
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
use structure::{add_box, add_card, add_co_premise, delete, select_new};
pub use structure::{is_attachment, moves_nowhere, selected_card, sibling_list};

pub use crate::canvas::camera::{ZOOM_MAX, ZOOM_MIN};

/// Carry out `action`. Document edits go through [`DocTab::edit`], which
/// records history only when the edit succeeds.
///
/// Every action first ends the running edit session, so it lands in
/// history before anything else happens, except live text for that same
/// kind of session (see `continues_session`).
/// Every action but picking a target then ends "Connect to…" in the active
/// tab, app-level ones (inserting, comments, links, switching tabs) too.
pub fn apply(app: &mut ReasonCraftApp, action: Action) {
    // A finished image job is not the user's doing: it leaves the active
    // tab's edit session and "Connect to…" alone.
    if let Action::ImagePrepared { job, node, result } = action {
        images::prepared(app, job, node, result);
        return;
    }
    // The user moved on: the last status note and export notice go.
    if !keeps_notes(&action) {
        app.status_note = None;
        app.export.notice = None;
    }
    if let Some(tab) = app.tabs.get_mut(app.active) {
        if !continues_session(&action, tab.session()) {
            tab.commit_session();
        }
        connect::end_unless_kept(tab, &action);
    }
    match action {
        Action::New => {
            app.tabs
                .push(DocTab::with_language(&app.settings.default_doc_language));
            app.active = app.tabs.len() - 1;
        }
        Action::Open => file_ops::open_with_dialog(app),
        Action::ImportArgdown => argdown::import_with_dialog(app),
        Action::Save => file_ops::save_active(app, false),
        Action::SaveAs => file_ops::save_active(app, true),
        Action::CloseTab(i) => {
            // A finished image lands first, so the prompt counts it.
            images::flush_tab(app, i);
            file_ops::request_close_tab(app, i);
        }
        Action::CloseWindow => {
            for i in 0..app.tabs.len() {
                images::flush_tab(app, i);
            }
            file_ops::request_close_window(app);
        }
        Action::ResolveUnsaved(choice) => {
            // "Save" includes the images still being prepared: it waits
            // for them with the prompt open.
            if choice != UnsavedChoice::Save || !images::wait_before_save(app) {
                file_ops::resolve_unsaved(app, choice);
            }
        }
        Action::ResolveReplace(replace) => file_ops::resolve_replace(app, replace),
        Action::DismissModal => {
            app.modal = None;
            app.link_edit = None;
        }
        Action::ActivateTab(i) => {
            if i < app.tabs.len() {
                app.active = i;
            }
        }
        Action::SetTheme(_)
        | Action::SetUiLanguage(_)
        | Action::SetConnectorStyle(_)
        | Action::ToggleMarkers
        | Action::SetMinimap(_)
        | Action::ToggleNumbers
        | Action::ToggleShading
        | Action::TogglePanel(_)
        | Action::ShowPanel(_)
        | Action::SetTextTab(_)
        | Action::SetOutlineOption(..)
        | Action::SetCheckEnabled(..)
        | Action::SetDefaultDocLanguage(_)
        | Action::SetSpellcheck(_)
        | Action::SetAutosaveSecs(_)
        | Action::SetUiScale(_) => view::apply(app, action),
        Action::ResetSettings => app.modal = Some(file_ops::Modal::ResetSettings),
        Action::ConfirmResetSettings => {
            app.modal = None;
            app.settings.reset_to_defaults();
            app.apply_ui_language();
        }
        Action::ResolveRecovery(choice) => crate::recovery::resolve(app, choice),
        Action::AddComment { .. }
        | Action::RemoveComment { .. }
        | Action::OpenComments(_)
        | Action::CommentSelected
        | Action::CloseComments
        | Action::SetAuthor(_) => comments::apply(app, action),
        Action::Insert { .. } => insert::apply(app, action),
        Action::SetCardKind(_) | Action::EditLink | Action::SetLink(_) | Action::OpenLink(_) => {
            box_group::apply(app, action)
        }
        Action::InsertImage | Action::RemoveImage => images::apply(app, action),
        Action::OpenFind { .. }
        | Action::CloseFind
        | Action::FindNext
        | Action::FindPrev
        | Action::ReplaceCurrent
        | Action::ReplaceAll
        | Action::SetFindQuery(_)
        | Action::SetReplaceText(_)
        | Action::SetFindOptions(_) => find::apply(app, action),
        Action::OpenSpelling
        | Action::CloseSpelling
        | Action::SpellChange { .. }
        | Action::SpellChangeAll { .. }
        | Action::SpellIgnore
        | Action::SpellIgnoreAll(_)
        | Action::SpellAddWord(_) => crate::spell::apply(app, action),
        // Editing a box: text past the limit is cut, with a note.
        Action::Edit | Action::StopEditing | Action::EditText { .. } | Action::TypeText(_) => {
            if let Some(tab) = app.tabs.get_mut(app.active)
                && editing::apply(tab, action)
            {
                app.status_note = Some(t("status.text_cut"));
            }
        }
        Action::OpenExport(_)
        | Action::RunExport(_)
        | Action::CloseExportDialog
        | Action::OpenPageSetup
        | Action::SetPageSetup(_)
        | Action::CopyAsImage
        | Action::Print
        | Action::OpenExportFolder
        | Action::ResolveExportReplace(_)
        | Action::ResolveClosingExports(_) => export::apply(app, action),
        // A running inline edit was committed above, so the box editor
        // lets go of the keyboard.
        Action::OpenCommandSearch => {
            if let Some(ctx) = &app.egui_ctx {
                crate::shell::command_search::focus(ctx);
            }
        }
        #[cfg(debug_assertions)]
        Action::LoadSampleMap => {
            let doc = reasoncraft_core::fixtures::sample_iot_like(117);
            app.tabs.push(DocTab::from_document(doc));
            app.active = app.tabs.len() - 1;
        }
        #[cfg(debug_assertions)]
        Action::ToggleSourceIcons => app.show_source_icons = !app.show_source_icons,
        other => {
            if let Some(tab) = app.tabs.get_mut(app.active) {
                apply_to_tab(tab, other);
            }
        }
    }
}

fn apply_to_tab(tab: &mut DocTab, action: Action) {
    match action {
        Action::Undo => tab.undo(),
        Action::Redo => tab.redo(),
        Action::AddReason => add_card(tab, CardKind::Reason),
        Action::AddObjection => add_card(tab, CardKind::Objection),
        Action::AddCoPremise => add_co_premise(tab),
        Action::AddCategory => add_box(tab, BoxKind::Category, NewLoose::Category),
        Action::AddNote => add_box(tab, BoxKind::Note, NewLoose::Note),
        Action::AddLoose(kind) => {
            let item = tab.edit(|d| Ok(d.add_loose(kind, "")));
            select_new(tab, item);
        }
        Action::TargetWholeCard(kind) => {
            if let Some(c) = selected_card(tab) {
                let new = tab.edit(|d| d.add_card(Parent::Card(c), kind, ""));
                select_new(tab, new.map(|(_, p)| Item::Box(p)));
            }
        }
        Action::Delete => delete(tab, RemoveMode::KeepChildren),
        Action::DeleteBranch => delete(tab, RemoveMode::Branch),
        Action::Move { .. } | Action::Reorder(_) => structure::apply(tab, action),
        Action::StartConnect | Action::CancelConnect => connect::apply(tab, action),
        Action::Select(item) => tab.selection = item.filter(|i| tab.contains(*i)),
        Action::RevealSelection
        | Action::Navigate(_)
        | Action::JumpParking
        | Action::ZoomIn
        | Action::ZoomOut
        | Action::ZoomFit
        | Action::ZoomFitSelection
        | Action::Zoom100
        | Action::SetZoom(_)
        | Action::SetLayoutDirection(_) => view::apply_to_tab(tab, action),
        Action::Rate(_)
        | Action::SetAcceptance(_)
        | Action::SetStrength(_)
        | Action::SetSourceRating(_)
        | Action::ClearEvaluation(_) => evaluate::apply(tab, action),
        Action::AddSource(_) | Action::SetSourceType(_) => sources::apply(tab, action),
        Action::SetDocMeta { .. }
        | Action::SetDocLanguage(_)
        | Action::IgnoreHint(_)
        | Action::RestoreHints => meta::apply(tab, action),
        Action::AddSnippet(_)
        | Action::EditSnippetText { .. }
        | Action::EditSnippetCitation { .. }
        | Action::EditSnippetLink { .. }
        | Action::SetSnippetSource { .. }
        | Action::RemoveSnippet(_)
        | Action::MoveSnippet { .. } => snippets::apply(tab, action),
        Action::EditEssayText { .. } | Action::RemoveUnplaced(_) => essay::apply(tab, action),
        Action::New
        | Action::Edit
        | Action::StopEditing
        | Action::EditText { .. }
        | Action::TypeText(_)
        | Action::Open
        | Action::ImportArgdown
        | Action::Save
        | Action::SaveAs
        | Action::CloseTab(_)
        | Action::CloseWindow
        | Action::ResolveUnsaved(_)
        | Action::ResolveReplace(_)
        | Action::DismissModal
        | Action::ActivateTab(_)
        | Action::SetTheme(_)
        | Action::SetUiLanguage(_)
        | Action::SetConnectorStyle(_)
        | Action::ToggleMarkers
        | Action::SetMinimap(_)
        | Action::ToggleNumbers
        | Action::ToggleShading
        | Action::AddComment { .. }
        | Action::RemoveComment { .. }
        | Action::OpenComments(_)
        | Action::CommentSelected
        | Action::CloseComments
        | Action::SetAuthor(_)
        | Action::Insert { .. }
        | Action::SetCardKind(_)
        | Action::EditLink
        | Action::SetLink(_)
        | Action::OpenLink(_)
        | Action::InsertImage
        | Action::ImagePrepared { .. }
        | Action::RemoveImage
        | Action::TogglePanel(_)
        | Action::ShowPanel(_)
        | Action::SetTextTab(_)
        | Action::SetOutlineOption(..)
        | Action::SetCheckEnabled(..)
        | Action::SetDefaultDocLanguage(_)
        | Action::OpenExport(_)
        | Action::RunExport(_)
        | Action::CloseExportDialog
        | Action::OpenPageSetup
        | Action::SetPageSetup(_)
        | Action::CopyAsImage
        | Action::Print
        | Action::OpenExportFolder
        | Action::ResolveExportReplace(_)
        | Action::ResolveClosingExports(_)
        | Action::OpenCommandSearch
        | Action::OpenFind { .. }
        | Action::CloseFind
        | Action::FindNext
        | Action::FindPrev
        | Action::ReplaceCurrent
        | Action::ReplaceAll
        | Action::SetFindQuery(_)
        | Action::SetReplaceText(_)
        | Action::SetFindOptions(_)
        | Action::OpenSpelling
        | Action::CloseSpelling
        | Action::SpellChange { .. }
        | Action::SpellChangeAll { .. }
        | Action::SpellIgnore
        | Action::SpellIgnoreAll(_)
        | Action::SpellAddWord(_)
        | Action::SetSpellcheck(_)
        | Action::SetAutosaveSecs(_)
        | Action::SetUiScale(_)
        | Action::ResetSettings
        | Action::ConfirmResetSettings
        | Action::ResolveRecovery(_) => unreachable!("app-level action"),
        #[cfg(debug_assertions)]
        Action::LoadSampleMap | Action::ToggleSourceIcons => unreachable!("app-level action"),
    }
}

#[cfg(test)]
mod tests;
