//! Edit operations for ratings, kinds, links, comments and images.

use uuid::Uuid;

use crate::edit::EditError;
use crate::ids::{CardId, Item, NodeId};
use crate::model::{
    Acceptance, BoxKind, BoxNode, CardKind, CardStrength, Comment, Document, ImageRef,
    SourceRating, SourceType, Verdict, now_rfc3339,
};

/// What [`Document::clear_evaluation`] resets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClearScope {
    /// One box's rating or one card's strength.
    Item(Item),
    /// Every rating in the document, the parking area included.
    Map,
}

impl Document {
    fn node_mut(&mut self, id: NodeId) -> Result<&mut BoxNode, EditError> {
        self.boxes.get_mut(&id).ok_or(EditError::NotFound)
    }

    fn finish_edit(&mut self) {
        self.touch();
        #[cfg(debug_assertions)]
        self.check_invariants();
    }

    /// Accept, doubt or reject a claim or premise.
    pub fn set_acceptance(&mut self, id: NodeId, v: Option<Acceptance>) -> Result<(), EditError> {
        let node = self.node_mut(id)?;
        if !matches!(node.kind, BoxKind::Claim | BoxKind::Premise) {
            return Err(EditError::NotAllowed);
        }
        node.acceptance = v;
        self.finish_edit();
        Ok(())
    }

    /// Rate a source box.
    pub fn set_source_rating(
        &mut self,
        id: NodeId,
        v: Option<SourceRating>,
    ) -> Result<(), EditError> {
        let node = self.node_mut(id)?;
        if !matches!(node.kind, BoxKind::Source { .. }) {
            return Err(EditError::NotAllowed);
        }
        node.source_rating = v;
        self.finish_edit();
        Ok(())
    }

    /// Rate how strongly a card's premises carry its conclusion.
    pub fn set_card_strength(
        &mut self,
        id: CardId,
        v: Option<CardStrength>,
    ) -> Result<(), EditError> {
        self.cards.get_mut(&id).ok_or(EditError::NotFound)?.strength = v;
        self.finish_edit();
        Ok(())
    }

    /// Turn a reason into an objection or back. The strength is kept.
    pub fn set_card_kind(&mut self, id: CardId, kind: CardKind) -> Result<(), EditError> {
        self.cards.get_mut(&id).ok_or(EditError::NotFound)?.kind = kind;
        self.finish_edit();
        Ok(())
    }

    /// Change the type of a source box.
    pub fn set_source_type(&mut self, id: NodeId, t: SourceType) -> Result<(), EditError> {
        let node = self.node_mut(id)?;
        let BoxKind::Source { source_type } = &mut node.kind else {
            return Err(EditError::NotAllowed);
        };
        *source_type = t;
        self.finish_edit();
        Ok(())
    }

    /// Set or clear a box's link. The value is trimmed; empty means none.
    pub fn set_link(&mut self, id: NodeId, link: Option<String>) -> Result<(), EditError> {
        let node = self.node_mut(id)?;
        node.link = link.map(|l| l.trim().to_owned()).filter(|l| !l.is_empty());
        self.finish_edit();
        Ok(())
    }

    /// Add a comment to a box and return its id.
    pub fn add_comment(
        &mut self,
        id: NodeId,
        author: &str,
        text: &str,
        verdict: Option<Verdict>,
    ) -> Result<Uuid, EditError> {
        let node = self.node_mut(id)?;
        let comment = Comment {
            id: Uuid::new_v4(),
            author: author.to_owned(),
            text: text.to_owned(),
            verdict,
            created: now_rfc3339(),
        };
        let cid = comment.id;
        node.comments.push(comment);
        self.finish_edit();
        Ok(cid)
    }

    /// Remove one comment from a box.
    pub fn remove_comment(&mut self, id: NodeId, comment: Uuid) -> Result<(), EditError> {
        let node = self.node_mut(id)?;
        let at = node
            .comments
            .iter()
            .position(|c| c.id == comment)
            .ok_or(EditError::NotFound)?;
        node.comments.remove(at);
        self.finish_edit();
        Ok(())
    }

