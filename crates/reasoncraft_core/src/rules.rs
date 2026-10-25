//! Structure rules: which items may be attached under which parent.

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
            match node.kind {
                BoxKind::Claim => match new {
                    NewItem::Source | NewItem::Note => true,
                    NewItem::Card(_) => !has_category(doc, &node.children),
                    NewItem::Category => !node.children.iter().any(|i| matches!(i, Item::Card(_))),
                },
                BoxKind::Premise => {
                    matches!(new, NewItem::Card(_) | NewItem::Source | NewItem::Note)
                }
                BoxKind::Category => matches!(new, NewItem::Category | NewItem::Note),
                BoxKind::Source { .. } => matches!(new, NewItem::Note),
                BoxKind::Note => false,
            }
        }
    }
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
}
