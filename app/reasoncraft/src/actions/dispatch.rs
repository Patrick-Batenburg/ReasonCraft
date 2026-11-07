//! The one match that carries out every action: no wildcard, so a new
//! action does not compile until it is given a place here.

use reasoncraft_core::{BoxKind, CardKind, NewLoose, RemoveMode};

use super::structure::{self, add_box, add_card, add_co_premise, delete};
use super::{
    Action, UnsavedChoice, box_group, clipboard, comments, connect, editing, essay, evaluate,
    examples, export, find, help, images, insert, meta, snippets, sources, view,
};
use crate::app::ReasonCraftApp;
use crate::doc_tab::DocTab;
use crate::file_ops;
use crate::i18n::t;
use crate::settings::Settings;

/// Run `f` on the active tab, if there is one.
fn on_tab(app: &mut ReasonCraftApp, f: impl FnOnce(&mut DocTab)) {
    if let Some(tab) = app.tabs.get_mut(app.active) {
        f(tab);
    }
}

/// Run `f` on the active tab; when it reports that box text was cut at
/// the limit, say so in the status bar.
fn noting_cut(app: &mut ReasonCraftApp, f: impl FnOnce(&mut DocTab) -> bool) {
    if let Some(tab) = app.tabs.get_mut(app.active)
        && f(tab)
    {
        app.status_note = Some(t("status.text_cut"));
    }
}

/// Switch `step` tabs to the right, round from the last to the first.
fn step_tab(app: &mut ReasonCraftApp, step: usize) {
    if !app.tabs.is_empty() {
        app.active = (app.active + step) % app.tabs.len();
    }
}

/// Close tab `i`, asking first if it has unsaved work. A finished image
/// lands first, so the prompt counts it.
fn close_tab(app: &mut ReasonCraftApp, i: usize) {
    let Some(key) = app.tabs.get(i).map(|t| t.recovery_key) else {
        return;
    };
    images::flush_tab(app, i);
    file_ops::request_close_tab(app, key);
}

