//! Every command the user can give, and the one place that carries them out.

use reasoncraft_core::{
    BoxKind, CardId, CardKind, Item, NewItem, NewLoose, Parent, RemoveMode, can_attach,
};

use crate::app::ReasonCraftApp;
use crate::canvas::input::navigate;
use crate::doc_tab::DocTab;
use crate::theme::ThemeChoice;

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

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    New,
    Open,
    Save,
    SaveAs,
    CloseTab(usize),
    Undo,
    Redo,
    AddReason,
    AddObjection,
    AddCoPremise,
    AddCategory,
    AddNote,
    AddLoose(NewLoose),
    /// Add a card that targets the whole selected card (or the card of the selected premise).
    TargetWholeCard(CardKind),
    Delete,
    DeleteBranch,
    /// Start editing the selected box's text.
    Edit,
    /// Commit the inline edit (if any).
    StopEditing,
    /// The inline editor's text changed: show it live, without a history step.
    EditText(String),
    /// Text typed while a box is selected: start editing it (if needed) and append.
    TypeText(String),
    /// Connect, reattach or detach `item` (drag and drop).
    Move {
        item: Item,
        to: Parent,
    },
    Select(Option<Item>),
    Navigate(Dir),
    ZoomIn,
    ZoomOut,
    ZoomFit,
    Zoom100,
    /// Set the zoom factor directly (status-bar slider), clamped to the zoom range.
    SetZoom(f32),
    SetTheme(ThemeChoice),
    TogglePanel(Panel),
    /// Switch to the tab at this index.
    ActivateTab(usize),
    /// Open the built-in sample map in a new tab (debug builds only).
    #[cfg(debug_assertions)]
    LoadSampleMap,
}

/// Carry out `action`. Document edits go through [`DocTab::edit`], which
/// records history only when the edit succeeds.
///
/// Every action except the inline editor's own text updates first commits a
/// running inline edit, so it lands in history before anything else happens.
pub fn apply(app: &mut ReasonCraftApp, action: Action) {
    if !matches!(action, Action::EditText(_) | Action::TypeText(_))
        && let Some(tab) = app.tabs.get_mut(app.active)
    {
        tab.commit_edit();
    }
    match action {
        Action::New => {
            app.tabs.push(DocTab::new());
            app.active = app.tabs.len() - 1;
        }
        // File operations arrive in Task 11.
        Action::Open | Action::Save | Action::SaveAs => {}
        Action::CloseTab(i) => close_tab(app, i),
        Action::ActivateTab(i) => {
            if i < app.tabs.len() {
                app.active = i;
            }
        }
        Action::SetTheme(choice) => app.theme = choice,
        #[cfg(debug_assertions)]
        Action::LoadSampleMap => {
            let doc = reasoncraft_core::fixtures::sample_iot_like(117);
            app.tabs.push(DocTab::from_document(doc));
            app.active = app.tabs.len() - 1;
        }
        Action::TogglePanel(Panel::Left) => app.left_panel = !app.left_panel,
        Action::TogglePanel(Panel::Right) => app.right_panel = !app.right_panel,
        other => {
            if let Some(tab) = app.tabs.get_mut(app.active) {
                apply_to_tab(tab, other);
            }
        }
    }
}

