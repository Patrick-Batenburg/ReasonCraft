//! Structure rules: which items may be attached under which parent, what
//! a kind of box can carry, and how a box's children are grouped on screen.
//! The app asks these instead of re-coding them.

use crate::ids::{Item, Parent};
use crate::model::{BoxKind, CardKind, Document};

/// An item that is about to be created.
#[derive(Debug, Clone, PartialEq)]
pub enum NewItem {
    Card(CardKind),
    Category,
    Source,
    Note,
}

/// Whether `new` may be attached under `parent`.
///
/// Root claim: cards or categories, never both (the first child decides);
/// sources and notes are always allowed. Premise: cards, sources, notes.
/// Whole card: cards only. Category: categories and notes. Source: notes.
/// Note: nothing. Parking: anything.
pub fn can_attach(doc: &Document, parent: Parent, new: NewItem) -> bool {
    match parent {
        Parent::Parking => true,
        Parent::Card(id) => doc.cards.contains_key(&id) && matches!(new, NewItem::Card(_)),
        Parent::Box(id) => {
            let Some(node) = doc.boxes.get(&id) else {
                return false;
            };
            if !kind_accepts(&node.kind, &new) {
                return false;
            }
            // A claim holds cards or categories, never both.
            match (&node.kind, new) {
                (BoxKind::Claim, NewItem::Card(_)) => !has_category(doc, &node.children),
                (BoxKind::Claim, NewItem::Category) => {
                    !node.children.iter().any(|i| matches!(i, Item::Card(_)))
                }
                _ => true,
            }
        }
    }
}

/// Whether a box of kind `parent` may hold `child` at all, leaving aside
/// what it already holds. The table behind [`can_attach`] and
/// `Document::validate`.
pub fn kind_accepts(parent: &BoxKind, child: &NewItem) -> bool {
    match parent {
        BoxKind::Claim => true,
        BoxKind::Premise => matches!(child, NewItem::Card(_) | NewItem::Source | NewItem::Note),
        BoxKind::Category => matches!(child, NewItem::Category | NewItem::Note),
        BoxKind::Source { .. } => matches!(child, NewItem::Note),
        BoxKind::Note => false,
    }
}

/// The rating a kind of box takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxRating {
    /// Accept, unsure or reject: claims and premises.
    Acceptance,
    /// Solid, shaky or nil: sources.
    Source,
}

impl BoxKind {
    /// Sources and notes: shown before a box's other children.
    pub fn is_attachment(&self) -> bool {
        matches!(self, BoxKind::Source { .. } | BoxKind::Note)
    }

    /// Whether a box of this kind can hold an image (all but notes).
    pub fn takes_image(&self) -> bool {
        *self != BoxKind::Note
    }

    /// The rating a box of this kind takes, if any (categories and notes
    /// take none).
    pub fn rating(&self) -> Option<BoxRating> {
        match self {
            BoxKind::Claim | BoxKind::Premise => Some(BoxRating::Acceptance),
            BoxKind::Source { .. } => Some(BoxRating::Source),
            BoxKind::Category | BoxKind::Note => None,
        }
    }
}

/// Whether `item` is a source or a note (see [`BoxKind::is_attachment`]).
pub fn is_attachment(doc: &Document, item: Item) -> bool {
    matches!(item, Item::Box(b) if doc.boxes.get(&b).is_some_and(|n| n.kind.is_attachment()))
}

/// Whether `a` and `b`, children of `parent`, are shown in the same group:
/// under a box sources and notes come first, then the rest; cards and the
/// parking area have one group.
pub fn same_display_group(doc: &Document, parent: Parent, a: Item, b: Item) -> bool {
    !matches!(parent, Parent::Box(_)) || is_attachment(doc, a) == is_attachment(doc, b)
}

fn has_category(doc: &Document, children: &[Item]) -> bool {
    children.iter().any(|i| {
        matches!(i, Item::Box(b) if doc.boxes.get(b).is_some_and(|n| n.kind == BoxKind::Category))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::MapKind;

    #[test]
    fn category_rejects_sources_and_cards() {
        let mut d = Document::new("Topic");
        let c = d
            .add_box(Parent::Box(d.root), BoxKind::Category, "Industry")
            .unwrap();
        assert!(!can_attach(&d, Parent::Box(c), NewItem::Source));
        assert!(!can_attach(
            &d,
            Parent::Box(c),
            NewItem::Card(CardKind::Reason)
        ));
        assert!(can_attach(&d, Parent::Box(c), NewItem::Category));
        assert_eq!(d.map_kind(), MapKind::Grouping);
    }

    #[test]
    fn note_takes_nothing_source_takes_notes() {
        let mut d = Document::new("C");
        let n = d.add_box(Parent::Box(d.root), BoxKind::Note, "n").unwrap();
        let s = d
            .add_box(
                Parent::Box(d.root),
                BoxKind::Source {
                    source_type: crate::model::SourceType::Publication,
                },
                "s",
            )
            .unwrap();
        assert!(!can_attach(&d, Parent::Box(n), NewItem::Note));
        assert!(can_attach(&d, Parent::Box(s), NewItem::Note));
        assert!(!can_attach(&d, Parent::Box(s), NewItem::Source));
        assert!(can_attach(&d, Parent::Parking, NewItem::Category));
    }

    #[test]
    fn root_with_category_refuses_cards() {
        let mut d = Document::new("C");
        d.add_box(Parent::Box(d.root), BoxKind::Category, "X")
            .unwrap();
        assert!(!can_attach(
            &d,
            Parent::Box(d.root),
            NewItem::Card(CardKind::Objection)
        ));
        assert!(can_attach(&d, Parent::Box(d.root), NewItem::Source));
    }

    #[test]
    fn kind_predicates() {
        let source = BoxKind::Source {
            source_type: crate::model::SourceType::Web,
        };
        let kinds = [
            BoxKind::Claim,
            BoxKind::Premise,
            BoxKind::Category,
            source.clone(),
            BoxKind::Note,
        ];
        let attach: Vec<bool> = kinds.iter().map(BoxKind::is_attachment).collect();
        assert_eq!(attach, [false, false, false, true, true]);
        let image: Vec<bool> = kinds.iter().map(BoxKind::takes_image).collect();
        assert_eq!(image, [true, true, true, true, false]);
        let rating: Vec<_> = kinds.iter().map(BoxKind::rating).collect();
        use BoxRating::{Acceptance as A, Source as S};
        assert_eq!(rating, [Some(A), Some(A), None, Some(S), None]);
        assert!(kind_accepts(&BoxKind::Claim, &NewItem::Category));
        assert!(!kind_accepts(&BoxKind::Premise, &NewItem::Category));
        assert!(kind_accepts(&source, &NewItem::Note));
    }

    /// Under a box, sources and notes form one group and the rest another;
    /// a card's children and the parking area are one group.
    #[test]
    fn display_groups() {
        let mut d = Document::new("C");
        let root = Parent::Box(d.root);
        let (card, _) = d.add_card(root, CardKind::Reason, "p").unwrap();
        let note = Item::Box(d.add_box(root, BoxKind::Note, "n").unwrap());
        let card = Item::Card(card);
        assert!(is_attachment(&d, note));
        assert!(!is_attachment(&d, card));
        assert!(!same_display_group(&d, root, note, card));
        assert!(same_display_group(&d, root, card, card));
        assert!(same_display_group(&d, Parent::Parking, note, card));
        assert_eq!(crate::layout::display_children(&d, d.root), [note, card]);
    }
}
