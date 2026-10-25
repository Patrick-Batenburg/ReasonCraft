//! End to end: build a map through actions only (an image included), save
//! it, open it again and check that nothing was lost.

use reasoncraft::actions::{Action, apply};
use reasoncraft::app::ReasonCraftApp;
use reasoncraft::file_ops::FakeDialogs;
use reasoncraft_core::fixtures::FixedMeasure;
use reasoncraft_core::{
    BoxKind, CardId, CardKind, Item, LayoutParams, NewLoose, NodeId, Parent, Role, layout,
};

/// Apply each action in turn.
fn run(app: &mut ReasonCraftApp, actions: impl IntoIterator<Item = Action>) {
    for a in actions {
        apply(app, a);
    }
}

/// The box the active tab is editing (every add starts an edit).
fn editing(app: &ReasonCraftApp) -> NodeId {
    app.active_tab().editing.expect("an add starts an edit")
}

/// The card that owns premise `b`.
fn card_of(app: &ReasonCraftApp, b: NodeId) -> CardId {
    match app.active_tab().doc.owner_of(Item::Box(b)) {
        Some(Parent::Card(c)) => c,
        other => panic!("{b:?} is owned by {other:?}, not a card"),
    }
}

fn typed(text: &str) -> Action {
    Action::TypeText(text.to_owned())
}

#[test]
fn e2e_build_save_reopen() {
    let fake = FakeDialogs::default();
    let mut app = ReasonCraftApp::new_for_test_with(Box::new(fake.clone()));
    let root = app.active_tab().doc.root;

    // The claim (selected in a new tab), typed straight in.
    run(&mut app, [typed("Cities should ban cars from the centre")]);

    // Reason 1 with two co-premises.
    run(&mut app, [Action::AddReason, typed("Cleaner air")]);
    let r1 = editing(&app);
    run(
        &mut app,
        [Action::AddCoPremise, typed("Air quality affects health")],
    );
    let r1b = editing(&app);
    run(
        &mut app,
        [Action::AddCoPremise, typed("Traffic is the main polluter")],
    );
    let r1c = editing(&app);

    // Reason 2.
    run(
        &mut app,
        [Action::Select(Some(Item::Box(root))), Action::AddReason],
    );
    run(&mut app, [typed("Streets become safer for children")]);
    let r2 = editing(&app);

    // An objection, and (Shift+Tab while editing it) a rebuttal under it.
    run(
        &mut app,
        [Action::Select(Some(Item::Box(root))), Action::AddObjection],
    );
    run(&mut app, [typed("Shops lose customers")]);
    let o1 = editing(&app);
    run(
        &mut app,
        [
            Action::AddObjection,
            typed("Pedestrian zones raise footfall"),
        ],
    );
    let rebuttal = editing(&app);

    // A whole-card attack on reason 2.
    let r2_card = card_of(&app, r2);
    run(
        &mut app,
        [
            Action::Select(Some(Item::Card(r2_card))),
            Action::TargetWholeCard(CardKind::Objection),
            typed("Child safety does not depend on cars alone"),
        ],
    );
    let attack = editing(&app);

    // A note under the first premise, and a parking item.
    run(
        &mut app,
        [
            Action::Select(Some(Item::Box(r1))),
            Action::AddNote,
            typed("Check the 2024 air report"),
        ],
    );
    let note = editing(&app);
    run(
        &mut app,
        [
            Action::Select(None),
            Action::AddLoose(NewLoose::Card(CardKind::Reason)),
            typed("Parking spaces could become parks"),
            Action::StopEditing,
        ],
    );
    let parked = app.active_tab().doc.parking[0];

    // A photo in the first premise (stored as PNG, outside history).
    let dir = tempfile::tempdir().unwrap();
    let photo = dir.path().join("air.png");
    image::RgbImage::from_pixel(300, 200, image::Rgb([90, 140, 200]))
        .save(&photo)
        .unwrap();
    fake.set_image(photo);
    run(
        &mut app,
        [Action::Select(Some(Item::Box(r1))), Action::InsertImage],
    );
    assert!(app.modal.is_none(), "image insert failed: {:?}", app.modal);
    let asset = app.active_tab().doc.boxes[&r1].image.clone().unwrap().asset;
    let asset_bytes = app.active_tab().assets.get(&asset).unwrap();

    // The map is what was asked for.
    {
        let doc = &app.active_tab().doc;
        assert_eq!(
            doc.boxes[&root].text,
            "Cities should ban cars from the centre"
        );
        assert_eq!(doc.boxes.len(), 10, "claim, 8 premises, a note");
        assert_eq!(doc.cards.len(), 6);
        let r1_card = card_of(&app, r1);
        assert_eq!(doc.cards[&r1_card].premises, vec![r1, r1b, r1c]);
        assert_eq!(doc.card_role(card_of(&app, o1)), Role::Opposes);
        assert_eq!(doc.card_role(card_of(&app, rebuttal)), Role::Rebuts);
        let attack_card = card_of(&app, attack);
        assert_eq!(
            doc.owner_of(Item::Card(attack_card)),
            Some(Parent::Card(r2_card))
        );
        assert_eq!(doc.boxes[&note].kind, BoxKind::Note);
        assert_eq!(doc.owner_of(Item::Box(note)), Some(Parent::Box(r1)));
        assert_eq!(doc.parking.len(), 1);
        assert!(matches!(parked, Item::Card(_)));
        assert!(app.active_tab().is_dirty());
    }
    let built = app.active_tab().doc.clone();

    // Save (the dialog answers with a path) and close the clean tab.
    let path = dir.path().join("cars.rcraft");
    fake.set_save(path.clone());
    run(&mut app, [Action::Save]);
    assert!(app.modal.is_none(), "save failed: {:?}", app.modal);
    assert!(path.is_file());
    assert!(!app.active_tab().is_dirty());
    run(&mut app, [Action::CloseTab(0)]);
    assert!(app.modal.is_none(), "a saved tab closes without asking");
    assert_eq!(app.tabs.len(), 1);
    assert_ne!(
        app.active_tab().doc,
        built,
        "the tab was replaced by a new one"
    );

    // Open it again.
    fake.set_open(path.clone());
    run(&mut app, [Action::Open]);
    assert!(app.modal.is_none(), "open failed: {:?}", app.modal);
    let reopened = app.active_tab();
    assert_eq!(reopened.path.as_deref(), Some(path.as_path()));
    assert!(!reopened.is_dirty());
    assert_eq!(reopened.doc, built);
    assert_eq!(reopened.assets.get(&asset), Some(asset_bytes));

    // Same layout, box for box.
    let p = LayoutParams::default();
    let a = layout(&built, &FixedMeasure, &p);
    let b = layout(&reopened.doc, &FixedMeasure, &p);
    let near = |x: f32, y: f32| (x - y).abs() <= 0.5;
    let same = |r: eframe::egui::Rect, s: eframe::egui::Rect| {
        near(r.min.x, s.min.x)
            && near(r.min.y, s.min.y)
            && near(r.max.x, s.max.x)
            && near(r.max.y, s.max.y)
    };
    assert!(same(a.bounds, b.bounds), "{:?} vs {:?}", a.bounds, b.bounds);
    assert_eq!(a.boxes.len(), b.boxes.len());
    for (id, r) in &a.boxes {
        assert!(
            same(*r, b.boxes[id]),
            "box {id:?}: {r:?} vs {:?}",
            b.boxes[id]
        );
    }
    for (id, r) in &a.cards {
        assert!(
            same(*r, b.cards[id]),
            "card {id:?}: {r:?} vs {:?}",
            b.cards[id]
        );
    }
}
