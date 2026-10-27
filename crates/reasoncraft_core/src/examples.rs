//! The example maps and templates bundled with ReasonCraft, in English
//! and Dutch. They are ordinary documents, built through the public edit
//! operations, so they always satisfy the model's rules.

mod grouping;
mod reasoning;
mod templates;

use crate::edit::EditError;
use crate::ids::{CardId, NodeId, Parent};
use crate::meta::primary_language;
use crate::model::{
    Acceptance, BoxKind, CardKind, CardStrength, Document, SourceRating, SourceType,
};

/// A finished map that shows how a kind of map is built.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Example {
    /// "Cities should make public transport free": reasons, objections,
    /// co-premises, a rebuttal, a whole-card objection, sources, ratings
    /// and a note.
    Reasoning,
    /// "Ways to reduce food waste", three levels of categories deep.
    Grouping,
}

/// A map to start from, with placeholder text to replace.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Template {
    /// A claim with one reason and one objection.
    BlankReasoning,
    /// A topic with three categories.
    BlankGrouping,
    /// A claim, three reasons, an objection with a rebuttal, and prompts
    /// for the essay's introduction and conclusion.
    EssayPlan,
    /// A claim with a note on reviewing with the comment presets.
    PeerReview,
}

impl Template {
    /// Every template, in the order the app lists them.
    pub const ALL: [Template; 4] = [
        Template::BlankReasoning,
        Template::BlankGrouping,
        Template::EssayPlan,
        Template::PeerReview,
    ];
}

/// Example `which` in language `lang` (Dutch for any `nl` tag, else
/// English), with a title. It opens unsaved and untitled, like any new map.
pub fn example(which: Example, lang: &str) -> Document {
    let l = Lang::of(lang);
    let built = match which {
        Example::Reasoning => reasoning::build(l),
        Example::Grouping => grouping::build(l),
    };
    built.expect("a bundled example is a valid map")
}

/// Template `which` in language `lang` (Dutch for any `nl` tag, else
/// English). Its title is left for the user.
pub fn template(which: Template, lang: &str) -> Document {
    templates::build(which, Lang::of(lang)).expect("a bundled template is a valid map")
}

/// The document language a map is written in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Lang {
    dutch: bool,
}

impl Lang {
    fn of(tag: &str) -> Self {
        Self {
            dutch: primary_language(tag).eq_ignore_ascii_case("nl"),
        }
    }

    fn code(self) -> &'static str {
        if self.dutch { "nl" } else { "en" }
    }

    /// The text in this language: `en` or `nl`.
    fn s(self, en: &'static str, nl: &'static str) -> &'static str {
        if self.dutch { nl } else { en }
    }
}

/// A new map in `l` whose root says `root`, titled `title`.
fn start(l: Lang, root: &str, title: &str) -> Result<Document, EditError> {
    let mut d = Document::new(root);
    d.set_language(l.code())?;
    d.set_meta(title, "")?;
    Ok(d)
}

/// Add a card of `kind` under `parent` whose premises say `premises`
/// (at least one); returns the card and its premises.
fn card(
    d: &mut Document,
    parent: Parent,
    kind: CardKind,
    premises: &[&str],
) -> Result<(CardId, Vec<NodeId>), EditError> {
    let (first, rest) = premises.split_first().ok_or(EditError::NotAllowed)?;
    let (card, p) = d.add_card(parent, kind, first)?;
    let mut ids = vec![p];
    for text in rest {
        ids.push(d.add_premise(card, None, text)?);
    }
    Ok((card, ids))
}

/// Rate `card` and accept, doubt or reject each of its premises in turn.
fn rate(
    d: &mut Document,
    card: CardId,
    strength: CardStrength,
    premises: &[NodeId],
    verdicts: &[Acceptance],
) -> Result<(), EditError> {
    d.set_card_strength(card, Some(strength))?;
    for (&p, &v) in premises.iter().zip(verdicts) {
        d.set_acceptance(p, Some(v))?;
    }
    Ok(())
}

/// Add a source of type `t` under `premise`, rated `rating`.
fn source(
    d: &mut Document,
    premise: NodeId,
    t: SourceType,
    text: &str,
    rating: SourceRating,
) -> Result<NodeId, EditError> {
    let id = d.add_box(
        Parent::Box(premise),
        BoxKind::Source { source_type: t },
        text,
    )?;
    d.set_source_rating(id, Some(rating))?;
    Ok(id)
}

#[cfg(test)]
mod tests;
