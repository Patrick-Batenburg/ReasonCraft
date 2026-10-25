//! The Home ribbon tab: clipboard, adding items, editing and the
//! selected box's type, link and image.

use eframe::egui::{Button, Popup, Ui};
use reasoncraft_core::{BoxKind, CardKind, Item, SourceType};

use super::{group, large, large_menu, small};
use crate::actions::{Action, is_enabled, selected_card};
use crate::app::ReasonCraftApp;
use crate::canvas::plus_menu::source_button;
use crate::i18n::t;
use crate::shell::icons::Icon;

pub(super) fn show(ui: &mut Ui, app: &mut ReasonCraftApp) {
    let tk = app.tokens;
    let tab = app.active_tab();
    let enabled = |a: &Action| is_enabled(tab, a);
    let mut out = Vec::new();

    group(ui, &tk, &t("group.clipboard"), |ui| {
        large(ui, &tk, Icon::Paste, &t("action.paste"), false, false);
        ui.vertical(|ui| {
            small(ui, &tk, Icon::Cut, &t("action.cut"), false);
            small(ui, &tk, Icon::Copy, &t("action.copy"), false);
            let label = t("action.copy_image");
            if small(ui, &tk, Icon::CopyImage, &label, true).clicked() {
                out.push(Action::CopyAsImage);
            }
        });
    });
    group(ui, &tk, &t("group.add"), |ui| {
        let adds = [
            (Icon::Reason, "action.add_reason", Action::AddReason),
            (
                Icon::Objection,
                "action.add_objection",
                Action::AddObjection,
            ),
            (
                Icon::CoPremise,
                "action.add_co_premise",
                Action::AddCoPremise,
            ),
            (Icon::Category, "action.add_category", Action::AddCategory),
            (Icon::Note, "action.add_note", Action::AddNote),
        ];
        for (icon, key, action) in adds {
            if large(ui, &tk, icon, &t(key), enabled(&action), false).clicked() {
                out.push(action);
            }
        }
        let generic = Action::AddSource(SourceType::Source);
        let label = t("action.add_source");
        let resp = large_menu(ui, &tk, Icon::Source, &label, enabled(&generic));
        Popup::menu(&resp).show(|ui| {
            for st in SourceType::ALL {
                let action = Action::AddSource(st.clone());
                if ui
                    .add_enabled(enabled(&action), source_button(&st))
                    .clicked()
                {
                    out.push(action);
                }
            }
        });
        let comment = Action::CommentSelected;
        let label = t("action.comment");
        if large(ui, &tk, Icon::Comment, &label, enabled(&comment), false).clicked() {
            out.push(comment);
        }
    });
    group(ui, &tk, &t("group.editing"), |ui| {
        let deletes = [
            (Icon::Delete, "action.delete", Action::Delete),
            (
                Icon::DeleteBranch,
                "action.delete_branch",
                Action::DeleteBranch,
            ),
        ];
        for (icon, key, action) in deletes {
            if large(ui, &tk, icon, &t(key), enabled(&action), false).clicked() {
                out.push(action);
            }
        }
        let connect = Action::StartConnect;
        let label = t("action.connect");
        let on = tab.connect_mode.is_some();
        if large(ui, &tk, Icon::Connect, &label, enabled(&connect) || on, on).clicked() {
            out.push(if on { Action::CancelConnect } else { connect });
        }
        ui.vertical(|ui| {
            let history = [
                (Icon::Undo, "action.undo", Action::Undo),
                (Icon::Redo, "action.redo", Action::Redo),
            ];
            for (icon, key, action) in history {
                if small(ui, &tk, icon, &t(key), enabled(&action)).clicked() {
                    out.push(action);
                }
            }
        });
    });
    box_group(ui, app, &mut out);
    app.pending.extend(out);
}

/// "Type ▾" (a card's kind, a source's type), "Link", "Image" and
/// "Remove image".
fn box_group(ui: &mut Ui, app: &ReasonCraftApp, out: &mut Vec<Action>) {
    let tk = app.tokens;
    let tab = app.active_tab();
    let source = match tab.selection {
        Some(Item::Box(b)) => match &tab.doc.boxes.get(&b).map(|n| &n.kind) {
            Some(BoxKind::Source { source_type }) => Some(source_type.clone()),
            _ => None,
        },
        _ => None,
    };
    let card_kinds = [
        (CardKind::Reason, "card_kind.reason"),
        (CardKind::Objection, "card_kind.objection"),
    ];
    let current_kind = selected_card(tab).map(|c| tab.doc.cards[&c].kind.clone());
    group(ui, &tk, &t("group.box"), |ui| {
        let label = t("action.box_type");
        let resp = large_menu(
            ui,
            &tk,
            Icon::BoxType,
            &label,
            current_kind.is_some() || source.is_some(),
        );
        Popup::menu(&resp).show(|ui| {
            if let Some(current) = &source {
                for st in SourceType::ALL {
                    let button = source_button(&st).selected(&st == current);
                    if ui.add(button).clicked() {
                        out.push(Action::SetSourceType(st));
                    }
                }
            } else {
                for (kind, key) in card_kinds {
                    let selected = current_kind.as_ref() == Some(&kind);
                    if ui.add(Button::selectable(selected, t(key))).clicked() {
                        out.push(Action::SetCardKind(kind));
                    }
                }
            }
        });
        let buttons = [
            (Icon::Link, "action.link", Action::EditLink),
            (Icon::Image, "action.image", Action::InsertImage),
            (
                Icon::RemoveImage,
                "action.remove_image",
                Action::RemoveImage,
            ),
        ];
        for (icon, key, action) in buttons {
            if large(ui, &tk, icon, &t(key), is_enabled(tab, &action), false).clicked() {
                out.push(action);
            }
        }
    });
}
