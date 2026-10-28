//! Layout time grows with the map, not faster (SEC-R5): a chain of
//! whole-card objections, each against the card before it, lays out about
//! as fast as a chain of reasons of the same size. Its whole-card ports
//! used to be merged into their premise's group one `shift_remove` at a
//! time, which moved every later group: quadratic, about 10 s a frame for
//! 30,000 levels, so Windows took the window for hung and kept it busy.

use std::time::{Duration, Instant};

use reasoncraft_core::fixtures::{FixedMeasure, deep_chain, deep_objection_chain};
use reasoncraft_core::layout::{LayoutParams, TargetKind, layout};
use reasoncraft_core::{Document, file};

/// `doc` as it opens from a file: branches deeper than the map goes are
/// parked (ruling R2-1), as in the app.
fn opened(doc: &Document) -> Document {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("chain.rcraft");
    file::save(doc, &path).unwrap();
    file::open(&path).unwrap()
}

/// The best of three layouts of `doc`.
fn layout_time(doc: &Document) -> Duration {
    let p = LayoutParams::default();
    (0..3)
        .map(|_| {
            let start = Instant::now();
            let _ = layout(doc, &FixedMeasure, &p);
            start.elapsed()
        })
        .min()
        .unwrap()
}

#[test]
fn every_whole_card_objection_gets_its_connector() {
    let doc = opened(&deep_objection_chain(2_000));
    let l = layout(&doc, &FixedMeasure, &LayoutParams::default());
    let whole = l
        .connectors
        .iter()
        .filter(|c| c.target == TargetKind::WholeCard)
        .count();
    // Every objection but the first of each parked branch hangs on a card.
    assert_eq!(whole, 2_000 - (doc.parking.len()));
    assert_eq!(l.boxes.len(), doc.boxes.len());
}

#[test]
#[ignore = "timing: run in release"]
fn whole_card_chains_lay_out_in_linear_time() {
    let levels = 10_000;
    let objections = layout_time(&opened(&deep_objection_chain(levels)));
    let reasons = layout_time(&opened(&deep_chain(levels).0));
    println!("{levels} levels: objections {objections:?}, reasons {reasons:?}");
    assert!(
        objections < reasons * 3,
        "objections {objections:?}, reasons {reasons:?}"
    );
}
