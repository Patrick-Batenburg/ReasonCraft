//! The Home ribbon tab: clipboard, adding items, editing and the
//! selected box's type, link and image.

use eframe::egui::{Button, Ui};
use reasoncraft_core::{BoxKind, CardKind, Item, SourceType};

use super::{
    command, group, large, large_menu, menu_item, menu_popup, small_column, small_command, tips,
};
use crate::actions::{Action, can_run, is_enabled, selected_card};
use crate::app::ReasonCraftApp;
use crate::canvas::plus_menu::source_button;
use crate::commands;
use crate::doc_tab::DocTab;
use crate::i18n::t;
use crate::shell::icons::Icon;
use crate::theme::Tokens;

pub(super) fn show(ui: &mut Ui, app: &mut ReasonCraftApp) {
    let tk = app.tokens;
    let tab = app.active_tab();
    let enabled = |id: &str| is_enabled(tab, &commands::get(id).action);
    let mut out = Vec::new();

    group(ui, &tk, Icon::Paste, &t("group.clipboard"), |ui| {
        let paste = can_run(app, &commands::get("home.paste").action);
        command(ui, &tk, "home.paste", paste, false, &mut out);
        small_column(ui, |ui| {
            for id in ["home.cut", "home.copy"] {
                small_command(ui, &tk, id, enabled(id), &mut out);
            }
            small_command(ui, &tk, "export.copy_image", true, &mut out);
        });
    });
    group(ui, &tk, Icon::Reason, &t("group.add"), |ui| {
        let adds = [
            "home.add_reason",
            "home.add_objection",
            "home.add_co_premise",
        ];
        for id in adds {
            command(ui, &tk, id, enabled(id), false, &mut out);
        }
        source_menu(ui, &tk, tab, &mut out);
        // The other items in a column of small buttons, so that the Home
        // tab fits 1280 px in the longer Dutch labels too.
        small_column(ui, |ui| {
            let others = ["home.add_category", "home.add_note", "home.comment"];
            for id in others {
                small_command(ui, &tk, id, enabled(id), &mut out);
            }
        });
    });
    group(ui, &tk, Icon::Undo, &t("group.editing"), |ui| {
        small_column(ui, |ui| {
            for id in ["home.delete", "home.delete_branch", "home.park"] {
                small_command(ui, &tk, id, enabled(id), &mut out);
            }
        });
        let connect = commands::get("home.connect");
        let label = t(connect.label);
        let on = tab.connect_mode.is_some();
        let can = is_enabled(tab, &connect.action) || on;
        let resp = large(ui, &tk, connect.ribbon_icon(), &label, can, on);
        if tips::command(resp, connect, can).clicked() {
            out.push(if on {
                Action::CancelConnect
            } else {
                connect.action.clone()
            });
        }
        small_column(ui, |ui| {
            for id in ["home.undo", "home.redo"] {
                small_command(ui, &tk, id, enabled(id), &mut out);
            }
        });
        small_column(ui, |ui| {
            for id in ["home.find", "home.replace"] {
                small_command(ui, &tk, id, true, &mut out);
            }
            let spelling = can_run(app, &commands::get("home.spelling").action);
            small_command(ui, &tk, "home.spelling", spelling, &mut out);
        });
    });
    box_group(ui, app, &mut out);
    app.pending.extend(out);
}

/// "Source ▾": a menu of every source type to add.
pub(super) fn source_menu(ui: &mut Ui, tk: &Tokens, tab: &DocTab, out: &mut Vec<Action>) {
    let generic = Action::AddSource(SourceType::Source);
    let label = t("action.add_source");
    let enabled = is_enabled(tab, &generic);
    let resp = large_menu(ui, tk, Icon::Source, &label, enabled);
    let tip = tips::text(&resp.ctx, &label, &generic, enabled);
    let resp = resp.on_hover_text(tip);
    menu_popup(ui, &resp, |ui| {
        for st in SourceType::ALL {
            let cmd = commands::get(&commands::add_source_id(&st));
            let on = is_enabled(tab, &cmd.action);
            if menu_item(ui, on, source_button(&st)) {
                out.push(cmd.action.clone());
            }
        }
    });
}

/// "Type ▾" (a card's kind, a source's type), then "Link", "Image" and
/// "Remove image" as small buttons.
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
    group(ui, &tk, Icon::BoxType, &t("group.box"), |ui| {
        let label = t("action.box_type");
        let resp = large_menu(
            ui,
            &tk,
            Icon::BoxType,
            &label,
            current_kind.is_some() || source.is_some(),
        );
        menu_popup(ui, &resp, |ui| {
            if let Some(current) = &source {
                for st in SourceType::ALL {
                    let button = source_button(&st).selected(&st == current);
                    if menu_item(ui, true, button) {
                        out.push(Action::SetSourceType(st));
                    }
                }
            } else {
                for (kind, key) in card_kinds {
                    let selected = current_kind.as_ref() == Some(&kind);
                    if menu_item(ui, true, Button::selectable(selected, t(key))) {
                        out.push(Action::SetCardKind(kind));
                    }
                }
            }
        });
        small_column(ui, |ui| {
            let buttons = ["home.link", "home.image", "home.remove_image"];
            for id in buttons {
                let on = is_enabled(tab, &commands::get(id).action);
                small_command(ui, &tk, id, on, out);
            }
        });
    });
}
