//! Card roles (supports, opposes, rebuts) and the owner index they are
//! derived from.

use std::collections::{HashMap, HashSet};

use indexmap::IndexMap;

use crate::ids::{CardId, Item, NodeId, Parent};
use crate::model::{Card, CardKind, Document, Role};

/// The role of card `id`: reasons support. An objection whose owner sits
/// inside an objection card rebuts; any other objection (including loose
/// ones) opposes. A card that does not exist counts as opposing.
pub fn role_of(doc: &Document, id: CardId) -> Role {
    let Some(card) = doc.cards.get(&id) else {
        return Role::Opposes;
    };
    if card.kind == CardKind::Reason {
        return Role::Supports;
    }
    let in_objection = |c: &Card| c.kind == CardKind::Objection;
    let rebuts = match doc.owner_of(Item::Card(id)) {
        Some(Parent::Card(c)) => doc.cards.get(&c).is_some_and(in_objection),
        Some(Parent::Box(b)) => doc
            .cards
            .values()
            .any(|c| in_objection(c) && c.premises.contains(&b)),
        Some(Parent::Parking) | None => false,
    };
    if rebuts { Role::Rebuts } else { Role::Opposes }
}

/// Who holds each placed item, built in one pass. Same answers as
/// [`Document::owner_of`] (parking first, then box children, then card
/// children and premises), without a scan per query.
pub fn owner_index(doc: &Document) -> HashMap<Item, Parent> {
    let mut index = HashMap::new();
    for &item in &doc.parking {
        index.entry(item).or_insert(Parent::Parking);
    }
    for b in doc.boxes.values() {
        for &item in &b.children {
            index.entry(item).or_insert(Parent::Box(b.id));
        }
    }
    for c in doc.cards.values() {
        for &child in &c.children {
            index.entry(Item::Card(child)).or_insert(Parent::Card(c.id));
        }
    }
    for c in doc.cards.values() {
        for &pid in &c.premises {
            index.entry(Item::Box(pid)).or_insert(Parent::Card(c.id));
        }
    }
    index
}

/// The role of every card, from one owner index. The rule is that of
/// [`role_of`]: reasons support; an objection owned by an objection card,
/// or by a premise of one, rebuts; any other objection opposes.
pub fn card_roles(doc: &Document) -> IndexMap<CardId, Role> {
    let owners = owner_index(doc);
    let objection_premises: HashSet<NodeId> = doc
        .cards
        .values()
        .filter(|c| c.kind == CardKind::Objection)
        .flat_map(|c| c.premises.iter().copied())
        .collect();
    let mut roles = IndexMap::with_capacity(doc.cards.len());
    for (&id, card) in &doc.cards {
        let role = if card.kind == CardKind::Reason {
            Role::Supports
        } else {
            let rebuts = match owners.get(&Item::Card(id)) {
                Some(Parent::Card(c)) => doc
                    .cards
                    .get(c)
                    .is_some_and(|c| c.kind == CardKind::Objection),
                Some(Parent::Box(b)) => objection_premises.contains(b),
                Some(Parent::Parking) | None => false,
            };
            if rebuts { Role::Rebuts } else { Role::Opposes }
        };
        roles.insert(id, role);
    }
    roles
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::sample_iot_like;

    #[test]
    fn card_roles_equals_card_role() {
        let doc = sample_iot_like(117);
        let roles = card_roles(&doc);
        assert_eq!(roles.len(), doc.cards.len());
        for &id in doc.cards.keys() {
            assert_eq!(roles[&id], doc.card_role(id));
            assert_eq!(role_of(&doc, id), doc.card_role(id));
        }
    }
}