/// Carry out `action` (after [`super::apply`] has ended what it ends).
pub(super) fn run(app: &mut ReasonCraftApp, action: Action) {
    match action {
        // Files, tabs and messages.
        Action::New => {
            let lang = app.settings.new_doc_language(app.os_locale);
            app.tabs.push(DocTab::with_language(lang));
            app.active = app.tabs.len() - 1;
        }
        Action::Open => file_ops::open_with_dialog(app),
        Action::ImportArgdown => super::argdown::import_with_dialog(app),
        Action::Save => file_ops::save_active(app, false),
        Action::SaveAs => file_ops::save_active(app, true),
        Action::CloseTab(i) => close_tab(app, i),
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
        Action::DismissModal => app.modal = None,
        Action::CloseLinkEdit => app.link_edit = None,
        Action::ActivateTab(i) => {
            if i < app.tabs.len() {
                app.active = i;
            }
        }
        Action::NextTab => step_tab(app, 1),
        Action::PrevTab => step_tab(app, app.tabs.len().saturating_sub(1)),
        Action::CloseActiveTab => close_tab(app, app.active),
        Action::ResolveRecovery(choice) => crate::recovery::resolve(app, choice),
        Action::OpenExample(which) => examples::open_example(app, which),
        Action::OpenTemplate(which) => examples::open_template(app, which),
        Action::OpenRecent(path) => file_ops::open_recent(app, &path),

        // History and structure.
        Action::Undo => on_tab(app, DocTab::undo),
        Action::Redo => on_tab(app, DocTab::redo),
        Action::AddReason => on_tab(app, |t| add_card(t, CardKind::Reason)),
        Action::AddObjection => on_tab(app, |t| add_card(t, CardKind::Objection)),
        Action::AddCoPremise => on_tab(app, add_co_premise),
        Action::AddCategory => on_tab(app, |t| add_box(t, BoxKind::Category, NewLoose::Category)),
        Action::AddNote => on_tab(app, |t| add_box(t, BoxKind::Note, NewLoose::Note)),
        Action::AddLoose(kind) => on_tab(app, |t| structure::add_loose(t, kind)),
        Action::TargetWholeCard(kind) => on_tab(app, |t| structure::target_whole_card(t, kind)),
        Action::Delete => on_tab(app, |t| delete(t, RemoveMode::KeepChildren)),
        Action::DeleteBranch => on_tab(app, |t| delete(t, RemoveMode::Branch)),
        Action::Move { item, to, index } => {
            on_tab(app, |t| structure::move_item(t, item, to, index));
        }
        Action::Reorder(screen) => on_tab(app, |t| structure::reorder(t, screen)),
        Action::StartConnect => on_tab(app, connect::start),
        Action::MoveToParking => on_tab(app, |t| {
            if let Some(Action::Move { item, to, index }) = connect::park_move(t) {
                structure::move_item(t, item, to, index);
            }
        }),
        // `apply` has already ended the mode.
        Action::CancelConnect => {}
        Action::Insert { what, to, on } => insert::apply(app, what, to, on),

        // Box text.
        Action::Edit => on_tab(app, editing::start),
        // `apply` has already committed the edit.
        Action::StopEditing => {}
        Action::EditText { node, text } => noting_cut(app, |t| t.set_edit_text(node, text)),
        Action::TypeText(typed) => noting_cut(app, |t| editing::type_text(t, &typed)),

        // The Box group, images and sources.
        Action::SetCardKind(kind) => on_tab(app, |t| box_group::set_card_kind(t, kind)),
        Action::EditLink => box_group::edit_link(app),
        Action::SetLink(link) => box_group::set_link(app, link),
        Action::OpenLink(b) => box_group::open_link(app, b),
        Action::OpenSelectedLink => box_group::open_selected_link(app),
        Action::InsertImage => images::insert(app),
        Action::RemoveImage => on_tab(app, images::remove),
        Action::ImagePrepared { job, node, result } => images::prepared(app, job, node, result),
        Action::AddSource(kind) => on_tab(app, |t| sources::add(t, kind)),
        Action::SetSourceType(kind) => on_tab(app, |t| sources::set_type(t, kind)),

        // Evaluation and comments.
        Action::Rate(rate) => on_tab(app, |t| evaluate::rate(t, rate)),
        Action::SetAcceptance(v) => on_tab(app, |t| evaluate::set_acceptance(t, v)),
        Action::SetStrength(v) => on_tab(app, |t| evaluate::set_strength(t, v)),
        Action::SetSourceRating(v) => on_tab(app, |t| evaluate::set_source_rating(t, v)),
        Action::ClearEvaluation(scope) => on_tab(app, |t| evaluate::clear(t, scope)),
        Action::AddComment {
            node,
            text,
            verdict,
        } => comments::add(app, node, &text, verdict),
        Action::RemoveComment { node, comment } => {
            on_tab(app, |t| comments::remove(t, node, comment));
        }
        Action::OpenComments(node) => on_tab(app, |t| comments::open(t, node)),
        Action::CommentSelected => on_tab(app, comments::comment_selected),
        Action::CloseComments => on_tab(app, |t| t.comments_open = None),
        Action::SetAuthor(name) => app.settings.author = name.trim().to_owned(),

        // Snippets, the essay and the document's metadata.
        Action::AddSnippet(text) => on_tab(app, |t| snippets::add(t, &text)),
        Action::EditSnippetText { id, text } => on_tab(app, |t| t.set_snippet_text(id, &text)),
        Action::EditSnippetCitation { id, citation } => {
            on_tab(app, |t| snippets::edit_citation(t, id, citation));
        }
        Action::EditSnippetLink { id, link } => on_tab(app, |t| snippets::edit_link(t, id, link)),
        Action::SetSnippetSource { id, source } => {
            on_tab(app, |t| snippets::set_source(t, id, source));
        }
        Action::RemoveSnippet(id) => on_tab(app, |t| snippets::remove(t, id)),
        Action::MoveSnippet { id, index } => on_tab(app, |t| snippets::move_to(t, id, index)),
        Action::EditEssayText { slot, text } => {
            on_tab(app, |t| t.set_essay_text_live(slot, &text));
        }
        Action::RemoveUnplaced(index) => on_tab(app, |t| essay::remove_unplaced(t, index)),
        Action::SetDocMeta { title, author } => {
            on_tab(app, |t| t.edit_meta_live(&title, &author));
        }
        Action::SetDocLanguage(lang) => on_tab(app, |t| meta::set_language(t, lang)),
        Action::IgnoreHint(issue) => on_tab(app, |t| {
            t.edit(|d| d.ignore_hint(&issue));
        }),
        Action::RestoreHints => on_tab(app, |t| {
            t.edit(|d| d.restore_hints());
        }),

        // Moving around the map.
        Action::Select(item) => on_tab(app, |t| t.selection = item.filter(|i| t.contains(*i))),
        Action::RevealSelection => on_tab(app, |t| t.reveal_pending = true),
        Action::Navigate(screen) => on_tab(app, |t| view::navigate_tab(t, screen)),
        Action::JumpParking => on_tab(app, view::jump_parking),
        Action::ZoomIn => on_tab(app, |t| view::zoom_step(t, true)),
        Action::ZoomOut => on_tab(app, |t| view::zoom_step(t, false)),
        // The canvas fits on its next frame, with the layout of that frame.
        Action::ZoomFit => on_tab(app, |t| t.fit_pending = true),
        Action::ZoomFitSelection => {
            on_tab(app, |t| t.fit_selection_pending = t.selection.is_some())
        }
        Action::Zoom100 => on_tab(app, |t| view::set_zoom(t, 1.0)),
        Action::SetZoom(z) => on_tab(app, |t| view::set_zoom(t, z)),
        Action::SetLayoutDirection(d) => on_tab(app, |t| view::set_layout_direction(t, d)),

        // View preferences and personal settings (no history).
        Action::SetTheme(choice) => app.settings.theme = choice,
        Action::SetUiLanguage(lang) => view::set_ui_language(app, lang),
        Action::SetConnectorStyle(style) => app.settings.connector_style = style,
        Action::ToggleMarkers => app.settings.show_markers = !app.settings.show_markers,
        Action::SetMinimap(mode) => app.settings.minimap = mode,
        Action::ToggleNumbers => app.settings.show_numbers = !app.settings.show_numbers,
        Action::ToggleShading => app.settings.eval_shading = !app.settings.eval_shading,
        Action::TogglePanel(panel) => {
            let shown = view::panel(app, panel);
            *shown = !*shown;
        }
        Action::ShowPanel(panel) => *view::panel(app, panel) = true,
        Action::SetTextTab(tab) => app.settings.text_tab = tab,
        Action::SetOutlineOption(toggle, on) => app.settings.set_outline_toggle(toggle, on),
        Action::SetCheckEnabled(kind, on) => app.settings.set_check_enabled(kind, on),
        Action::SetDefaultDocLanguage(lang) => view::set_default_doc_language(app, lang),
        Action::SetSpellcheck(on) => view::set_spellcheck(app, on),
        Action::SetAutosaveSecs(secs) => {
            app.settings.autosave_secs = Settings::clamp_autosave(secs)
        }
        Action::SetUiScale(scale) => app.settings.ui_scale = Settings::clamp_ui_scale(scale),
        Action::ResetSettings => app.modal = Some(file_ops::Modal::ResetSettings),
        Action::ConfirmResetSettings => {
            app.modal = None;
            app.settings.reset_to_defaults();
            app.apply_ui_language();
        }
        Action::NextPane => app.panes.request = Some(true),
        Action::PrevPane => app.panes.request = Some(false),
        // A running inline edit was committed by `apply`, so the box
        // editor lets go of the keyboard.
        Action::OpenCommandSearch => {
            if let Some(ctx) = &app.egui_ctx {
                crate::shell::command_search::focus(ctx);
            }
        }

        // The clipboard.
        Action::Copy => clipboard::copy(app),
        Action::Cut => clipboard::cut(app),
        Action::Paste(text) => clipboard::paste(app, text),

        // Exports.
        Action::OpenExport(format) => export::open(app, format),
        Action::RunExport(request) => export::run_request(app, request),
        Action::CloseExportDialog => app.export_dialog = None,
        Action::OpenPageSetup => export::open_page_setup(app),
        Action::SetPageSetup(page) => export::set_page_setup(app, page),
        Action::CopyAsImage => export::copy_as_image(app),
        Action::Print => export::print(app),
        Action::OpenExportFolder => export::open_folder(app),
        Action::ResolveExportReplace(replace) => export::resolve_replace(app, replace),
        Action::ResolveClosingExports(quit) => export::resolve_closing(app, quit),

        // Find and replace, spelling.
        Action::OpenFind { replace } => on_tab(app, |t| find::open(t, replace)),
        Action::CloseFind => on_tab(app, find::close),
        Action::FindNext => on_tab(app, |t| find::step(t, true)),
        Action::FindPrev => on_tab(app, |t| find::step(t, false)),
        Action::ReplaceCurrent => find::replace_one(app),
        Action::ReplaceAll => find::replace_every(app),
        Action::SetFindQuery(query) => on_tab(app, |t| find::set_query(t, query)),
        Action::SetReplaceText(text) => on_tab(app, |t| t.find.replacement = text),
        Action::SetFindOptions(opts) => on_tab(app, |t| find::set_options(t, opts)),
        Action::OpenSpelling => crate::spell::open(app),
        Action::CloseSpelling => on_tab(app, crate::spell::close),
        Action::SpellChange { hit, with } => crate::spell::change(app, hit, &with),
        Action::SpellChangeAll { word, with } => crate::spell::change_all(app, &word, &with),
        Action::SpellIgnore => crate::spell::ignore(app),
        Action::SpellIgnoreAll(word) => crate::spell::ignore_all(app, &word),
        Action::SpellAddWord(word) => crate::spell::add_word(app, &word),

        // Help.
        Action::OpenHelp(page) => help::show(app, page),
        Action::ContextHelp => help::context(app),
        Action::HelpBack => app.help.back(),
        Action::ShowAbout => help::show_about(app),

        #[cfg(debug_assertions)]
        Action::LoadSampleMap => {
            let doc = reasoncraft_core::fixtures::sample_iot_like(117);
            app.tabs.push(DocTab::from_document(doc));
            app.active = app.tabs.len() - 1;
        }
        #[cfg(debug_assertions)]
        Action::ToggleSourceIcons => app.show_source_icons = !app.show_source_icons,
    }
}
