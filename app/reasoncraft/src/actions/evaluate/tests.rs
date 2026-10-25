use reasoncraft_core::{BoxKind, CardId, CardKind, Item, NodeId, Parent};

use super::Rate;
use crate::actions::{Action, apply, is_enabled};
use crate::app::ReasonCraftApp;
use crate::doc_tab::DocTab;

fn app() -> ReasonCraftApp {
    ReasonCraftApp::new_for_test()
}

fn tab(app: &ReasonCraftApp) -> &DocTab {
    &app.tabs[app.active]
}

/// root ← card [premise] ← source under the premise; the app shows it.
fn rated_app() -> (ReasonCraftApp, CardId, NodeId, NodeId) {
    use reasoncraft_core::{Document, SourceType};
    let mut d = Document::new("Claim");
    let root = d.root;
    let (card, premise) = d
        .add_card(Parent::Box(root), CardKind::Reason, "p")
        .unwrap();
    let source = d
        .add_box(
            Parent::Box(premise),
            BoxKind::Source {
                source_type: SourceType::Data,
            },
            "s",
        )
        .unwrap();
    let mut a = app();
    a.tabs[0] = DocTab::from_document(d);
    (a, card, premise, source)
}

#[test]
fn rate_maps_per_selection_kind() {
    use reasoncraft_core::{Acceptance, CardStrength, SourceRating};
    let (mut a, card, premise, source) = rated_app();

    apply(&mut a, Action::Select(Some(Item::Box(premise))));
    apply(&mut a, Action::Rate(Rate::High));
    assert_eq!(
        tab(&a).doc.boxes[&premise].acceptance,
        Some(Acceptance::Accept)
    );
    apply(&mut a, Action::Rate(Rate::Low));
    assert_eq!(
        tab(&a).doc.boxes[&premise].acceptance,
        Some(Acceptance::Reject)
    );

    apply(&mut a, Action::Select(Some(Item::Card(card))));
    apply(&mut a, Action::Rate(Rate::High));
    assert_eq!(
        tab(&a).doc.cards[&card].strength,
        Some(CardStrength::Strong)
    );
    apply(&mut a, Action::Rate(Rate::Mid));
    assert_eq!(tab(&a).doc.cards[&card].strength, Some(CardStrength::Weak));

    apply(&mut a, Action::Select(Some(Item::Box(source))));
    apply(&mut a, Action::Rate(Rate::High));
    assert_eq!(
        tab(&a).doc.boxes[&source].source_rating,
        Some(SourceRating::Solid)
    );
    apply(&mut a, Action::Rate(Rate::Low));
    assert_eq!(
        tab(&a).doc.boxes[&source].source_rating,
        Some(SourceRating::Nil)
    );
    let steps = tab(&a).history.undo.len();
    assert_eq!(steps, 6);

    // Clear: each kind loses its rating, one step each.
    apply(&mut a, Action::Rate(Rate::Clear));
    assert_eq!(tab(&a).doc.boxes[&source].source_rating, None);
    apply(&mut a, Action::Select(Some(Item::Card(card))));
    apply(&mut a, Action::Rate(Rate::Clear));
    assert_eq!(tab(&a).doc.cards[&card].strength, None);
    apply(&mut a, Action::Select(Some(Item::Box(premise))));
    apply(&mut a, Action::Rate(Rate::Clear));
    assert_eq!(tab(&a).doc.boxes[&premise].acceptance, None);
    assert_eq!(tab(&a).history.undo.len(), steps + 3);

    // Clearing what is already clear records nothing.
    apply(&mut a, Action::Rate(Rate::Clear));
    assert_eq!(tab(&a).history.undo.len(), steps + 3);
}

#[test]
fn rating_refused_records_no_history() {
    use reasoncraft_core::{Acceptance, CardStrength, ClearScope, SourceRating};
    let (mut a, card, premise, _) = rated_app();
    let root = tab(&a).doc.root;
    apply(&mut a, Action::Select(Some(Item::Box(root))));
    let strong = Action::SetStrength(Some(CardStrength::Strong));
    assert!(!is_enabled(tab(&a), &strong));
    apply(&mut a, strong);
    assert_eq!(tab(&a).history.undo.len(), 0);
    assert_eq!(tab(&a).doc.cards[&card].strength, None);
    let solid = Action::SetSourceRating(Some(SourceRating::Solid));
    assert!(!is_enabled(tab(&a), &solid));
    apply(&mut a, solid);
    assert_eq!(tab(&a).history.undo.len(), 0);
    // A claim takes acceptance.
    assert!(is_enabled(
        tab(&a),
        &Action::SetAcceptance(Some(Acceptance::Accept))
    ));

    // Categories, notes and an empty selection take no rating at all.
    apply(&mut a, Action::Select(Some(Item::Box(premise))));
    apply(&mut a, Action::AddNote);
    apply(&mut a, Action::StopEditing);
    let steps = tab(&a).history.undo.len();
    for rate in [Rate::High, Rate::Mid, Rate::Low, Rate::Clear] {
        assert!(!is_enabled(tab(&a), &Action::Rate(rate)));
        apply(&mut a, Action::Rate(rate));
    }
    apply(&mut a, Action::Select(None));
    for rate in [Rate::High, Rate::Clear] {
        assert!(!is_enabled(tab(&a), &Action::Rate(rate)));
        apply(&mut a, Action::Rate(rate));
    }
    assert_eq!(tab(&a).history.undo.len(), steps);

    // Nothing rated: clearing is disabled and records nothing.
    let map = Action::ClearEvaluation(ClearScope::Map);
    assert!(!is_enabled(tab(&a), &map));
    apply(&mut a, map.clone());
    assert_eq!(tab(&a).history.undo.len(), steps);
    apply(&mut a, Action::Select(Some(Item::Card(card))));
    apply(&mut a, Action::Rate(Rate::High));
    assert!(is_enabled(tab(&a), &map));
    let item = Action::ClearEvaluation(ClearScope::Item(Item::Card(card)));
    assert!(is_enabled(tab(&a), &item));
    assert!(!is_enabled(
        tab(&a),
        &Action::ClearEvaluation(ClearScope::Item(Item::Box(premise)))
    ));
    apply(&mut a, map);
    assert_eq!(tab(&a).doc.cards[&card].strength, None);
    assert_eq!(tab(&a).history.undo.len(), steps + 2);
}

#[test]
fn toggles_flip_settings() {
    let mut a = app();
    assert!(a.settings.show_markers && a.settings.eval_shading);
    apply(&mut a, Action::ToggleMarkers);
    assert!(!a.settings.show_markers && a.settings.eval_shading);
    apply(&mut a, Action::ToggleShading);
    assert!(!a.settings.eval_shading);
    apply(&mut a, Action::SetTheme(crate::theme::ThemeChoice::Light));
    assert_eq!(a.settings.theme, crate::theme::ThemeChoice::Light);
}
