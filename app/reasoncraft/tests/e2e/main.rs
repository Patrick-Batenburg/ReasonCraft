//! End to end: build and evaluate a map through actions only (ratings,
//! sources, links, comments, an image, a reorder, a "Connect to…"), save
//! it, open it again and check that nothing was lost. The step-3 text
//! panel flow is in [`text_panel`], the title/author session on the
//! close paths in [`meta_sessions`], the step-4 essay and exports in
//! [`output`], the step-5 interchange (find, spelling, Argdown, SVG,
//! recovery) in [`interchange`].

mod interchange;
mod meta_sessions;
mod output;
mod text_panel;

use reasoncraft::actions::{Action, Dir, Insertable, Rate, apply, wait_for_images};
use reasoncraft::app::ReasonCraftApp;
use reasoncraft::canvas::comments::PRESETS;
use reasoncraft::file_ops::FakeDialogs;
use reasoncraft::i18n::t;
use reasoncraft_core::fixtures::FixedMeasure;
use reasoncraft_core::{
    Acceptance, BoxKind, CardId, CardKind, CardStrength, Item, LayoutParams, NewLoose, NodeId,
    Parent, Role, SourceRating, SourceType, Verdict, file, layout,
};

/// Apply each action in turn.
fn run(app: &mut ReasonCraftApp, actions: impl IntoIterator<Item = Action>) {
    for a in actions {
        apply(app, a);
    }
}

/// The box the active tab is editing (every add starts an edit).
fn editing(app: &ReasonCraftApp) -> NodeId {
    app.active_tab().editing().expect("an add starts an edit")
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
    let Item::Card(parked_card) = parked else {
        panic!("a loose reason parks as a card")
    };
    let parked_box = app.active_tab().doc.cards[&parked_card].premises[0];

    // An expert-opinion source under reason 2, with a link.
    run(
        &mut app,
        [
            Action::Select(Some(Item::Box(r2))),
            Action::AddSource(SourceType::ExpertOpinion),
            typed("Road safety review, Dr. Lee 2023"),
        ],
    );
    let source = editing(&app);
    run(
        &mut app,
        [Action::SetLink(Some(
            "https://example.org/road-safety".into(),
        ))],
    );

    // Ratings as Alt+1/2/3 send them: the source, the claim, a card and a premise.
    run(
        &mut app,
        [
            Action::Rate(Rate::High), // the source is still selected
            Action::Select(Some(Item::Box(root))),
            Action::Rate(Rate::High),
            Action::Select(Some(Item::Card(r2_card))),
            Action::Rate(Rate::Mid),
            Action::Select(Some(Item::Box(r1b))),
            Action::Rate(Rate::Low),
        ],
    );

    // Two comments: a typed one on the claim and a preset on reason 1.
    run(
        &mut app,
        [
            Action::SetAuthor("  Reviewer  ".into()),
            Action::AddComment {
                node: root,
                text: "Say which centre".into(),
                verdict: None,
            },
            Action::Insert {
                what: Insertable::Comment(1),
                to: None,
                on: Some(r1),
            },
        ],
    );

    // Reorder: the third co-premise of reason 1 moves one place left.
    run(
        &mut app,
        [
            Action::Select(Some(Item::Box(r1c))),
            Action::Reorder(Dir::Left),
        ],
    );

    // "Connect to…" the parked reason: it goes under the claim.
    run(
        &mut app,
        [
            Action::Select(Some(Item::Box(parked_box))),
            Action::StartConnect,
        ],
    );
    assert_eq!(app.active_tab().connect_mode, Some(parked));
    run(
        &mut app,
        [Action::Move {
            item: parked,
            to: Parent::Box(root),
            index: None,
        }],
    );
    assert_eq!(
        app.active_tab().connect_mode,
        None,
        "the move ends the mode"
    );

    // A loose source keeps the parking area in the saved map.
    run(
        &mut app,
        [
            Action::Select(None),
            Action::AddSource(SourceType::Statistic),
            typed("Traffic counts 2024"),
        ],
    );
    let loose_source = editing(&app);

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
    wait_for_images(&mut app);
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
        assert_eq!(doc.boxes.len(), 12, "claim, 8 premises, a note, 2 sources");
        assert_eq!(doc.cards.len(), 6);
        let r1_card = card_of(&app, r1);
        assert_eq!(
            doc.cards[&r1_card].premises,
            vec![r1, r1c, r1b],
            "reordered"
        );
        assert_eq!(doc.card_role(card_of(&app, o1)), Role::Opposes);
        assert_eq!(doc.card_role(card_of(&app, rebuttal)), Role::Rebuts);
        let attack_card = card_of(&app, attack);
        assert_eq!(
            doc.owner_of(Item::Card(attack_card)),
            Some(Parent::Card(r2_card))
        );
        assert_eq!(doc.boxes[&note].kind, BoxKind::Note);
        assert_eq!(doc.owner_of(Item::Box(note)), Some(Parent::Box(r1)));
        // Connected, and the loose source is all that is parked.
        assert_eq!(doc.owner_of(parked), Some(Parent::Box(root)));
        assert_eq!(doc.parking, vec![Item::Box(loose_source)]);

        // Source, link and ratings.
        let s = &doc.boxes[&source];
        assert_eq!(
            s.kind,
            BoxKind::Source {
                source_type: SourceType::ExpertOpinion
            }
        );
        assert_eq!(s.link.as_deref(), Some("https://example.org/road-safety"));
        assert_eq!(s.source_rating, Some(SourceRating::Solid));
        assert_eq!(doc.owner_of(Item::Box(source)), Some(Parent::Box(r2)));
        assert_eq!(doc.boxes[&root].acceptance, Some(Acceptance::Accept));
        assert_eq!(doc.boxes[&r1b].acceptance, Some(Acceptance::Reject));
        assert_eq!(doc.cards[&r2_card].strength, Some(CardStrength::Weak));

        // Comments: the author is trimmed; the preset carries its verdict.
        let c = &doc.boxes[&root].comments;
        assert_eq!(c.len(), 1);
        assert_eq!(
            (c[0].author.as_str(), c[0].text.as_str(), c[0].verdict),
            ("Reviewer", "Say which centre", None)
        );
        let c = &doc.boxes[&r1].comments;
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].text, t(PRESETS[1].0));
        assert_eq!(c[0].verdict, Some(Verdict::Positive));
        assert!(app.active_tab().is_dirty());
    }
    let built = app.active_tab().doc.clone();
    let built_assets = app.active_tab().assets.clone();
    assert_eq!(built_assets.len(), 1);

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
    assert_eq!(reopened.assets, built_assets);

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
    assert_eq!(a.card_roles, b.card_roles);
    assert_eq!(a.card_strength, b.card_strength);
    assert_eq!(a.connectors.len(), b.connectors.len());
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

