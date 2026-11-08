//! Ribbon tooltips: a command's label with its shortcut, "Paste
//! (Ctrl+V)", written as Search commands writes it; for a greyed-out add
//! command, also why ("Select a premise to add a co-premise.").
//!
//! The ribbon's buttons are drawn without the app at hand, so each frame
//! [`prepare`] notes the platform and the reasons of the moment in the
//! context's memory, and the buttons read them from there.

use eframe::egui::{Context, Id, Response};

use crate::actions::{Action, EXPLAINED_DISABLED, why_disabled};
use crate::canvas::input::{Os, shortcut_for, shortcut_text};
use crate::commands::Command;
use crate::doc_tab::DocTab;
use crate::i18n::t;

/// What the tooltips need this frame.
#[derive(Clone)]
struct Tips {
    os: Os,
    /// Each explained action that is greyed out, with why (translated).
    why: Vec<(Action, String)>,
}

fn id() -> Id {
    Id::unique("ribbon_tips")
}

/// Notes this frame's platform and reasons; the ribbon calls it first.
pub(super) fn prepare(ctx: &Context, os: Os, tab: &DocTab) {
    let why = EXPLAINED_DISABLED
        .iter()
        .map(|ctor| ctor())
        .filter_map(|a| why_disabled(tab, &a).map(|key| (a, t(key))))
        .collect();
    ctx.data_mut(|d| d.insert_temp(id(), Tips { os, why }));
}

/// The tooltip of `label` for `action`: "Label (shortcut)", or the
/// label alone; then, when it is greyed out and the reason is known,
/// the reason on a line of its own.
pub(super) fn text(ctx: &Context, label: &str, action: &Action, enabled: bool) -> String {
    let tips: Option<Tips> = ctx.data(|d| d.get_temp(id()));
    let Some(tips) = tips else {
        return label.to_owned();
    };
    let mut text = match shortcut_for(action, tips.os) {
        Some(s) => format!("{label} ({})", shortcut_text(s, tips.os)),
        None => label.to_owned(),
    };
    if !enabled && let Some((_, why)) = tips.why.iter().find(|(a, _)| a == action) {
        text.push('\n');
        text.push_str(why);
    }
    text
}

/// `resp` with the tooltip of command `cmd` (see [`text`]).
pub(super) fn command(resp: Response, cmd: &Command, enabled: bool) -> Response {
    let tip = text(&resp.ctx, &t(cmd.label), &cmd.action, enabled);
    resp.on_hover_text(tip)
}
