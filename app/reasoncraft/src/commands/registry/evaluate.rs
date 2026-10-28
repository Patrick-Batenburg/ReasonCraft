//! The Evaluate tab's commands: ratings and how they show.

use reasoncraft_core::{Acceptance, CardStrength, ClearScope, SourceRating};

use super::cmd;
use crate::actions::{Action, Rate};
use crate::commands::Command;
use crate::shell::icons::Icon;

pub(super) fn commands() -> Vec<Command> {
    use Action as A;
    let (claim, card, source) = ("group.eval_claim", "group.eval_card", "group.eval_source");
    let accept = |a| A::SetAcceptance(Some(a));
    let strength = |s| A::SetStrength(Some(s));
    let rating = |r| A::SetSourceRating(Some(r));
    let rate = "group.evaluation";
    let display = "group.eval_view";
    vec![
        cmd(
            "evaluate.accept",
            "eval.accept",
            claim,
            accept(Acceptance::Accept),
        )
        .with_icon(Icon::Accept),
        cmd(
            "evaluate.unsure",
            "eval.unsure",
            claim,
            accept(Acceptance::Unsure),
        )
        .with_icon(Icon::Unsure),
        cmd(
            "evaluate.reject",
            "eval.reject",
            claim,
            accept(Acceptance::Reject),
        )
        .with_icon(Icon::Reject),
        cmd(
            "evaluate.strong",
            "eval.strong",
            card,
            strength(CardStrength::Strong),
        )
        .with_icon(Icon::Strong),
        cmd(
            "evaluate.weak",
            "eval.weak",
            card,
            strength(CardStrength::Weak),
        )
        .with_icon(Icon::Weak),
        cmd(
            "evaluate.card_nil",
            "eval.nil",
            card,
            strength(CardStrength::Nil),
        )
        .with_icon(Icon::Nil),
        cmd(
            "evaluate.solid",
            "eval.solid",
            source,
            rating(SourceRating::Solid),
        )
        .with_icon(Icon::Strong),
        cmd(
            "evaluate.shaky",
            "eval.shaky",
            source,
            rating(SourceRating::Shaky),
        )
        .with_icon(Icon::Weak),
        cmd(
            "evaluate.source_nil",
            "eval.nil",
            source,
            rating(SourceRating::Nil),
        )
        .with_icon(Icon::Nil),
        cmd(
            "evaluate.rate_high",
            "command.rate_high",
            rate,
            A::Rate(Rate::High),
        ),
        cmd(
            "evaluate.rate_mid",
            "command.rate_mid",
            rate,
            A::Rate(Rate::Mid),
        ),
        cmd(
            "evaluate.rate_low",
            "command.rate_low",
            rate,
            A::Rate(Rate::Low),
        ),
        cmd(
            "evaluate.rate_clear",
            "command.rate_clear",
            rate,
            A::Rate(Rate::Clear),
        ),
        cmd(
            "evaluate.clear_map",
            "eval.clear_map",
            "group.eval_clear",
            A::ClearEvaluation(ClearScope::Map),
        )
        .with_icon(Icon::Clear),
        cmd(
            "evaluate.markers",
            "eval.show_markers",
            display,
            A::ToggleMarkers,
        )
        .with_icon(Icon::Markers),
        cmd(
            "evaluate.shading",
            "eval.shading",
            display,
            A::ToggleShading,
        )
        .with_icon(Icon::Shading),
    ]
}