fn close_tab(app: &mut ReasonCraftApp, i: usize) {
    if i >= app.tabs.len() {
        return;
    }
    app.tabs.remove(i);
    if app.tabs.is_empty() {
        app.tabs.push(DocTab::new());
    }
    if app.active > i || app.active >= app.tabs.len() {
        app.active = app.active.saturating_sub(1);
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
        Action::EditText(text) => tab.set_edit_text(text),
        Action::TypeText(typed) => {
            if tab.editing.is_none()
                && let Some(Item::Box(b)) = tab.selection
            {
                tab.begin_edit(b, false);
            }
            if tab.editing.is_some() {
                let text = format!("{}{typed}", tab.edit_text);
                tab.set_edit_text(text);
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
        }
        Action::ZoomIn => set_zoom(tab, tab.camera.zoom * ZOOM_STEP),
        Action::ZoomOut => set_zoom(tab, tab.camera.zoom / ZOOM_STEP),
        // The canvas fits on its next frame, with the layout of that frame.
        Action::ZoomFit => tab.fit_pending = true,
        Action::Zoom100 => set_zoom(tab, 1.0),
        Action::SetZoom(z) => set_zoom(tab, z),
        Action::New
        | Action::Open
        | Action::Save
        | Action::SaveAs
        | Action::CloseTab(_)
        | Action::ActivateTab(_)
        | Action::SetTheme(_)
        | Action::TogglePanel(_) => unreachable!("app-level action"),
        #[cfg(debug_assertions)]
        Action::LoadSampleMap => unreachable!("app-level action"),
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

/// Select a newly added item and start editing its text (a card's: its
/// first premise). The add itself is the history step for that text.
fn select_new(tab: &mut DocTab, item: Option<Item>) {
    let b = match item {
        Some(Item::Box(b)) => b,
        Some(Item::Card(c)) => tab.doc.cards[&c].premises[0],
        None => return,
    };
    tab.begin_edit(b, true);
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
        Action::AddCoPremise | Action::TargetWholeCard(_) => selected_card(tab).is_some(),
        Action::Delete | Action::DeleteBranch => sel.is_some_and(|s| s != Item::Box(tab.doc.root)),
        Action::Edit => matches!(sel, Some(Item::Box(_))),
        Action::Undo => !tab.history.undo.is_empty() || tab.has_pending_edit(),
        Action::Redo => !tab.history.redo.is_empty(),
        Action::Move { item, to } => {
            tab.doc.owner_of(*item) != Some(*to)
                && tab.doc.clone().move_item(*item, *to, None).is_ok()
        }
        _ => true,
    }
}

/// Selected box → card under it; selected card → card targeting the whole
/// card; nothing selected → loose card in the parking area.
fn add_card(tab: &mut DocTab, kind: CardKind) {
    let premise = match tab.selection {
        Some(Item::Box(b)) => tab
            .edit(|d| d.add_card(Parent::Box(b), kind, ""))
            .map(|(_, p)| p),
        Some(Item::Card(c)) => tab
            .edit(|d| d.add_card(Parent::Card(c), kind, ""))
            .map(|(_, p)| p),
        None => tab.edit(|d| {
            let item = d.add_loose(NewLoose::Card(kind), "");
            Ok(match item {
                Item::Card(c) => d.cards[&c].premises[0],
                Item::Box(b) => b,
            })
        }),
    };
    select_new(tab, premise.map(Item::Box));
}

/// A premise for the selected card, or for the card that owns the selected premise.
fn add_co_premise(tab: &mut DocTab) {
    if let Some(c) = selected_card(tab) {
        let p = tab.edit(|d| d.add_premise(c, None, ""));
        select_new(tab, p.map(Item::Box));
    }
}

/// A category or note under the selected box, or loose when nothing is selected.
fn add_box(tab: &mut DocTab, kind: BoxKind, loose: NewLoose) {
    let item = match tab.selection {
        Some(Item::Box(b)) => tab
            .edit(|d| d.add_box(Parent::Box(b), kind, ""))
            .map(Item::Box),
        Some(Item::Card(_)) => None,
        None => tab.edit(|d| Ok(d.add_loose(loose, ""))),
    };
    select_new(tab, item);
}

/// Remove the selection, then select what held it (or the root).
fn delete(tab: &mut DocTab, mode: RemoveMode) {
    let Some(item) = tab.selection else { return };
    let owner = tab.doc.owner_of(item);
    if tab.edit(|d| d.remove_item(item, mode)).is_none() {
        return;
    }
    tab.selection = match owner {
        Some(Parent::Box(b)) => Some(Item::Box(b)),
        Some(Parent::Card(c)) => Some(Item::Card(c)),
        Some(Parent::Parking) | None => None,
    };
    tab.editing = None;
    tab.fix_selection();
}

#[cfg(test)]
mod tests;
