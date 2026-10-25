//! The Home ribbon tab: clipboard, adding items and editing.

use eframe::egui::{Popup, Ui};
use reasoncraft_core::SourceType;

use super::{group, large, large_menu, small};
use crate::actions::{Action, is_enabled};
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
    app.pending.extend(out);
}
