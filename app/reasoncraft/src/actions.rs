//! Every command the user can give, and the one place that carries them out.

use reasoncraft_core::{
    Acceptance, BoxKind, CardId, CardKind, CardStrength, ClearScope, Item, NewLoose, NodeId,
    Parent, RemoveMode, SourceRating, SourceType, Uuid, Verdict,
};

use crate::app::ReasonCraftApp;
use crate::canvas::input::{navigate, parking_jump};
use crate::doc_tab::DocTab;
use crate::file_ops;
use crate::theme::ThemeChoice;

mod box_group;
mod enabled;
pub use box_group::{FakeOpener, LinkOpener, SystemOpener};
pub use enabled::is_enabled;
mod comments;
mod evaluate;
mod images;
pub use evaluate::{Rate, Rated, rate_action, rated};
mod insert;
pub use insert::{Insertable, target_allowed};
mod connect;
pub use connect::connect_item;
mod sources;
mod structure;
use structure::{add_box, add_card, add_co_premise, delete};
pub use structure::{is_attachment, moves_nowhere, sibling_list};

pub use crate::canvas::camera::{ZOOM_MAX, ZOOM_MIN};
const ZOOM_STEP: f32 = 1.25;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Panel {
    Left,
    Right,
}

/// The answer to "save changes before closing?".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnsavedChoice {
    Save,
    Discard,
    Cancel,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    New,
    Open,
    Save,
    SaveAs,
    /// Close the tab at this index, asking first if it has unsaved changes.
    CloseTab(usize),
    /// Close the window, asking first about each tab with unsaved changes.
    CloseWindow,
    ResolveUnsaved(UnsavedChoice),
    /// The answer to "replace the existing file?" (true: replace).
    ResolveReplace(bool),
    /// Close the error message.
    DismissModal,
    Undo,
    Redo,
    AddReason,
    AddObjection,
    AddCoPremise,
    AddCategory,
    AddNote,
    AddLoose(NewLoose),
    /// Add a source of this type under the selected claim or premise, or
    /// loose when nothing is selected.
    AddSource(SourceType),
    /// Change the type of the selected source.
    SetSourceType(SourceType),
    /// Make the selected card (or the card of the selected premise) a
    /// reason or an objection; its strength is kept.
    SetCardKind(CardKind),
    /// Open the link editor for the selected box.
    EditLink,
    /// Set (`None`: remove) the link of the box being edited, or of the
    /// selected box, and close the link editor.
    SetLink(Option<String>),
    /// Open box `node`'s link in the browser (a click on its link glyph).
    OpenLink(NodeId),
    /// Pick an image file and put it in the selected box (replacing its image).
    InsertImage,
    RemoveImage,
    /// Add a card that targets the whole selected card (or the card of the selected premise).
    TargetWholeCard(CardKind),
    Delete,
    DeleteBranch,
    /// Start editing the selected box's text.
    Edit,
    /// Commit the inline edit (if any).
    StopEditing,
    /// The inline editor of box `node` changed its text: show it live,
    /// without a history step. Ignored unless `node` is still being edited.
    EditText {
        node: NodeId,
        text: String,
    },
    /// Text typed while a box is selected: start editing it (if needed) and append.
    TypeText(String),
    /// Connect, reattach, detach or reorder `item` (drag and drop, "Connect
    /// to…"). `index` counts `to`'s children with `item` taken out; `None`
    /// appends.
    Move {
        item: Item,
        to: Parent,
        index: Option<usize>,
    },
    /// Move the selection (a premise within its card) one place left or
    /// right among its siblings (Alt+Left/Right).
    Reorder(Dir),
    /// Pick a target for the selected parking item ("Connect to…").
    StartConnect,
    CancelConnect,
    Select(Option<Item>),
    Navigate(Dir),
    /// Jump between the main tree's root and the first parked item.
    JumpParking,
    ZoomIn,
    ZoomOut,
    ZoomFit,
    Zoom100,
    /// Set the zoom factor directly (status-bar slider), clamped to the zoom range.
    SetZoom(f32),
    SetTheme(ThemeChoice),
    /// Rate the selection, whatever its kind (Alt+1/2/3/0).
    Rate(Rate),
    /// Accept, doubt or reject the selected claim or premise (`None` clears).
    SetAcceptance(Option<Acceptance>),
    /// Rate the selected card's strength (`None` clears).
    SetStrength(Option<CardStrength>),
    /// Rate the selected source (`None` clears).
    SetSourceRating(Option<SourceRating>),
    /// Clear one item's rating or every rating in the map.
    ClearEvaluation(ClearScope),
    /// Show or hide rating markers.
    ToggleMarkers,
    /// Turn evaluation shading on or off.
    ToggleShading,
    /// Comment on box `node` as the author in the settings. Blank text is refused.
    AddComment {
        node: NodeId,
        text: String,
        verdict: Option<Verdict>,
    },
    RemoveComment {
        node: NodeId,
        comment: Uuid,
    },
    /// Show the comment popover of a box (and select it).
    OpenComments(NodeId),
    /// Show the comment popover of the selected box (ribbon, Ctrl+Shift+M).
    CommentSelected,
    CloseComments,
    /// Set the name put on new comments (trimmed).
    SetAuthor(String),
    /// Insert from the building panel: under `to` (cards, categories, notes,
    /// sources), on box `on` (a co-premise joins its card; a comment goes
    /// on it), or at the selection like the ribbon when both are `None`.
    Insert {
        what: Insertable,
        to: Option<Parent>,
        on: Option<NodeId>,
    },
    TogglePanel(Panel),
    /// Switch to the tab at this index.
    ActivateTab(usize),
    /// Open the built-in sample map in a new tab (debug builds only).
    #[cfg(debug_assertions)]
    LoadSampleMap,
    /// Show or hide the source icon preview (debug builds only).
    #[cfg(debug_assertions)]
    ToggleSourceIcons,
}

