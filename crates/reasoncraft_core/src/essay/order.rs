//! The order of the essay: one section per first-level card under the
//! claim, strongest first.

use std::collections::HashSet;

use indexmap::IndexMap;

use crate::ids::{CardId, Item};
use crate::layout::display_children;
use crate::model::{BoxKind, CardKind, CardStrength, Document, Role};
use crate::roles::card_roles;

use super::EssaySlot;

/// What an element contributes to its section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementRole {
    /// The claim itself. [`essay_order`] does not produce it: the essay's
    /// claim lives in the introduction and conclusion.
    Claim,
    /// A premise of a card that supports what it is attached to.
    Reason,
    /// A source under a premise.
    Evidence,
    /// A premise of an objection.
    Counter,
    /// A premise of an objection to an objection.
    Rebuttal,
}

/// One map statement in a section, with the slot for its paragraph.
#[derive(Debug, Clone, PartialEq)]
pub struct EssayElement {
    pub slot: EssaySlot,
    pub role: ElementRole,
    pub text: String,
    /// 0 for the section's own premises, one more per level below.
    pub depth: usize,
}

/// A section of the essay: one first-level card and everything under it.
#[derive(Debug, Clone, PartialEq)]
pub struct EssaySection {
    /// The first-level card.
    pub item: Item,
    /// 0 Strong, 1 unrated, 2 Weak, 3 Nil.
    pub strength_rank: u8,
    pub elements: Vec<EssayElement>,
}

fn strength_rank(strength: Option<CardStrength>) -> u8 {
    match strength {
        Some(CardStrength::Strong) => 0,
        None => 1,
        Some(CardStrength::Weak) => 2,
        Some(CardStrength::Nil) => 3,
    }
}

/// The sections for the cards under the claim, strongest first (Strong,
/// unrated, Weak, Nil). Within the same strength reasons come before
/// objections; otherwise the map order is kept. Parked items are left out.
///
/// A section lists its card's premises, then their sources, then the cards
/// under them (premise cards in display order, then whole-card attacks),
/// each in the same way, one level deeper. Notes are left out.
pub fn essay_order(doc: &Document) -> Vec<EssaySection> {
    let roles = card_roles(doc);
    let mut sections: Vec<EssaySection> = display_children(doc, doc.root)
        .into_iter()
        .filter_map(|item| match item {
            Item::Card(id) => doc.cards.get(&id).map(|card| EssaySection {
                item,
                strength_rank: strength_rank(card.strength),
                elements: elements(doc, &roles, id),
            }),
            Item::Box(_) => None,
        })
        .collect();
    let is_objection = |s: &EssaySection| match s.item {
        Item::Card(c) => doc.cards[&c].kind == CardKind::Objection,
        Item::Box(_) => false,
    };
    sections.sort_by_key(|s| (s.strength_rank, is_objection(s)));
    sections
}

/// The elements of the section for `top`, in pre-order. Iterative, so
/// chains of any depth are safe.
fn elements(doc: &Document, roles: &IndexMap<CardId, Role>, top: CardId) -> Vec<EssayElement> {
    let mut out = Vec::new();
    // `seen` stops a damaged, cyclic structure from looping forever.
    let mut seen = HashSet::new();
    let mut stack = vec![(top, 0usize)];
    while let Some((id, depth)) = stack.pop() {
        let Some(card) = doc.cards.get(&id).filter(|_| seen.insert(id)) else {
            continue;
        };
        let role = match roles.get(&id) {
            Some(Role::Supports) => ElementRole::Reason,
            Some(Role::Rebuts) => ElementRole::Rebuttal,
            Some(Role::Opposes) | None => ElementRole::Counter,
        };
        let premises: Vec<_> = card
            .premises
            .iter()
            .filter_map(|p| doc.boxes.get(p))
            .collect();
        for p in &premises {
            out.push(EssayElement {
                slot: EssaySlot::Item(Item::Box(p.id)),
                role,
                text: p.text.clone(),
                depth,
            });
        }
        let mut below = Vec::new();
        for p in &premises {
            for child in display_children(doc, p.id) {
                match child {
                    Item::Box(b) => {
                        if let Some(n) = doc.boxes.get(&b)
                            && matches!(n.kind, BoxKind::Source { .. })
                        {
                            out.push(EssayElement {
                                slot: EssaySlot::Item(child),
                                role: ElementRole::Evidence,
                                text: n.text.clone(),
                                depth: depth + 1,
                            });
                        }
                    }
                    Item::Card(c) => below.push(c),
                }
            }
        }
        below.extend(card.children.iter().copied());
        // Popped in order: the first card below comes next.
        stack.extend(below.into_iter().rev().map(|c| (c, depth + 1)));
    }
    out
}
