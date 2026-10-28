//! Rating the selection: acceptance of claims and premises, strength of
//! cards, reliability of sources.

use reasoncraft_core::{
    Acceptance, BoxRating, CardId, CardStrength, ClearScope, Document, Item, NodeId, SourceRating,
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
        Item::Box(b) => match tab.doc.boxes.get(&b)?.kind.rating()? {
            BoxRating::Acceptance => Some(Rated::Acceptance(b)),
            BoxRating::Source => Some(Rated::Source(b)),
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

// A rating the selection cannot take, or one it already has, changes
// nothing and records no history.

/// Rate the selection, whatever its kind (Alt+1/2/3/0).
pub(super) fn rate(tab: &mut DocTab, rate: Rate) {
    use Acceptance as A;
    use CardStrength as C;
    use SourceRating as S;
    match rated(tab) {
        Some(Rated::Acceptance(_)) => {
            set_acceptance(tab, rate.pick([A::Accept, A::Unsure, A::Reject]))
        }
        Some(Rated::Strength(_)) => set_strength(tab, rate.pick([C::Strong, C::Weak, C::Nil])),
        Some(Rated::Source(_)) => set_source_rating(tab, rate.pick([S::Solid, S::Shaky, S::Nil])),
        None => {}
    }
}

/// Accept, doubt or reject the selected claim or premise (`None` clears).
pub(super) fn set_acceptance(tab: &mut DocTab, v: Option<Acceptance>) {
    if let Some(Rated::Acceptance(b)) = rated(tab)
        && tab.doc.boxes[&b].acceptance != v
    {
        tab.edit(|d| d.set_acceptance(b, v));
    }
}

/// Rate the selected card's strength (`None` clears).
pub(super) fn set_strength(tab: &mut DocTab, v: Option<CardStrength>) {
    if let Some(Rated::Strength(c)) = rated(tab)
        && tab.doc.cards[&c].strength != v
    {
        tab.edit(|d| d.set_card_strength(c, v));
    }
}

/// Rate the selected source (`None` clears).
pub(super) fn set_source_rating(tab: &mut DocTab, v: Option<SourceRating>) {
    if let Some(Rated::Source(b)) = rated(tab)
        && tab.doc.boxes[&b].source_rating != v
    {
        tab.edit(|d| d.set_source_rating(b, v));
    }
}

/// Clear one item's rating or every rating in the map.
pub(super) fn clear(tab: &mut DocTab, scope: ClearScope) {
    if has_rating(&tab.doc, scope) {
        tab.edit(|d| d.clear_evaluation(scope));
    }
}

#[cfg(test)]
mod tests;