/// Carry out `action`. Document edits go through [`DocTab::edit`], which
/// records history only when the edit succeeds.
///
/// Every action except the inline editor's own text updates first commits a
/// running inline edit, so it lands in history before anything else happens.
/// Every action but picking a target then ends "Connect to…" in the active
/// tab, app-level ones (inserting, comments, links, switching tabs) too.
pub fn apply(app: &mut ReasonCraftApp, action: Action) {
    if let Some(tab) = app.tabs.get_mut(app.active) {
        if !matches!(action, Action::EditText { .. } | Action::TypeText(_)) {
            tab.commit_edit();
        }
        connect::end_unless_kept(tab, &action);
    }
    match action {
        Action::New => {
            app.tabs.push(DocTab::new());
            app.active = app.tabs.len() - 1;
        }
        Action::Open => file_ops::open_with_dialog(app),
        Action::Save => file_ops::save_active(app, false),
        Action::SaveAs => file_ops::save_active(app, true),
        Action::CloseTab(i) => file_ops::request_close_tab(app, i),
        Action::CloseWindow => file_ops::request_close_window(app),
        Action::ResolveUnsaved(choice) => file_ops::resolve_unsaved(app, choice),
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
        Action::SetTheme(choice) => app.settings.theme = choice,
        Action::ToggleMarkers => app.settings.show_markers = !app.settings.show_markers,
        Action::ToggleShading => app.settings.eval_shading = !app.settings.eval_shading,
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
        #[cfg(debug_assertions)]
        Action::LoadSampleMap => {
            let doc = reasoncraft_core::fixtures::sample_iot_like(117);
            app.tabs.push(DocTab::from_document(doc));
            app.active = app.tabs.len() - 1;
        }
        #[cfg(debug_assertions)]
        Action::ToggleSourceIcons => app.show_source_icons = !app.show_source_icons,
        Action::TogglePanel(Panel::Left) => app.left_panel = !app.left_panel,
        Action::TogglePanel(Panel::Right) => app.right_panel = !app.right_panel,
        other => {
            if let Some(tab) = app.tabs.get_mut(app.active) {
                apply_to_tab(tab, other);
            }
        }
    }
}

