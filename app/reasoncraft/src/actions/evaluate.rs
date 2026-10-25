//! Rating the selection: acceptance of claims and premises, strength of
//! cards, reliability of sources.

use reasoncraft_core::{
    Acceptance, BoxKind, CardId, CardStrength, ClearScope, Document, Item, NodeId, SourceRating,
};

use super::Action;
use crate::doc_tab::DocTab;

/// A rating step that means the right thing for whatever is selected
/// (Alt+1/2/3/0): claim or premise → accept / unsure / reject; card →
/// strong / weak / nil; source → solid / shaky / nil. `Clear` removes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rate {
    High,
    Mid,
    Low,
    Clear,
}

/// What the selection can be rated with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rated {
    /// A claim or premise.
    Acceptance(NodeId),
    Strength(CardId),
    Source(NodeId),
}

/// What the selection can be rated with, if anything (categories and notes
/// take no rating).
pub fn rated(tab: &DocTab) -> Option<Rated> {
    match tab.selection? {
        Item::Card(c) => tab.doc.cards.contains_key(&c).then_some(Rated::Strength(c)),
        Item::Box(b) => match tab.doc.boxes.get(&b)?.kind {
            BoxKind::Claim | BoxKind::Premise => Some(Rated::Acceptance(b)),
            BoxKind::Source { .. } => Some(Rated::Source(b)),
            BoxKind::Category | BoxKind::Note => None,
        },
    }
}

/// The explicit rating action `rate` stands for with the current selection.
pub fn rate_action(tab: &DocTab, rate: Rate) -> Option<Action> {
    use Acceptance as A;
    use CardStrength as C;
    use SourceRating as S;
    Some(match rated(tab)? {
        Rated::Acceptance(_) => Action::SetAcceptance(rate.pick([A::Accept, A::Unsure, A::Reject])),
        Rated::Strength(_) => Action::SetStrength(rate.pick([C::Strong, C::Weak, C::Nil])),
        Rated::Source(_) => Action::SetSourceRating(rate.pick([S::Solid, S::Shaky, S::Nil])),
    })
}

impl Rate {
    /// The high, mid or low value of a rating scale; `None` for `Clear`.
    fn pick<T>(self, [high, mid, low]: [T; 3]) -> Option<T> {
        match self {
            Rate::High => Some(high),
            Rate::Mid => Some(mid),
            Rate::Low => Some(low),
            Rate::Clear => None,
        }
    }
}

/// Whether `item` carries any rating.
fn item_rated(doc: &Document, item: Item) -> bool {
    match item {
        Item::Box(b) => doc
            .boxes
            .get(&b)
            .is_some_and(|n| n.acceptance.is_some() || n.source_rating.is_some()),
        Item::Card(c) => doc.cards.get(&c).is_some_and(|c| c.strength.is_some()),
    }
}

/// Whether clearing `scope` would change anything.
fn has_rating(doc: &Document, scope: ClearScope) -> bool {
    match scope {
        ClearScope::Item(item) => item_rated(doc, item),
        ClearScope::Map => {
            doc.boxes
                .values()
                .any(|n| n.acceptance.is_some() || n.source_rating.is_some())
                || doc.cards.values().any(|c| c.strength.is_some())
        }
    }
}

/// Whether an evaluation action applies to the current selection; `None`
/// for actions that are not evaluation actions.
pub(super) fn is_enabled(tab: &DocTab, action: &Action) -> Option<bool> {
    let sel = rated(tab);
    Some(match action {
        Action::Rate(rate) => rate_action(tab, *rate).is_some(),
        Action::SetAcceptance(_) => matches!(sel, Some(Rated::Acceptance(_))),
        Action::SetStrength(_) => matches!(sel, Some(Rated::Strength(_))),
        Action::SetSourceRating(_) => matches!(sel, Some(Rated::Source(_))),
        Action::ClearEvaluation(scope) => has_rating(&tab.doc, *scope),
        _ => return None,
    })
}

/// Carry out an evaluation action on the active tab. A rating the selection
/// cannot take, or one it already has, changes nothing and records no history.
pub(super) fn apply(tab: &mut DocTab, action: Action) {
    match (action, rated(tab)) {
        (Action::Rate(rate), _) => {
            if let Some(explicit) = rate_action(tab, rate) {
                apply(tab, explicit);
            }
        }
        (Action::SetAcceptance(v), Some(Rated::Acceptance(b)))
            if tab.doc.boxes[&b].acceptance != v =>
        {
            tab.edit(|d| d.set_acceptance(b, v));
        }
        (Action::SetStrength(v), Some(Rated::Strength(c))) if tab.doc.cards[&c].strength != v => {
            tab.edit(|d| d.set_card_strength(c, v));
        }
        (Action::SetSourceRating(v), Some(Rated::Source(b)))
            if tab.doc.boxes[&b].source_rating != v =>
        {
            tab.edit(|d| d.set_source_rating(b, v));
        }
        (Action::ClearEvaluation(scope), _) if has_rating(&tab.doc, scope) => {
            tab.edit(|d| d.clear_evaluation(scope));
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests;
