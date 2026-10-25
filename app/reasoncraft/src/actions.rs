//! Every command the user can give, and the one place that carries them out.

use reasoncraft_core::{BoxKind, CardKind, Item, NewLoose, Parent, RemoveMode};

use crate::app::ReasonCraftApp;
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
    Edit,
    StopEditing,
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
pub fn apply(app: &mut ReasonCraftApp, action: Action) {
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
            let card = match tab.selection {
                Some(Item::Card(c)) => Some(c),
                Some(Item::Box(b)) => match tab.doc.owner_of(Item::Box(b)) {
                    Some(Parent::Card(c)) => Some(c),
                    _ => None,
                },
                None => None,
            };
            if let Some(c) = card {
                let new = tab.edit(|d| d.add_card(Parent::Card(c), kind, ""));
                select_new(tab, new.map(|(_, p)| Item::Box(p)));
            }
        }
        Action::Delete => delete(tab, RemoveMode::KeepChildren),
        Action::DeleteBranch => delete(tab, RemoveMode::Branch),
        Action::Edit => {
            if let Some(Item::Box(b)) = tab.selection {
                tab.editing = Some(b);
            }
        }
        Action::StopEditing => tab.editing = None,
        Action::Select(item) => {
            tab.selection = item.filter(|i| tab.contains(*i));
            tab.editing = None;
        }
        // Keyboard navigation arrives in Task 10.
        Action::Navigate(_) => {}
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

fn select_new(tab: &mut DocTab, item: Option<Item>) {
    if let Some(item) = item {
        tab.selection = Some(item);
        tab.editing = None;
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
    let card = match tab.selection {
        Some(Item::Card(c)) => Some(c),
        Some(Item::Box(b))
            if tab
                .doc
                .boxes
                .get(&b)
                .is_some_and(|n| n.kind == BoxKind::Premise) =>
        {
            match tab.doc.owner_of(Item::Box(b)) {
                Some(Parent::Card(c)) => Some(c),
                _ => None,
            }
        }
        _ => None,
    };
    if let Some(c) = card {
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
mod tests {
    use super::*;

    fn app() -> ReasonCraftApp {
        ReasonCraftApp::new_for_test()
    }

    fn tab(app: &ReasonCraftApp) -> &DocTab {
        &app.tabs[app.active]
    }

    #[test]
    fn add_reason_to_root_selects_new_premise() {
        let mut a = app();
        apply(&mut a, Action::AddReason);
        let t = tab(&a);
        assert_eq!(t.doc.cards.len(), 1);
        let card = t.doc.cards.values().next().unwrap();
        assert_eq!(t.selection, Some(Item::Box(card.premises[0])));
        assert_eq!(t.history.undo.len(), 1);
    }

    #[test]
    fn co_premise_goes_to_owning_card() {
        let mut a = app();
        apply(&mut a, Action::AddReason);
        apply(&mut a, Action::AddCoPremise);
        let t = tab(&a);
        let card = t.doc.cards.values().next().unwrap();
        assert_eq!(card.premises.len(), 2);
        assert_eq!(t.selection, Some(Item::Box(card.premises[1])));
    }

    #[test]
    fn whole_card_target_and_card_selection() {
        let mut a = app();
        apply(&mut a, Action::AddReason);
        let c = *tab(&a).doc.cards.keys().next().unwrap();
        apply(&mut a, Action::TargetWholeCard(CardKind::Objection));
        assert_eq!(tab(&a).doc.cards[&c].children.len(), 1);
        apply(&mut a, Action::Select(Some(Item::Card(c))));
        apply(&mut a, Action::AddReason);
        assert_eq!(tab(&a).doc.cards[&c].children.len(), 2);
    }

    #[test]
    fn nothing_selected_adds_loose_items() {
        let mut a = app();
        apply(&mut a, Action::Select(None));
        apply(&mut a, Action::AddObjection);
        apply(&mut a, Action::Select(None));
        apply(&mut a, Action::AddNote);
        assert_eq!(tab(&a).doc.parking.len(), 2);
    }

    #[test]
    fn refused_edit_records_nothing() {
        let mut a = app();
        // Root takes cards, so a category is refused afterwards.
        apply(&mut a, Action::AddReason);
        let root = tab(&a).doc.root;
        apply(&mut a, Action::Select(Some(Item::Box(root))));
        apply(&mut a, Action::AddCategory);
        assert_eq!(tab(&a).history.undo.len(), 1);
        apply(&mut a, Action::Delete);
        assert_eq!(tab(&a).history.undo.len(), 1, "root cannot be deleted");
    }

    #[test]
    fn undo_restores_and_fixes_selection() {
        let mut a = app();
        apply(&mut a, Action::AddReason);
        apply(&mut a, Action::Undo);
        let t = tab(&a);
        assert!(t.doc.cards.is_empty());
        assert_eq!(t.selection, Some(Item::Box(t.doc.root)));
        apply(&mut a, Action::Redo);
        assert_eq!(tab(&a).doc.cards.len(), 1);
    }

    #[test]
    fn delete_selects_owner() {
        let mut a = app();
        apply(&mut a, Action::AddReason);
        apply(&mut a, Action::Delete);
        let t = tab(&a);
        assert!(t.doc.cards.is_empty());
        assert_eq!(t.selection, Some(Item::Box(t.doc.root)));
    }

    #[test]
    fn close_last_tab_leaves_a_fresh_one() {
        let mut a = app();
        apply(&mut a, Action::New);
        assert_eq!(a.active, 1);
        apply(&mut a, Action::CloseTab(1));
        assert_eq!((a.tabs.len(), a.active), (1, 0));
        apply(&mut a, Action::CloseTab(0));
        assert_eq!((a.tabs.len(), a.active), (1, 0));
    }

    #[test]
    fn zoom_is_clamped() {
        let mut a = app();
        apply(&mut a, Action::SetZoom(100.0));
        assert_eq!(tab(&a).camera.zoom, ZOOM_MAX);
        apply(&mut a, Action::Zoom100);
        apply(&mut a, Action::ZoomIn);
        assert!((tab(&a).camera.zoom - 1.25).abs() < 1e-6);
    }

    #[test]
    fn zoom_buttons_keep_view_centre_fixed() {
        use eframe::egui::{Rect, pos2, vec2};
        let mut a = app();
        let view = Rect::from_min_size(pos2(220.0, 140.0), vec2(800.0, 500.0));
        a.tabs[0].viewport = Some(view);
        a.tabs[0].camera.offset = vec2(300.0, 180.0);
        let world = a.tabs[0].camera.screen_to_world(view.center());
        apply(&mut a, Action::ZoomIn);
        apply(&mut a, Action::ZoomIn);
        apply(&mut a, Action::ZoomOut);
        let cam = tab(&a).camera;
        assert!((cam.zoom - 1.25).abs() < 1e-5);
        assert!((cam.world_to_screen(world) - view.center()).length() < 1e-3);
        a.tabs[0].fit_pending = false;
        apply(&mut a, Action::ZoomFit);
        assert!(tab(&a).fit_pending);
    }
}