/// Review focus 5 in the app: a `.rcraft` from a newer build with an extra
/// zip entry is opened, edited and saved as a new file through the app's
/// own commands, and the entry is still there.
#[test]
fn e2e_unknown_zip_entry_survives_open_edit_save_as() {
    let dir = tempfile::tempdir().unwrap();
    let newer = dir.path().join("newer.rcraft");
    let thumb = vec![0x89, b'P', b'N', b'G', 0, 1, 2, 3];
    let mut pkg = reasoncraft_core::Package::new(reasoncraft_core::Document::new("A claim"));
    pkg.other_entries
        .push(("thumbnail.png".to_owned(), thumb.clone()));
    file::save_package(&pkg, &newer).unwrap();

    let fake = FakeDialogs::default();
    let mut app = ReasonCraftApp::new_for_test_with(Box::new(fake.clone()));
    fake.set_open(newer.clone());
    run(&mut app, [Action::Open]);
    assert!(app.modal.is_none(), "open failed: {:?}", app.modal);
    let root = app.active_tab().doc.root;
    run(
        &mut app,
        [
            Action::Select(Some(Item::Box(root))),
            Action::AddReason,
            typed("A reason"),
        ],
    );
    let copy = dir.path().join("copy.rcraft");
    fake.set_save(copy.clone());
    run(&mut app, [Action::SaveAs]);
    assert!(app.modal.is_none(), "save failed: {:?}", app.modal);

    let saved = file::open_package(&copy).unwrap();
    assert_eq!(saved.doc.boxes.len(), 2, "the edit was saved");
    assert_eq!(
        saved.other_entries,
        vec![("thumbnail.png".to_owned(), thumb)]
    );
    // The tab still holds the entry for a later save.
    assert_eq!(app.active_tab().other_entries.len(), 1);
}
