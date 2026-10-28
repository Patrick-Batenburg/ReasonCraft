//! Cut, Copy and Paste of map items with the keys (which egui hands over
//! as events) and from the ribbon.

use eframe::egui;
use egui_kittest::kittest::Queryable;
use reasoncraft::i18n::t;
use reasoncraft_core::{CardKind, Document, Item, Parent};

use crate::common::harness_with_doc;

fn doc() -> (Document, Item) {
    let mut d = Document::new("Claim");
    let (card, _) = d
        .add_card(Parent::Box(d.root), CardKind::Reason, "Robots are cheap")
        .unwrap();
    (d, Item::Card(card))
}

#[test]
fn ctrl_c_and_ctrl_v_on_the_map_copy_a_card() {
    let (d, card) = doc();
    let root = Item::Box(d.root);
    let mut h = harness_with_doc(d);
    h.state_mut().tabs[0].selection = Some(card);
    h.event(egui::Event::Copy);
    h.run();
    assert!(h.state().clipboard.is_some());
    h.state_mut().tabs[0].selection = Some(root);
    let text = "Robots are cheap\n".to_owned();
    h.event(egui::Event::Paste(text));
    h.run();
    let tab = &h.state().tabs[0];
    assert_eq!(tab.doc.cards.len(), 2);
    assert_eq!(tab.history.undo.len(), 1);
}

#[test]
fn ribbon_cut_and_paste() {
    let (d, card) = doc();
    let root = Item::Box(d.root);
    let mut h = harness_with_doc(d);
    h.state_mut().tabs[0].selection = Some(card);
    h.run();
    h.get_by_label(&t("action.cut")).click();
    h.run();
    assert!(h.state().tabs[0].doc.cards.is_empty());
    h.state_mut().tabs[0].selection = Some(root);
    h.run();
    h.get_by_label(&t("action.paste")).click();
    h.run();
    assert_eq!(h.state().tabs[0].doc.cards.len(), 1);
}

/// While a box is edited inline, its editor keeps Ctrl+C and Ctrl+V.
#[test]
fn the_box_editor_keeps_the_keys() {
    let (d, card) = doc();
    let mut h = harness_with_doc(d);
    h.state_mut().tabs[0].selection = Some(card);
    h.event(egui::Event::Copy);
    h.run();
    let Item::Card(c) = card else { unreachable!() };
    let premise = h.state().tabs[0].doc.cards[&c].premises[0];
    h.state_mut().tabs[0].selection = Some(Item::Box(premise));
    h.state_mut()
        .pending
        .push(reasoncraft::actions::Action::Edit);
    h.run();
    h.event(egui::Event::Paste("Robots are cheap\n".into()));
    h.run();
    assert_eq!(h.state().tabs[0].doc.cards.len(), 1, "no card pasted");
}
