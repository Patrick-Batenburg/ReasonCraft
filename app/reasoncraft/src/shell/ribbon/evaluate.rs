//! The Evaluate ribbon tab: rate the selection, clear ratings and choose how
//! ratings show on the map.

use eframe::egui::Ui;
use reasoncraft_core::{Acceptance, CardStrength, ClearScope, SourceRating};

use super::{command, group, large};
use crate::a11y::{in_group, labelled};
use crate::actions::{Action, Rate, Rated, is_enabled, rated};
use crate::app::ReasonCraftApp;
use crate::canvas::input::{Os, shortcut_for, shortcut_text};
use crate::commands;
use crate::doc_tab::DocTab;
use crate::i18n::{FluentArgs, t, t_args};
use crate::shell::icons::Icon;
use crate::theme::Tokens;

pub(super) fn show(ui: &mut Ui, app: &mut ReasonCraftApp) {
    let tk = app.tokens;
    let tab = app.active_tab();
    let (mut acceptance, mut strength, mut source) = (None, None, None);
    match rated(tab) {
        Some(Rated::Acceptance(b)) => acceptance = tab.doc.boxes[&b].acceptance,
        Some(Rated::Strength(c)) => strength = tab.doc.cards[&c].strength,
        Some(Rated::Source(b)) => source = tab.doc.boxes[&b].source_rating,
        None => {}
    }
    let mut out = Vec::new();
    let keys = rating_keys(app.os);
    let tip = |key: &str| t_args(key, &keys);

    use Acceptance as A;
    use CardStrength as C;
    use SourceRating as S;
    let claim = [
        (Icon::Accept, "evaluate.accept", A::Accept),
        (Icon::Unsure, "evaluate.unsure", A::Unsure),
        (Icon::Reject, "evaluate.reject", A::Reject),
    ];
    let card = [
        (Icon::Strong, "evaluate.strong", C::Strong),
        (Icon::Weak, "evaluate.weak", C::Weak),
        (Icon::Nil, "evaluate.card_nil", C::Nil),
    ];
    let src = [
        (Icon::Strong, "evaluate.solid", S::Solid),
        (Icon::Weak, "evaluate.shaky", S::Shaky),
        (Icon::Nil, "evaluate.source_nil", S::Nil),
    ];
    let name = t("group.eval_claim");
    group(ui, &tk, Icon::Accept, &name, |ui| {
        let tip = tip("eval.tip.claim");
        out.extend(scale(
            ui,
            &tk,
            tab,
            (&name, &tip),
            acceptance,
            claim,
            Action::SetAcceptance,
        ));
    });
    let name = t("group.eval_card");
    group(ui, &tk, Icon::Strong, &name, |ui| {
        let tip = tip("eval.tip.card");
        out.extend(scale(
            ui,
            &tk,
            tab,
            (&name, &tip),
            strength,
            card,
            Action::SetStrength,
        ));
    });
    let name = t("group.eval_source");
    group(ui, &tk, Icon::Source, &name, |ui| {
        let tip = tip("eval.tip.source");
        out.extend(scale(
            ui,
            &tk,
            tab,
            (&name, &tip),
            source,
            src,
            Action::SetSourceRating,
        ));
    });
    group(ui, &tk, Icon::Clear, &t("group.eval_clear"), |ui| {
        let selection = tab
            .selection
            .map(|item| Action::ClearEvaluation(ClearScope::Item(item)));
        let enabled = selection.as_ref().is_some_and(|a| is_enabled(tab, a));
        if large(
            ui,
            &tk,
            Icon::Clear,
            &t("eval.clear_selection"),
            enabled,
            false,
        )
        .on_hover_text(tip("eval.tip.clear_selection"))
        .clicked()
        {
            out.extend(selection);
        }
        let id = "evaluate.clear_map";
        let on = is_enabled(tab, &commands::get(id).action);
        command(ui, &tk, Icon::Clear, id, on, false, &mut out)
            .on_hover_text(tip("eval.tip.clear_map"));
    });
    group(ui, &tk, Icon::Markers, &t("group.eval_view"), |ui| {
        display_toggles(ui, app, &mut out);
    });
    app.pending.extend(out);
}

/// One rating scale: a button per value (a registry command id), "on" for
/// the current value, named with the `group` it is in (Strong, Weak and
/// Nil of a card; Solid, Shaky and Nil of a source). Clicking the current
/// value clears it. Returns the clicked button's action.
fn scale<T: Copy + PartialEq>(
    ui: &mut Ui,
    tk: &Tokens,
    tab: &DocTab,
    (group, tip): (&str, &str),
    current: Option<T>,
    values: [(Icon, &str, T); 3],
    set: fn(Option<T>) -> Action,
) -> Option<Action> {
    let mut clicked = None;
    for (icon, id, value) in values {
        let on = current == Some(value);
        let cmd = commands::get(id);
        let action = cmd.action.clone();
        let enabled = is_enabled(tab, &action);
        let label = t(cmd.label);
        let button = large(ui, tk, icon, &label, enabled, on);
        if labelled(button, in_group(group, &label))
            .on_hover_text(tip)
            .clicked()
        {
            clicked = Some(if on { set(None) } else { action });
        }
    }
    clicked
}

/// The "show markers" and "evaluation shading" toggles (Evaluate and View tabs).
pub(super) fn display_toggles(ui: &mut Ui, app: &ReasonCraftApp, out: &mut Vec<Action>) {
    let tk = app.tokens;
    let toggles = [
        (Icon::Markers, "evaluate.markers", app.settings.show_markers),
        (Icon::Shading, "evaluate.shading", app.settings.eval_shading),
    ];
    for (icon, id, on) in toggles {
        command(ui, &tk, icon, id, true, on, out);
    }
}

/// The rating keys of `os`, for the tooltips: `high`, `mid`, `low`,
/// `clear` and `clear_map`.
fn rating_keys(os: Os) -> FluentArgs<'static> {
    let mut args = FluentArgs::new();
    for (name, action) in [
        ("high", Action::Rate(Rate::High)),
        ("mid", Action::Rate(Rate::Mid)),
        ("low", Action::Rate(Rate::Low)),
        ("clear", Action::Rate(Rate::Clear)),
        ("clear_map", Action::ClearEvaluation(ClearScope::Map)),
    ] {
        let keys = shortcut_for(&action, os).map(|s| shortcut_text(s, os));
        args.set(name, keys.unwrap_or_default());
    }
    args
}
