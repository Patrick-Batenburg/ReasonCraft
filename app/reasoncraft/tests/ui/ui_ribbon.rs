//! UI tests for ribbon commands: evaluation, ratings, source menus.

use crate::common::{app_with_doc, build};
use eframe::egui;
use egui_kittest::{
    Harness,
    kittest::{NodeT, Queryable},
};
use reasoncraft::ReasonCraftApp;
use reasoncraft::a11y::in_group;
use reasoncraft::i18n::t;

fn harness() -> Harness<'static, ReasonCraftApp> {
    build(ReasonCraftApp::new_for_test())
}

fn harness_with(doc: reasoncraft_core::Document) -> Harness<'static, ReasonCraftApp> {
    build(app_with_doc(doc))
}

/// Queue `events` so they all arrive in the same frame (kittest's own
/// `event` helpers give each event a frame of its own).
fn one_frame(h: &mut Harness<'static, ReasonCraftApp>, events: Vec<egui::Event>) {
    h.input_mut().events.extend(events);
    h.run();
}

#[test]
fn evaluate_tab_rates_selected_card() {
    use reasoncraft_core::{CardKind, CardStrength, Document, Item, Parent};
    let mut d = Document::new("Claim");
    let root = d.root;
    let (card, _) = d
        .add_card(Parent::Box(root), CardKind::Reason, "Premise")
        .unwrap();
    let mut h = harness_with(d);
    h.state_mut().tabs[0].selection = Some(Item::Card(card));
    h.run();
    h.get_by_label(&t("ribbon.tab.evaluate")).click();
    h.run();
    assert_eq!(t("eval.strong"), "Strong");
    // Named with their group: a card's and a source's scale both have Nil.
    let strong = in_group(&t("group.eval_card"), &t("eval.strong"));
    let toggled =
        |h: &Harness<'static, ReasonCraftApp>| h.get_by_label(&strong).accesskit_node().toggled();
    assert_eq!(toggled(&h), Some(egui::accesskit::Toggled::False));
    // A card takes no acceptance: that group is greyed out.
    let accept = in_group(&t("group.eval_claim"), &t("eval.accept"));
    assert!(h.get_by_label(&accept).accesskit_node().is_disabled());
    h.get_by_label(&strong).click();
    h.run();
    let tab = &h.state().tabs[0];
    assert_eq!(tab.doc.cards[&card].strength, Some(CardStrength::Strong));
    assert_eq!(tab.history.undo.len(), 1);
    assert_eq!(toggled(&h), Some(egui::accesskit::Toggled::True));
}

#[test]
fn view_tab_toggles_evaluation_display() {
    let mut h = harness();
    h.run();
    h.get_by_label(&t("ribbon.tab.view")).click();
    h.run();
    h.get_by_label(&t("eval.show_markers")).click();
    h.run();
    assert!(!h.state().settings.show_markers);
    h.get_by_label(&t("eval.shading")).click();
    h.run();
    assert!(!h.state().settings.eval_shading);
}

/// Alt+1 rates the selected claim; the "1" the OS also sends is not typed.
#[test]
fn alt_digit_rates_without_typing() {
    use reasoncraft_core::{Acceptance, Document};
    let d = Document::new("Claim");
    let root = d.root;
    let mut h = harness_with(d);
    h.state_mut().tabs[0].selection = Some(reasoncraft_core::Item::Box(root));
    h.run();
    let alt = |k| egui::Event::Key {
        key: k,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::ALT,
    };
    one_frame(
        &mut h,
        vec![alt(egui::Key::Num1), egui::Event::Text("1".to_owned())],
    );
    let tab = &h.state().tabs[0];
    assert_eq!(tab.doc.boxes[&root].acceptance, Some(Acceptance::Accept));
    assert_eq!(tab.doc.boxes[&root].text, "Claim");
    assert_eq!(tab.editing(), None);
    assert_eq!(tab.history.undo.len(), 1);
    one_frame(&mut h, vec![alt(egui::Key::Num0)]);
    assert_eq!(h.state().tabs[0].doc.boxes[&root].acceptance, None);
}

