//! Moving the selection with the arrow keys, and jumping between the main
//! tree and the parking area.

use crate::layout::display_children;
use crate::{Document, Item, LayoutDirection, Parent};

/// An arrow key's direction: on screen, or (after [`nav_dir`]) in
/// top-down terms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

/// The direction [`navigate`] takes for an arrow key pointing `screen` in a
/// map laid out in `layout`, so arrows keep their on-screen meaning:
/// top-down it is `screen` itself; bottom-up the parent is reached with
/// Down; left-to-right Left goes to the parent, Right to the first child
/// and Up/Down move between siblings.
pub fn nav_dir(screen: Dir, layout: LayoutDirection) -> Dir {
    match (layout, screen) {
        (LayoutDirection::TopDown, d) => d,
        (LayoutDirection::BottomUp, Dir::Up) => Dir::Down,
        (LayoutDirection::BottomUp, Dir::Down) => Dir::Up,
        (LayoutDirection::BottomUp, d) => d,
        (LayoutDirection::LeftRight, Dir::Left) => Dir::Up,
        (LayoutDirection::LeftRight, Dir::Right) => Dir::Down,
        (LayoutDirection::LeftRight, Dir::Up) => Dir::Left,
        (LayoutDirection::LeftRight, Dir::Down) => Dir::Right,
    }
}

/// The item one step from `from` in direction `dir` (top-down terms; see
/// [`nav_dir`]), or `None` at an edge.
///
/// - Up: the owner (a premise's card; a card's box, or the card it targets
///   as a whole; nothing for a parked item).
/// - Down: a card's first premise; a box's first child (a card, not its
///   premise) as the map shows them (notes and sources first, see
///   [`display_children`]).
/// - Left/Right: the previous/next co-premise, then the previous/next
///   sibling in the order the map shows its owner's children.
pub fn navigate(doc: &Document, from: Item, dir: Dir) -> Option<Item> {
    match dir {
        Dir::Up => match doc.owner_of(from)? {
            Parent::Box(b) => Some(Item::Box(b)),
            Parent::Card(c) => Some(Item::Card(c)),
            Parent::Parking => None,
        },
        Dir::Down => match from {
            Item::Card(c) => enter(doc, Item::Card(c), true),
            Item::Box(b) => display_children(doc, b).first().copied(),
        },
        Dir::Left => sideways(doc, from, -1),
        Dir::Right => sideways(doc, from, 1),
    }
}

/// Where Ctrl+Shift+P goes from `from`: the main root from anything in the
/// parking area, otherwise the first parked item (`None` if there is none).
pub fn parking_jump(doc: &Document, from: Option<Item>) -> Option<Item> {
    if from.is_some_and(|item| is_parked(doc, item)) {
        Some(Item::Box(doc.root))
    } else {
        doc.parking.first().copied()
    }
}

/// Whether `item` hangs, at any depth, under a parking entry.
fn is_parked(doc: &Document, mut item: Item) -> bool {
    // Bounded by the item count, so even a damaged cycle ends.
    for _ in 0..=doc.boxes.len() + doc.cards.len() {
        item = match doc.owner_of(item) {
            Some(Parent::Parking) => return true,
            Some(Parent::Box(b)) => Item::Box(b),
            Some(Parent::Card(c)) => Item::Card(c),
            None => return false,
        };
    }
    false
}

/// A card stands for its first (or last) premise when reached from a box.
fn enter(doc: &Document, item: Item, first: bool) -> Option<Item> {
    match item {
        Item::Card(c) => {
            let premises = &doc.cards.get(&c)?.premises;
            let p = if first {
                premises.first()
            } else {
                premises.last()
            };
            p.map(|&p| Item::Box(p))
        }
        b => Some(b),
    }
}

fn sideways(doc: &Document, from: Item, step: isize) -> Option<Item> {
    if let (Item::Box(b), Some(Parent::Card(c))) = (from, doc.owner_of(from)) {
        let premises = &doc.cards.get(&c)?.premises;
        let i = premises.iter().position(|&p| p == b)?;
        if let Some(&p) = neighbour(premises, i, step) {
            return Some(Item::Box(p));
        }
        let next = sibling(doc, Item::Card(c), step)?;
        return enter(doc, next, step > 0);
    }
    let next = sibling(doc, from, step)?;
    match from {
        Item::Card(_) => Some(next),
        Item::Box(_) => enter(doc, next, step > 0),
    }
}

/// The item `step` places away from `item` in its owner's ordered list.
fn sibling(doc: &Document, item: Item, step: isize) -> Option<Item> {
    let list: Vec<Item> = match doc.owner_of(item)? {
        Parent::Box(b) => display_children(doc, b),
        Parent::Card(c) => {
            let card = doc.cards.get(&c)?;
            card.children.iter().map(|&k| Item::Card(k)).collect()
        }
        Parent::Parking => doc.parking.clone(),
    };
    let i = list.iter().position(|&x| x == item)?;
    neighbour(&list, i, step).copied()
}

fn neighbour<T>(list: &[T], i: usize, step: isize) -> Option<&T> {
    list.get(i.checked_add_signed(step)?)
}

#[cfg(test)]
mod tests;
