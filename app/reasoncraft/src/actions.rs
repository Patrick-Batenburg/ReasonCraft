//! Every command the user can give, and the one place that carries them out.

use reasoncraft_core::{
    Acceptance, BoxKind, CardId, CardKind, CardStrength, ClearScope, Item, NewItem, NewLoose,
    NodeId, Parent, RemoveMode, SourceRating, SourceType, Uuid, Verdict, can_attach,
};

use crate::app::ReasonCraftApp;
use crate::canvas::input::{navigate, parking_jump};
use crate::doc_tab::DocTab;
use crate::file_ops;
use crate::theme::ThemeChoice;

mod comments;
mod evaluate;
pub use evaluate::{Rate, Rated, rate_action, rated};
mod sources;
mod structure;
use structure::{add_box, add_card, add_co_premise, delete};

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
    /// Connect, reattach or detach `item` (drag and drop).
    Move {
        item: Item,
        to: Parent,
    },
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
pub fn apply(app: &mut ReasonCraftApp, action: Action) {
    if !matches!(action, Action::EditText { .. } | Action::TypeText(_))
        && let Some(tab) = app.tabs.get_mut(app.active)
    {
        tab.commit_edit();
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
        Action::DismissModal => app.modal = None,
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
        Action::Move { item, to } => {
            if tab.edit(|d| d.move_item(item, to, None)).is_some() {
                tab.selection = Some(item);
                tab.fix_selection();
            }
        }
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
fn selected_card(tab: &DocTab) -> Option<CardId> {
    match tab.selection? {
        Item::Card(c) => Some(c),
        Item::Box(b) => match tab.doc.owner_of(Item::Box(b)) {
            Some(Parent::Card(c)) => Some(c),
            _ => None,
        },
    }
}

/// Whether `action` would do something for `tab` right now. Ribbon, "+"
/// menu and keyboard all ask this, so refused commands are greyed out
/// instead of silently doing nothing.
pub fn is_enabled(tab: &DocTab, action: &Action) -> bool {
    if let Some(enabled) = evaluate::is_enabled(tab, action)
        .or_else(|| sources::is_enabled(tab, action))
        .or_else(|| comments::is_enabled(tab, action))
    {
        return enabled;
    }
    let sel = tab.selection;
    let attach = |new: NewItem| match sel {
        Some(Item::Box(b)) => can_attach(&tab.doc, Parent::Box(b), new),
        Some(Item::Card(c)) => can_attach(&tab.doc, Parent::Card(c), new),
        None => true,
    };
    match action {
        Action::AddReason => attach(NewItem::Card(CardKind::Reason)),
        Action::AddObjection => attach(NewItem::Card(CardKind::Objection)),
        Action::AddCategory => attach(NewItem::Category),
        Action::AddNote => attach(NewItem::Note),
        Action::AddCoPremise => selected_card(tab).is_some(),
        Action::TargetWholeCard(kind) => selected_card(tab)
            .is_some_and(|c| can_attach(&tab.doc, Parent::Card(c), NewItem::Card(kind.clone()))),
        Action::Delete | Action::DeleteBranch => sel.is_some_and(|s| s != Item::Box(tab.doc.root)),
        Action::Edit => matches!(sel, Some(Item::Box(_))),
        Action::JumpParking => parking_jump(&tab.doc, sel).is_some(),
        Action::Undo => !tab.history.undo.is_empty() || tab.has_pending_edit(),
        Action::Redo => !tab.history.redo.is_empty(),
        Action::Move { item, to } => {
            tab.doc.owner_of(*item) != Some(*to)
                && tab.doc.clone().move_item(*item, *to, None).is_ok()
        }
        _ => true,
    }
}

#[cfg(test)]
mod tests;
