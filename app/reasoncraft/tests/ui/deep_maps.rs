//! Maps nested as deep as they go: a file holding deeper
//! branches opens with them parked, and an edit that would nest deeper
//! is refused, with a note in the status bar either way.

use eframe::egui;
use reasoncraft::ReasonCraftApp;
use reasoncraft::i18n::t_args;
use reasoncraft::shell::statusbar::NOTE_SECS;
use reasoncraft_core::fixtures::{deep_chain, deep_objection_chain};
use reasoncraft_core::{CardKind, Item, MAX_DEPTH, NewLoose, Parent};

use crate::common::{build, harness_with_doc, settle};

fn note(key: &str) -> String {
    let mut args = fluent_bundle::FluentArgs::new();
    args.set("max", MAX_DEPTH);
    t_args(key, &args)
}

#[test]
fn a_file_nested_too_deep_opens_with_its_deep_branches_parked() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("deep.rcraft");
    let (d, _) = deep_chain(2 * MAX_DEPTH + 1);
    reasoncraft_core::file::save(&d, &path).unwrap();
    let mut h = build(ReasonCraftApp::new_for_test());
    reasoncraft::file_ops::open_into_app(h.state_mut(), &path);
    h.run();
    let app = h.state();
    let doc = &app.tabs[app.active].doc;
    assert_eq!(doc.parking.len(), 1);
    assert_eq!(doc.boxes.len(), d.boxes.len());
    assert_eq!(app.status_note, Some(note("status.open_too_deep")));
}

/// A paste that would nest the map deeper than it goes is refused, and
/// the status bar says why.
#[test]
fn a_paste_too_deep_is_refused_with_a_note() {
    let (mut d, premises) = deep_chain(MAX_DEPTH);
    let loose = d.add_loose(NewLoose::Card(CardKind::Reason), "a");
    let Item::Card(loose_card) = loose else {
        unreachable!()
    };
    let loose_premise = d.cards[&loose_card].premises[0];
    d.add_card(Parent::Box(loose_premise), CardKind::Reason, "b")
        .unwrap();
    let last = Item::Box(*premises.last().unwrap());
    let mut h = harness_with_doc(d);
    h.state_mut().tabs[0].selection = Some(loose);
    h.event(egui::Event::Copy);
    h.run();
    h.state_mut().tabs[0].selection = Some(last);
    h.event(egui::Event::Paste("a\n  b\n".to_owned()));
    h.run();
    let tab = &h.state().tabs[0];
    assert!(tab.history.undo.is_empty(), "nothing pasted");
    assert_eq!(h.state().status_note, Some(note("status.too_deep")));
}

/// A chain of whole-card objections too deep for the map opens parked and
/// then rests: once its note has gone, a frame asks for no other
/// (not at once, not later). Its layout used to take time quadratic in its
/// size (10 s a frame at 30,000 levels), and Windows, taking the window
/// for hung, kept sending it events: it never rested.
#[test]
fn a_deep_objection_chain_rests_after_opening() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("objections.rcraft");
    let d = deep_objection_chain(3 * MAX_DEPTH);
    reasoncraft_core::file::save(&d, &path).unwrap();
    let mut h = build(ReasonCraftApp::new_for_test());
    reasoncraft::file_ops::open_into_app(h.state_mut(), &path);
    settle(&mut h);
    assert_eq!(h.state().tabs[h.state().active].doc.parking.len(), 2);
    // Each kittest step moves egui's clock by a quarter of a second.
    for _ in 0..(NOTE_SECS * 4.0) as usize + 2 {
        h.step();
    }
    assert_eq!(h.state().status_note, None);
    h.step();
    let delay = h.output().viewport_output[&egui::ViewportId::ROOT].repaint_delay;
    assert_eq!(
        delay,
        std::time::Duration::MAX,
        "{:?}",
        h.ctx.repaint_causes()
    );
}
