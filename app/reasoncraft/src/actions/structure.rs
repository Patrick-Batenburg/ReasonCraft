//! Adding cards, premises, categories and notes, deleting items, and
//! moving them (drag and drop, "Connect to…", Alt+arrow reordering).

use reasoncraft_core::{BoxKind, CardId, CardKind, Document, Item, NewLoose, Parent, RemoveMode};

use super::{Action, Dir};
use crate::canvas::input::nav_dir;
use crate::doc_tab::DocTab;

/// Moves and reorders.
pub(super) fn apply(tab: &mut DocTab, action: Action) {
    match action {
        Action::Move { item, to, index } => {
            if tab.edit(|d| d.move_item(item, to, index)).is_some() {
                tab.selection = Some(item);
                tab.fix_selection();
            }
        }
        Action::Reorder(screen) => {
            // At an edge nothing moves and no history is recorded.
            let dir = nav_dir(screen, tab.doc.layout_direction);
            if let Some((item, to, index)) =
                tab.selection.and_then(|s| reorder_target(&tab.doc, s, dir))
            {
                tab.edit(|d| d.move_item(item, to, Some(index)));
            }
        }
        _ => unreachable!("not a move"),
    }
}

/// `Some` for the actions this module handles.
pub(super) fn is_enabled(tab: &DocTab, action: &Action) -> Option<bool> {
    Some(match action {
        Action::Move { item, to, index } => {
            let doc = &tab.doc;
            !moves_nowhere(doc, *item, *to, *index)
                && doc.clone().move_item(*item, *to, *index).is_ok()
        }
        Action::Reorder(screen) => {
            let dir = nav_dir(*screen, tab.doc.layout_direction);
            tab.selection
                .is_some_and(|s| reorder_target(&tab.doc, s, dir).is_some())
        }
        _ => return None,
    })
}

/// The ordered list an item joins under `to`: a box's children, a card's
/// premises (for a premise) or whole-card children, or the parking list.
pub fn sibling_list(doc: &Document, to: Parent, premise: bool) -> Vec<Item> {
    match to {
        Parent::Box(b) => doc
            .boxes
            .get(&b)
            .map(|n| n.children.clone())
            .unwrap_or_default(),
        Parent::Card(c) => doc.cards.get(&c).map_or_else(Vec::new, |card| {
            if premise {
                card.premises.iter().map(|&p| Item::Box(p)).collect()
            } else {
                card.children.iter().map(|&k| Item::Card(k)).collect()
            }
        }),
        Parent::Parking => doc.parking.clone(),
    }
}

/// Sources and notes: the layout shows them before a box's other children.
pub fn is_attachment(doc: &Document, item: Item) -> bool {
    matches!(item, Item::Box(b) if doc.boxes.get(&b)
        .is_some_and(|n| matches!(n.kind, BoxKind::Source { .. } | BoxKind::Note)))
}

fn is_premise(doc: &Document, item: Item) -> bool {
    matches!(item, Item::Box(b) if doc.boxes.get(&b).is_some_and(|n| n.kind == BoxKind::Premise))
}

/// Whether moving `item` to `to` at `index` (see [`Action::Move`]) would
/// leave the map looking the same: `to` already holds it, and its place
/// among the siblings shown in its group (under a box, sources and notes
/// come first) stays the same. A move that changes only the hidden
/// document order counts as no change. With no index nothing moves.
pub fn moves_nowhere(doc: &Document, item: Item, to: Parent, index: Option<usize>) -> bool {
    if doc.owner_of(item) != Some(to) {
        return false;
    }
    let Some(index) = index else { return true };
    let before = sibling_list(doc, to, is_premise(doc, item));
    let mut after: Vec<Item> = before.iter().copied().filter(|&x| x != item).collect();
    if index > after.len() {
        return false;
    }
    after.insert(index, item);
    let shown = |list: Vec<Item>| -> Vec<Item> {
        list.into_iter()
            .filter(|&x| {
                !matches!(to, Parent::Box(_)) || is_attachment(doc, x) == is_attachment(doc, item)
            })
            .collect()
    };
    shown(before) == shown(after)
}

