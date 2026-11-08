//! Why an add command is greyed out: the short hint its ribbon tooltip
//! shows ("Select a premise to add a co-premise.").

use reasoncraft_core::{BoxKind, Item};

use super::{Action, is_enabled};
use crate::doc_tab::DocTab;

/// The actions [`why_disabled`] can explain: the ribbon's add commands.
pub const EXPLAINED: [fn() -> Action; 9] = [
    || Action::AddReason,
    || Action::AddObjection,
    || Action::AddCoPremise,
    || Action::AddCategory,
    || Action::AddNote,
    || Action::CommentSelected,
    || Action::TargetWholeCard(reasoncraft_core::CardKind::Reason),
    || Action::TargetWholeCard(reasoncraft_core::CardKind::Objection),
    || Action::AddSource(reasoncraft_core::SourceType::Source),
];

/// The i18n key of why `action` cannot run for `tab` right now, if it
/// cannot and is an add command (see [`EXPLAINED`]).
pub fn why_disabled(tab: &DocTab, action: &Action) -> Option<&'static str> {
    if is_enabled(tab, action) {
        return None;
    }
    let claim = match tab.selection {
        Some(Item::Box(b)) => tab.doc.boxes.get(&b).map(|n| n.kind == BoxKind::Claim),
        _ => None,
    } == Some(true);
    Some(match action {
        // A claim takes cards unless it has categories.
        Action::AddReason | Action::AddObjection if claim => "ribbon.why.claim_has_categories",
        Action::AddReason | Action::AddObjection => "ribbon.why.add_card",
        Action::AddCoPremise => "ribbon.why.add_co_premise",
        // A claim takes categories unless it has cards.
        Action::AddCategory if claim => "ribbon.why.claim_has_cards",
        Action::AddCategory => "ribbon.why.add_category",
        Action::AddNote => "ribbon.why.add_note",
        Action::CommentSelected => "ribbon.why.comment",
        Action::TargetWholeCard(_) => "ribbon.why.whole_card",
        Action::AddSource(_) => "ribbon.why.add_source",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::apply;
    use crate::app::ReasonCraftApp;
    use reasoncraft_core::{CardKind, Document, Parent};

    fn tab_with(select: impl FnOnce(&mut Document) -> Option<Item>) -> DocTab {
        let mut d = Document::new("Claim");
        let item = select(&mut d);
        let mut tab = DocTab::from_document(d);
        tab.selection = item;
        tab
    }

    #[test]
    fn every_explained_action_has_a_reason_when_greyed_out() {
        // A note takes nothing; a card no category, note or comment.
        let note = tab_with(|d| {
            let root = Parent::Box(d.root);
            let n = d.add_box(root, BoxKind::Note, "n").unwrap();
            Some(Item::Box(n))
        });
        for ctor in EXPLAINED {
            let action = ctor();
            if !is_enabled(&note, &action) {
                assert!(why_disabled(&note, &action).is_some(), "{action:?}");
            }
        }
        assert_eq!(
            why_disabled(&note, &Action::AddCoPremise),
            Some("ribbon.why.add_co_premise")
        );
        assert_eq!(
            why_disabled(&note, &Action::AddReason),
            Some("ribbon.why.add_card")
        );
        assert_eq!(why_disabled(&note, &Action::Undo), None, "not an add");
    }

    #[test]
    fn a_claim_says_what_it_already_holds() {
        let cards = tab_with(|d| {
            let root = Parent::Box(d.root);
            d.add_card(root, CardKind::Reason, "r").unwrap();
            Some(Item::Box(d.root))
        });
        assert_eq!(
            why_disabled(&cards, &Action::AddCategory),
            Some("ribbon.why.claim_has_cards")
        );
        assert_eq!(why_disabled(&cards, &Action::AddReason), None, "enabled");
        let mut a = ReasonCraftApp::new_for_test();
        apply(&mut a, Action::AddCategory);
        let root = a.tabs[0].doc.root;
        a.tabs[0].selection = Some(Item::Box(root));
        assert_eq!(
            why_disabled(&a.tabs[0], &Action::AddObjection),
            Some("ribbon.why.claim_has_categories")
        );
    }
}
