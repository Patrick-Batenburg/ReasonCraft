//! The spelling pass (F7) walks a large map a slice a frame, so the
//! window never waits for it.

use crate::common::app_with_doc;
use std::time::{Duration, Instant};

use reasoncraft::ReasonCraftApp;
use reasoncraft::actions::{Action, apply};
use reasoncraft::spell::poll;
use reasoncraft_core::{CardKind, Document, Item, NodeId, Parent};

/// An app showing a Dutch map of 500 boxes (a claim and 499 reasons), the
/// only misspelling in the last one, with the dictionary loaded. Each
/// text differs, so none comes from the cache.
fn dutch_map() -> (ReasonCraftApp, Vec<NodeId>) {
    let line = "Dit is een fietsenstalling bij het station, vlak naast de ingang";
    let mut d = Document::new(line);
    d.meta.language = "nl".to_owned();
    let root = Parent::Box(d.root);
    let mut ids = vec![d.root];
    for i in 0..499 {
        let text = match i {
            498 => "Een fietz bij het station".to_owned(),
            _ => format!("{line} {i}"),
        };
        ids.push(d.add_card(root, CardKind::Reason, &text).unwrap().1);
    }
    let mut app = app_with_doc(d);
    let start = Instant::now();
    while app.spell.get("nl").is_none() {
        assert!(start.elapsed() < Duration::from_secs(20), "nl never loaded");
        std::thread::sleep(Duration::from_millis(5));
    }
    (app, ids)
}

fn walking(app: &ReasonCraftApp) -> bool {
    reasoncraft::spell::walking(app)
}

#[test]
fn f7_pass_slices_work() {
    let (mut app, ids) = dutch_map();
    app.spell.slice_boxes = Some(50);
    let before = app.tabs[0].selection;
    assert_ne!(before, Some(Item::Box(ids[499])));
    // The first slice runs at once, the other nine one a frame.
    apply(&mut app, Action::OpenSpelling);
    let mut frames = 1;
    while walking(&app) {
        assert!(app.tabs[0].spelling.as_ref().unwrap().current.is_none());
        assert_eq!(app.tabs[0].selection, before, "not selected yet");
        poll(&mut app);
        frames += 1;
    }
    assert_eq!(frames, 500usize.div_ceil(50));
    let pass = app.tabs[0].spelling.clone().unwrap();
    assert_eq!(pass.word, "fietz");
    assert_eq!(pass.current.as_ref().unwrap().node, ids[499]);
    assert_eq!(app.tabs[0].selection, Some(Item::Box(ids[499])));
    assert!(!pass.done);
}

/// The time a frame's slice takes, measured; run with `--release --
/// --ignored` (a debug build, or a busy machine, takes longer).
#[test]
#[ignore = "timing: run in release"]
fn f7_pass_stays_within_a_frame() {
    let (mut app, ids) = dutch_map();
    apply(&mut app, Action::OpenSpelling);
    let mut frames = 1;
    while walking(&app) {
        poll(&mut app);
        frames += 1;
    }
    let pass = app.tabs[0].spelling.clone().unwrap();
    assert_eq!(pass.current.as_ref().unwrap().node, ids[499]);
    let longest = pass.longest_slice();
    assert!(
        longest <= Duration::from_millis(10),
        "{longest:?} in a frame"
    );
    eprintln!("f7 pass: {frames} frames, longest {longest:?}");
}