/// The [`Action::Reorder`] move for `selected`: the item that moves (a
/// premise within its card; a card's only premise moves its card), its
/// owner and its new index. It swaps places with the nearest sibling in
/// direction `dir` that the layout shows in the same group (under a box,
/// sources and notes come first), so every step is visible. `None` at an
/// edge, for the root, and for Up/Down.
fn reorder_target(doc: &Document, selected: Item, dir: Dir) -> Option<(Item, Parent, usize)> {
    let step: isize = match dir {
        Dir::Left => -1,
        Dir::Right => 1,
        Dir::Up | Dir::Down => return None,
    };
    let mut item = selected;
    if let (Item::Box(_), Some(Parent::Card(c))) = (item, doc.owner_of(item))
        && doc.cards.get(&c)?.premises.len() == 1
    {
        item = Item::Card(c);
    }
    let to = doc.owner_of(item)?;
    let list = sibling_list(doc, to, is_premise(doc, item));
    let at = list.iter().position(|&x| x == item)?;
    let group = |x: Item| {
        !matches!(to, Parent::Box(_)) || is_attachment(doc, x) == is_attachment(doc, item)
    };
    let mut j = at;
    loop {
        j = j.checked_add_signed(step)?;
        if group(*list.get(j)?) {
            // With `item` taken out, a later neighbour shifts down to j - 1,
            // so index j lands just past the neighbour either way.
            return Some((item, to, j));
        }
    }
}

/// Selected box → card under it; selected card → card targeting the whole
/// card; nothing selected → loose card in the parking area.
pub(super) fn add_card(tab: &mut DocTab, kind: CardKind) {
    let premise = match tab.selection {
        Some(Item::Box(b)) => tab
            .edit(|d| d.add_card(Parent::Box(b), kind, ""))
            .map(|(_, p)| p),
        Some(Item::Card(c)) => tab
            .edit(|d| d.add_card(Parent::Card(c), kind, ""))
            .map(|(_, p)| p),
        None => tab.edit(|d| {
            let item = d.add_loose(NewLoose::Card(kind), "");
            Ok(match item {
                Item::Card(c) => d.cards[&c].premises[0],
                Item::Box(b) => b,
            })
        }),
    };
    select_new(tab, premise.map(Item::Box));
}

/// A premise for the selected card, or for the card that owns the selected premise.
pub(super) fn add_co_premise(tab: &mut DocTab) {
    if let Some(c) = selected_card(tab) {
        let p = tab.edit(|d| d.add_premise(c, None, ""));
        select_new(tab, p.map(Item::Box));
    }
}

/// A category or note under the selected box, or loose when nothing is selected.
pub(super) fn add_box(tab: &mut DocTab, kind: BoxKind, loose: NewLoose) {
    let item = match tab.selection {
        Some(Item::Box(b)) => tab
            .edit(|d| d.add_box(Parent::Box(b), kind, ""))
            .map(Item::Box),
        Some(Item::Card(_)) => None,
        None => tab.edit(|d| Ok(d.add_loose(loose, ""))),
    };
    select_new(tab, item);
}

/// Remove the selection, then select what held it (or the root).
pub(super) fn delete(tab: &mut DocTab, mode: RemoveMode) {
    let Some(item) = tab.selection else { return };
    let owner = tab.doc.owner_of(item);
    if tab.edit(|d| d.remove_item(item, mode)).is_none() {
        return;
    }
    tab.selection = match owner {
        Some(Parent::Box(b)) => Some(Item::Box(b)),
        Some(Parent::Card(c)) => Some(Item::Card(c)),
        Some(Parent::Parking) | None => None,
    };
    tab.fix_selection();
}

/// The card a whole-card target or co-premise goes to: the selected card,
/// or the card that owns the selected premise.
pub fn selected_card(tab: &DocTab) -> Option<CardId> {
    match tab.selection? {
        Item::Card(c) => Some(c),
        Item::Box(b) => match tab.doc.owner_of(Item::Box(b)) {
            Some(Parent::Card(c)) => Some(c),
            _ => None,
        },
    }
}

#[cfg(test)]
mod tests;

/// Select a newly added item, scroll it into view and start editing its
/// text (a card's: its first premise). The add itself is the history step
/// for that text.
pub(super) fn select_new(tab: &mut DocTab, item: Option<Item>) {
    let b = match item {
        Some(Item::Box(b)) => b,
        Some(Item::Card(c)) => tab.doc.cards[&c].premises[0],
        None => return,
    };
    tab.begin_edit(b, true);
    tab.reveal_pending = true;
}
