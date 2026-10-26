//! The Home ribbon tab: clipboard, adding items, editing and the
//! selected box's type, link and image.

use eframe::egui::{Button, Popup, Ui};
use reasoncraft_core::{BoxKind, CardKind, Item, SourceType};

use super::{command, group, large, large_menu, small, small_command};
use crate::actions::{Action, is_enabled, selected_card};
use crate::app::ReasonCraftApp;
use crate::canvas::plus_menu::source_button;
use crate::commands;
use crate::i18n::t;
use crate::shell::icons::Icon;

pub(super) fn show(ui: &mut Ui, app: &mut ReasonCraftApp) {
    let tk = app.tokens;
    let tab = app.active_tab();
    let enabled = |id: &str| is_enabled(tab, &commands::get(id).action);
    let mut out = Vec::new();

    group(ui, &tk, &t("group.clipboard"), |ui| {
        large(ui, &tk, Icon::Paste, &t("action.paste"), false, false);
        ui.vertical(|ui| {
            small(ui, &tk, Icon::Cut, &t("action.cut"), false);
            small(ui, &tk, Icon::Copy, &t("action.copy"), false);
            small_command(
                ui,
                &tk,
                Icon::CopyImage,
                "export.copy_image",
                true,
                &mut out,
            );
        });
    });
    group(ui, &tk, &t("group.add"), |ui| {
        let adds = [
            (Icon::Reason, "home.add_reason"),
            (Icon::Objection, "home.add_objection"),
            (Icon::CoPremise, "home.add_co_premise"),
            (Icon::Category, "home.add_category"),
            (Icon::Note, "home.add_note"),
        ];
        for (icon, id) in adds {
            command(ui, &tk, icon, id, enabled(id), false, &mut out);
        }
        let generic = Action::AddSource(SourceType::Source);
        let label = t("action.add_source");
        let resp = large_menu(ui, &tk, Icon::Source, &label, is_enabled(tab, &generic));
        Popup::menu(&resp).show(|ui| {
            for st in SourceType::ALL {
                let cmd = commands::get(&format!("home.add_source.{}", st.key()));
                let on = is_enabled(tab, &cmd.action);
                if ui.add_enabled(on, source_button(&st)).clicked() {
                    out.push(cmd.action.clone());
                }
            }
        });
        let id = "home.comment";
        command(ui, &tk, Icon::Comment, id, enabled(id), false, &mut out);
    });
    group(ui, &tk, &t("group.editing"), |ui| {
        for (icon, id) in [
            (Icon::Delete, "home.delete"),
            (Icon::DeleteBranch, "home.delete_branch"),
        ] {
            command(ui, &tk, icon, id, enabled(id), false, &mut out);
        }
        let connect = commands::get("home.connect");
        let label = t(connect.label);
        let on = tab.connect_mode.is_some();
        let can = is_enabled(tab, &connect.action) || on;
        if large(ui, &tk, Icon::Connect, &label, can, on).clicked() {
            out.push(if on {
                Action::CancelConnect
            } else {
                connect.action.clone()
            });
        }
        ui.vertical(|ui| {
            for (icon, id) in [(Icon::Undo, "home.undo"), (Icon::Redo, "home.redo")] {
                small_command(ui, &tk, icon, id, enabled(id), &mut out);
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
            (Icon::Link, "home.link"),
            (Icon::Image, "home.image"),
            (Icon::RemoveImage, "home.remove_image"),
        ];
        for (icon, id) in buttons {
            let on = is_enabled(tab, &commands::get(id).action);
            command(ui, &tk, icon, id, on, false, out);
        }
    });
}