    /// Set or clear a box's image. Notes cannot hold one.
    pub fn set_image(&mut self, id: NodeId, image: Option<ImageRef>) -> Result<(), EditError> {
        let node = self.node_mut(id)?;
        if node.kind == BoxKind::Note {
            return Err(EditError::NotAllowed);
        }
        node.image = image;
        self.finish_edit();
        Ok(())
    }

    /// Clear the rating of one item, or every rating in the document.
    pub fn clear_evaluation(&mut self, scope: ClearScope) -> Result<(), EditError> {
        match scope {
            ClearScope::Item(Item::Box(b)) => {
                let node = self.node_mut(b)?;
                node.acceptance = None;
                node.source_rating = None;
            }
            ClearScope::Item(Item::Card(c)) => {
                self.cards.get_mut(&c).ok_or(EditError::NotFound)?.strength = None;
            }
            ClearScope::Map => {
                for node in self.boxes.values_mut() {
                    node.acceptance = None;
                    node.source_rating = None;
                }
                for card in self.cards.values_mut() {
                    card.strength = None;
                }
            }
        }
        self.finish_edit();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edit::NewLoose;
    use crate::ids::Parent;
    use crate::model::{
        Acceptance, BoxKind, BoxNode, CardKind, CardStrength, Document, ImageRef, SourceRating,
        SourceType, Verdict,
    };

    fn source(d: &mut Document) -> NodeId {
        d.add_box(
            Parent::Box(d.root),
            BoxKind::Source {
                source_type: SourceType::Source,
            },
            "s",
        )
        .unwrap()
    }

    #[test]
    fn ratings_follow_kind_rules() {
        let mut d = Document::new("C");
        let s = source(&mut d);
        let (card, p) = d
            .add_card(Parent::Box(d.root), CardKind::Reason, "P")
            .unwrap();
        assert_eq!(
            d.set_acceptance(s, Some(Acceptance::Accept)),
            Err(EditError::NotAllowed)
        );
        assert_eq!(
            d.set_source_rating(p, Some(SourceRating::Solid)),
            Err(EditError::NotAllowed)
        );
        assert_eq!(
            d.set_source_type(p, SourceType::Law),
            Err(EditError::NotAllowed)
        );
        d.set_acceptance(p, Some(Acceptance::Unsure)).unwrap();
        d.set_acceptance(d.root, Some(Acceptance::Reject)).unwrap();
        d.set_source_rating(s, Some(SourceRating::Shaky)).unwrap();
        d.set_source_type(s, SourceType::Law).unwrap();
        d.set_card_strength(card, Some(CardStrength::Strong))
            .unwrap();
        d.set_card_kind(card, CardKind::Objection).unwrap();
        assert_eq!(d.cards[&card].kind, CardKind::Objection);
        assert_eq!(d.cards[&card].strength, Some(CardStrength::Strong));
        assert_eq!(
            d.set_card_strength(CardId::new(), None),
            Err(EditError::NotFound)
        );
        assert_eq!(d.validate(), Ok(()));

        let mut bad = d.clone();
        bad.boxes[&s].acceptance = Some(Acceptance::Accept);
        let err = bad.validate().unwrap_err();
        assert!(err.contains(&format!("{s:?}")), "{err}");
        let mut bad = d.clone();
        bad.boxes[&p].source_rating = Some(SourceRating::Nil);
        assert!(bad.validate().is_err());
    }

    #[test]
    fn clear_map_clears_parking_too() {
        let mut d = Document::new("C");
        let (card, p) = d
            .add_card(Parent::Box(d.root), CardKind::Reason, "P")
            .unwrap();
        let Item::Card(parked) = d.add_loose(NewLoose::Card(CardKind::Objection), "x") else {
            unreachable!()
        };
        let parked_premise = d.cards[&parked].premises[0];
        let s = d.add_loose(NewLoose::Source(SourceType::Source), "s");
        let Item::Box(s) = s else { unreachable!() };
        d.set_acceptance(p, Some(Acceptance::Accept)).unwrap();
        d.set_acceptance(parked_premise, Some(Acceptance::Reject))
            .unwrap();
        d.set_card_strength(card, Some(CardStrength::Weak)).unwrap();
        d.set_card_strength(parked, Some(CardStrength::Nil))
            .unwrap();
        d.set_source_rating(s, Some(SourceRating::Solid)).unwrap();
        d.clear_evaluation(ClearScope::Map).unwrap();
        assert!(d.boxes.values().all(|b| b.acceptance.is_none()));
        assert!(d.boxes.values().all(|b| b.source_rating.is_none()));
        assert!(d.cards.values().all(|c| c.strength.is_none()));
    }

    #[test]
    fn clear_item_scope_only_touches_that_item() {
        let mut d = Document::new("C");
        let (card, p) = d
            .add_card(Parent::Box(d.root), CardKind::Reason, "P")
            .unwrap();
        d.set_acceptance(p, Some(Acceptance::Accept)).unwrap();
        d.set_acceptance(d.root, Some(Acceptance::Accept)).unwrap();
        d.set_card_strength(card, Some(CardStrength::Weak)).unwrap();
        d.clear_evaluation(ClearScope::Item(Item::Box(p))).unwrap();
        assert_eq!(d.boxes[&p].acceptance, None);
        assert_eq!(d.boxes[&d.root].acceptance, Some(Acceptance::Accept));
        assert_eq!(d.cards[&card].strength, Some(CardStrength::Weak));
        d.clear_evaluation(ClearScope::Item(Item::Card(card)))
            .unwrap();
        assert_eq!(d.cards[&card].strength, None);
        assert_eq!(
            d.clear_evaluation(ClearScope::Item(Item::Box(NodeId::new()))),
            Err(EditError::NotFound)
        );
    }

    #[test]
    fn comments_add_remove() {
        let mut d = Document::new("C");
        let root = d.root;
        let a = d
            .add_comment(root, "Pat", "first", Some(Verdict::Problem))
            .unwrap();
        let b = d.add_comment(root, "Pat", "second", None).unwrap();
        assert_ne!(a, b);
        assert_eq!(d.boxes[&root].comments.len(), 2);
        assert!(!d.boxes[&root].comments[0].created.is_empty());
        d.remove_comment(root, a).unwrap();
        assert_eq!(d.boxes[&root].comments.len(), 1);
        assert_eq!(d.boxes[&root].comments[0].id, b);
        assert_eq!(d.remove_comment(root, a), Err(EditError::NotFound));
        assert_eq!(
            d.add_comment(NodeId::new(), "x", "y", None),
            Err(EditError::NotFound)
        );
        let mut bad = d.clone();
        let dup = bad.boxes[&root].comments[0].clone();
        bad.boxes[&root].comments.push(dup);
        assert!(bad.validate().is_err());
    }

    #[test]
    fn link_trimmed_and_empty_is_none() {
        let mut d = Document::new("C");
        let root = d.root;
        d.set_link(root, Some("  https://example.org  ".into()))
            .unwrap();
        assert_eq!(d.boxes[&root].link.as_deref(), Some("https://example.org"));
        d.set_link(root, Some("  ".into())).unwrap();
        assert_eq!(d.boxes[&root].link, None);
        d.set_link(root, Some("x".into())).unwrap();
        d.set_link(root, None).unwrap();
        assert_eq!(d.boxes[&root].link, None);
        assert_eq!(d.set_link(NodeId::new(), None), Err(EditError::NotFound));
    }

    #[test]
    fn images_not_on_notes() {
        let mut d = Document::new("C");
        let note = d.add_box(Parent::Box(d.root), BoxKind::Note, "n").unwrap();
        let img = || {
            Some(ImageRef {
                asset: "k".into(),
                aspect: 0.75,
            })
        };
        assert_eq!(d.set_image(note, img()), Err(EditError::NotAllowed));
        let root = d.root;
        d.set_image(root, img()).unwrap();
        assert!(d.boxes[&root].image.is_some());
        d.set_image(root, None).unwrap();
        assert!(d.boxes[&root].image.is_none());
        let mut bad = d.clone();
        bad.boxes[&note] = {
            let mut n = BoxNode::new(note, BoxKind::Note, "n");
            n.image = img();
            n
        };
        assert!(bad.validate().is_err());
    }
}