/// A map with every kind of rating paints, with markers and shading on and off.
#[test]
fn rated_map_renders() {
    use reasoncraft_core::{Acceptance, BoxKind, CardStrength, SourceRating};
    let mut doc = reasoncraft_core::fixtures::sample_iot_like(40);
    let strengths = [
        Some(CardStrength::Strong),
        Some(CardStrength::Weak),
        Some(CardStrength::Nil),
        None,
    ];
    for (card, s) in doc.cards.values_mut().zip(strengths.into_iter().cycle()) {
        card.strength = s;
    }
    let acceptances = [Acceptance::Accept, Acceptance::Unsure, Acceptance::Reject];
    let ratings = [SourceRating::Solid, SourceRating::Shaky, SourceRating::Nil];
    let (mut a, mut r) = (0, 0);
    for b in doc.boxes.values_mut() {
        match b.kind {
            BoxKind::Claim | BoxKind::Premise => {
                b.acceptance = Some(acceptances[a % 3]);
                a += 1;
            }
            BoxKind::Source { .. } => {
                b.source_rating = Some(ratings[r % 3]);
                r += 1;
            }
            _ => {}
        }
    }
    assert!(a >= 3 && r >= 3, "the sample has premises and sources");
    let mut h = harness_with(doc);
    h.run_steps(3);
    // At 100% every label and marker is drawn (fitting may hide small text).
    h.state_mut().tabs[0].camera.zoom = 1.0;
    h.run_steps(3);
    h.state_mut().settings.show_markers = false;
    h.state_mut().settings.eval_shading = false;
    h.run_steps(3);
}

#[test]
fn plus_menu_source_submenu_adds_expert_opinion() {
    use reasoncraft_core::{BoxKind, CardKind, Document, Item, Parent, SourceType};
    let mut doc = Document::new("Claim");
    let (_, p) = doc
        .add_card(Parent::Box(doc.root), CardKind::Reason, "One")
        .unwrap();
    let mut h = harness_with(doc);
    // The building panel has items with the same names; hide it.
    h.state_mut().left_panel = false;
    h.state_mut().tabs[0].selection = Some(Item::Box(p));
    h.run();
    h.get_by_label(&t("plus.open")).click();
    h.run();
    // The ribbon's "Source" dropdown has the same name; the submenu is the lower one.
    let source = t("plus.source");
    let submenu = h
        .get_all_by_label(&source)
        .max_by(|a, b| a.rect().center().y.total_cmp(&b.rect().center().y))
        .expect("Source submenu in the plus menu");
    submenu.hover();
    h.run();
    h.get_by_label(&t("source.expert_opinion")).click();
    h.run();
    let tab = &h.state().tabs[0];
    let children = &tab.doc.boxes[&p].children;
    assert_eq!(children.len(), 1);
    let Item::Box(s) = children[0] else {
        panic!("not a box")
    };
    assert_eq!(
        tab.doc.boxes[&s].kind,
        BoxKind::Source {
            source_type: SourceType::ExpertOpinion
        }
    );
    assert_eq!(tab.selection, Some(Item::Box(s)));
    assert_eq!(tab.editing(), Some(s));
    assert_eq!(tab.history.undo.len(), 1);
}

#[test]
fn ribbon_source_dropdown_adds_law_to_claim() {
    use reasoncraft_core::{BoxKind, Item, SourceType};
    let mut h = harness();
    // The building panel has items with the same names; hide it.
    h.state_mut().left_panel = false;
    h.run();
    h.get_by_label(&t("action.add_source")).click();
    h.run();
    h.get_by_label(&t("source.law")).click();
    h.run();
    let tab = &h.state().tabs[0];
    let root = tab.doc.root;
    let Some(&Item::Box(s)) = tab.doc.boxes[&root].children.first() else {
        panic!("no source under the claim")
    };
    assert_eq!(
        tab.doc.boxes[&s].kind,
        BoxKind::Source {
            source_type: SourceType::Law
        }
    );
    assert_eq!(tab.editing(), Some(s));
}

#[test]
fn insert_tab_adds_items_including_whole_card_ones() {
    use reasoncraft_core::{CardKind, Document, Item, Parent};
    let mut d = Document::new("Claim");
    let root = d.root;
    let (card, premise) = d
        .add_card(Parent::Box(root), CardKind::Reason, "Premise")
        .unwrap();
    let mut h = harness_with(d);
    h.state_mut().left_panel = false;
    h.state_mut().tabs[0].selection = Some(Item::Box(premise));
    h.run();
    h.get_by_label(&t("ribbon.tab.insert")).click();
    h.run();
    h.get_by_label(&t("command.whole_card_objection")).click();
    h.run();
    let tab = &h.state().tabs[0];
    let on_card = &tab.doc.cards[&card].children;
    assert_eq!(on_card.len(), 1, "an objection to the whole card");
    assert_eq!(tab.doc.cards[&on_card[0]].kind, CardKind::Objection);
    // The tab has the box items too; nothing says "coming soon".
    for key in [
        "action.add_reason",
        "action.add_note",
        "action.comment",
        "action.link",
    ] {
        assert!(h.query_by_label(&t(key)).is_some(), "{key}");
    }
}