fn apply_to_tab(tab: &mut DocTab, action: Action) {
    match action {
        Action::Undo => {
            tab.history.undo(&mut tab.doc);
            tab.fix_selection();
        }
        Action::Redo => {
            tab.history.redo(&mut tab.doc);
            tab.fix_selection();
        }
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
        Action::Edit => {
            if let Some(Item::Box(b)) = tab.selection {
                tab.begin_edit(b, false);
            }
        }
        // `apply` has already committed the edit.
        Action::StopEditing => {}
        Action::EditText { node, text } => tab.set_edit_text(node, text),
        Action::TypeText(typed) => {
            if tab.editing.is_none()
                && let Some(Item::Box(b)) = tab.selection
            {
                tab.begin_edit(b, false);
            }
            if let Some(b) = tab.editing {
                let text = format!("{}{typed}", tab.edit_text);
                tab.set_edit_text(b, text);
            }
        }
        Action::Move { .. } | Action::Reorder(_) => structure::apply(tab, action),
        Action::StartConnect | Action::CancelConnect => connect::apply(tab, action),
        Action::Select(item) => tab.selection = item.filter(|i| tab.contains(*i)),
        Action::Navigate(dir) => {
            tab.selection = match tab.selection {
                Some(from) => navigate(&tab.doc, from, dir).or(Some(from)),
                None => Some(Item::Box(tab.doc.root)),
            };
            tab.reveal_pending = true;
        }
        Action::JumpParking => {
            if let Some(to) = parking_jump(&tab.doc, tab.selection) {
                tab.selection = Some(to);
                tab.reveal_pending = true;
            }
        }
        Action::ZoomIn => set_zoom(tab, tab.camera.zoom * ZOOM_STEP),
        Action::ZoomOut => set_zoom(tab, tab.camera.zoom / ZOOM_STEP),
        // The canvas fits on its next frame, with the layout of that frame.
        Action::ZoomFit => tab.fit_pending = true,
        Action::Zoom100 => set_zoom(tab, 1.0),
        Action::SetZoom(z) => set_zoom(tab, z),
        Action::Rate(_)
        | Action::SetAcceptance(_)
        | Action::SetStrength(_)
        | Action::SetSourceRating(_)
        | Action::ClearEvaluation(_) => evaluate::apply(tab, action),
        Action::AddSource(_) | Action::SetSourceType(_) => sources::apply(tab, action),
        Action::New
        | Action::Open
        | Action::Save
        | Action::SaveAs
        | Action::CloseTab(_)
        | Action::CloseWindow
        | Action::ResolveUnsaved(_)
        | Action::ResolveReplace(_)
        | Action::DismissModal
        | Action::ActivateTab(_)
        | Action::SetTheme(_)
        | Action::ToggleMarkers
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
        | Action::RemoveImage
        | Action::TogglePanel(_) => unreachable!("app-level action"),
        #[cfg(debug_assertions)]
        Action::LoadSampleMap | Action::ToggleSourceIcons => unreachable!("app-level action"),
    }
}

/// Zoom around the centre of the view (or the world origin before the
/// canvas has been shown), clamped to the zoom range.
fn set_zoom(tab: &mut DocTab, zoom: f32) {
    let centre = tab
        .viewport
        .map_or(eframe::egui::Pos2::ZERO, |v| v.center());
    let factor = zoom.clamp(ZOOM_MIN, ZOOM_MAX) / tab.camera.zoom;
    tab.camera.zoom_at(centre, factor);
}

/// Select a newly added item, scroll it into view and start editing its
/// text (a card's: its first premise). The add itself is the history step
/// for that text.
fn select_new(tab: &mut DocTab, item: Option<Item>) {
    let b = match item {
        Some(Item::Box(b)) => b,
        Some(Item::Card(c)) => tab.doc.cards[&c].premises[0],
        None => return,
    };
    tab.begin_edit(b, true);
    tab.reveal_pending = true;
}

/// The card a whole-card target or co-premise goes to: the selected card,
/// or the card that owns the selected premise.
pub fn selected_card(tab: &DocTab) -> Option<CardId> {
    match tab.selection? {
        Item::Card(c) => Some(c),
        Item::Box(b) => match tab.doc.owner_of(Item::Box(b)) {
            Some(Parent::Card(c)) => Some(c),
            _ => None,
        },
    }
}

#[cfg(test)]
mod tests;
