//! End to end: a Dutch map laid out left to right with tapered
//! connectors and numbers, find and replace (one undo step), the spelling
//! pass, an Argdown round trip into a new tab, the same SVG from both
//! tabs, and autosave restored after a crash.

use reasoncraft::actions::{Action, apply};
use reasoncraft::app::ReasonCraftApp;
use reasoncraft::export::ExportFormat;
use reasoncraft::file_ops::{FakeDialogs, Modal};
use reasoncraft::recovery::{self, RecoveryChoice};
use reasoncraft_core::fixtures::structurally_equal;
use reasoncraft_core::{
    BoxKind, CardKind, ConnectorStyle, Item, LayoutDirection, NewLoose, Parent, Role, SourceType,
};

use super::output::export;
use super::{card_of, editing, run, typed};

/// Runs spelling frames (dictionary loads included) until the pass shows
/// a misspelling with its suggestions, or is done (at most 20 s).
fn wait_for_spelling(app: &mut ReasonCraftApp) {
    crate::common::poll_until("the spelling pass", || {
        reasoncraft::spell::poll(app);
        let pass = app.active_tab().spelling.as_ref().expect("a pass runs");
        pass.suggestions.is_some() || pass.done
    });
}

/// Every box's text in `app`'s active tab, sorted.
fn texts(app: &ReasonCraftApp) -> Vec<String> {
    let mut t: Vec<String> = app
        .active_tab()
        .doc
        .boxes
        .values()
        .map(|b| b.text.clone())
        .collect();
    t.sort();
    t
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

#[test]
fn interchange_round_trip() {
    let rec = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeDialogs::default();
    let mut app = ReasonCraftApp::new_for_test_with_recovery(rec.path().to_path_buf());
    app.dialogs = Box::new(fake.clone());
    assert!(
        app.modal.is_none(),
        "an empty recovery folder offers nothing"
    );
    let root = app.active_tab().doc.root;

    // 1. A Dutch map: a claim, a two-premise reason with a source, an
    // objection on one premise, a whole-card rebuttal and a parked note.
    run(
        &mut app,
        [
            Action::SetDocLanguage("nl".into()),
            Action::SetDocMeta {
                title: "Verkeer".into(),
                author: "Patrick".into(),
            },
            Action::StopEditing,
            Action::Select(Some(Item::Box(root))),
            typed("De stad moet de fiets voorrang geven"),
            Action::AddReason,
            typed("Een fiets maakt geen lawaai"),
        ],
    );
    let p1 = editing(&app);
    run(
        &mut app,
        [Action::AddCoPremise, typed("Stille straten zijn gezondt")],
    );
    let p2 = editing(&app);
    run(
        &mut app,
        [
            Action::Select(Some(Item::Box(p1))),
            Action::AddSource(SourceType::Statistic),
            typed("Geluidsmetingen van de gemeente"),
        ],
    );
    let source = editing(&app);
    run(
        &mut app,
        [
            Action::Select(Some(Item::Box(p2))),
            Action::AddObjection,
            typed("Een elektrische auto is ook stil"),
        ],
    );
    let objection = editing(&app);
    let objection_card = card_of(&app, objection);
    run(
        &mut app,
        [
            Action::Select(Some(Item::Card(objection_card))),
            Action::TargetWholeCard(CardKind::Objection),
            typed("Banden maken bij hoge snelheid toch lawaai"),
        ],
    );
    let rebuttal = editing(&app);
    run(
        &mut app,
        [
            Action::Select(None),
            Action::AddLoose(NewLoose::Note),
            typed("Navragen hoeveel mensen met de fiets komen"),
            Action::StopEditing,
        ],
    );
    {
        let doc = &app.active_tab().doc;
        assert_eq!(doc.meta.language, "nl");
        assert_eq!(doc.meta.title, "Verkeer");
        assert_eq!(doc.cards[&card_of(&app, p1)].premises, vec![p1, p2]);
        assert_eq!(doc.owner_of(Item::Box(source)), Some(Parent::Box(p1)));
        assert_eq!(
            doc.owner_of(Item::Card(objection_card)),
            Some(Parent::Box(p2))
        );
        let rebuttal_card = card_of(&app, rebuttal);
        assert_eq!(doc.card_role(rebuttal_card), Role::Rebuts);
        assert_eq!(
            doc.owner_of(Item::Card(rebuttal_card)),
            Some(Parent::Card(objection_card))
        );
        let [Item::Box(parked)] = doc.parking[..] else {
            panic!("one parked note: {:?}", doc.parking)
        };
        assert_eq!(doc.boxes[&parked].kind, BoxKind::Note);
    }

    // 2. Left to right, tapered connectors, numbers on.
    run(
        &mut app,
        [
            Action::SetLayoutDirection(LayoutDirection::LeftRight),
            Action::SetConnectorStyle(ConnectorStyle::Tapered),
            Action::ToggleNumbers,
        ],
    );
    assert_eq!(
        app.active_tab().doc.layout_direction,
        LayoutDirection::LeftRight
    );
    assert_eq!(app.settings.connector_style, ConnectorStyle::Tapered);
    assert!(app.settings.show_numbers);

    // 3. Find "fiets", replace all with "bike": one undo step.
    let before = texts(&app);
    let steps = app.active_tab().history.undo.len();
    run(
        &mut app,
        [
            Action::OpenFind { replace: true },
            Action::SetFindQuery("fiets".into()),
            Action::SetReplaceText("bike".into()),
        ],
    );
    assert_eq!(app.tabs[app.active].find_hits().len(), 3);
    run(&mut app, [Action::ReplaceAll]);
    assert_eq!(app.active_tab().history.undo.len(), steps + 1);
    let replaced = texts(&app);
    assert!(
        replaced.iter().all(|t| !t.contains("fiets")),
        "{replaced:?}"
    );
    assert_eq!(replaced.iter().filter(|t| t.contains("bike")).count(), 3);
    run(&mut app, [Action::Undo]);
    assert_eq!(texts(&app), before, "one undo restores every box");
    run(&mut app, [Action::Redo, Action::CloseFind]);
    assert_eq!(texts(&app), replaced);

    // 4. The spelling pass from the seeded misspelling: change it.
    run(
        &mut app,
        [Action::Select(Some(Item::Box(p2))), Action::OpenSpelling],
    );
    wait_for_spelling(&mut app);
    let pass = app.active_tab().spelling.clone().unwrap();
    assert_eq!(pass.word, "gezondt");
    let found = pass.suggestions.as_ref().unwrap();
    assert!(found.contains(&"gezond".to_owned()), "{pass:?}");
    let hit = pass.current.unwrap();
    assert_eq!(hit.node, p2);
    run(
        &mut app,
        [
            Action::SpellChange {
                hit,
                with: "gezond".into(),
            },
            Action::CloseSpelling,
        ],
    );
    assert_eq!(
        app.active_tab().doc.boxes[&p2].text,
        "Stille straten zijn gezond"
    );

    // 5. Export Argdown and import it into a new tab: the same map.
    let argdown = export(&mut app, &fake, dir.path(), ExportFormat::Argdown);
    let argdown_path = dir.path().join("Argdown.argdown");
    assert_eq!(std::fs::read(&argdown_path).unwrap(), argdown);
    fake.set_import(argdown_path);
    run(&mut app, [Action::ImportArgdown]);
    assert!(app.modal.is_none(), "nothing dropped: {:?}", app.modal);
    assert_eq!((app.tabs.len(), app.active), (2, 1));
    if let Err(e) = structurally_equal(&app.tabs[0].doc, &app.tabs[1].doc) {
        panic!("the import differs: {e}");
    }

    // 6. The same SVG from both tabs (Argdown has no layout direction).
    // The SVG carries no ids, so "identical apart from ids" is identical.
    run(
        &mut app,
        [Action::SetLayoutDirection(LayoutDirection::LeftRight)],
    );
    let svg_dir = |name: &str| {
        let d = dir.path().join(name);
        std::fs::create_dir(&d).unwrap();
        d
    };
    let (source_dir, import_dir) = (svg_dir("source"), svg_dir("import"));
    let imported_svg = export(&mut app, &fake, &import_dir, ExportFormat::Svg);
    run(&mut app, [Action::ActivateTab(0)]);
    let source_svg = export(&mut app, &fake, &source_dir, ExportFormat::Svg);
    assert!(!contains(&source_svg, b"id=\""), "the SVG has no ids");
    assert!(source_svg == imported_svg, "the SVGs differ");

    // 7. Autosave, crash (no shutdown), start again on the same folder
    // and restore: both tabs come back as they were.
    recovery::tick(&mut app, 0.0);
    recovery::tick(&mut app, 46.0);
    app.recovery.as_ref().unwrap().wait_idle();
    let docs: Vec<_> = app.tabs.iter().map(|t| t.doc.clone()).collect();
    drop(app);

    let mut app = ReasonCraftApp::new_for_test_with_recovery(rec.path().to_path_buf());
    match &app.modal {
        Some(Modal::Recover(offered)) => assert_eq!(offered.len(), 2),
        other => panic!("expected the recovery prompt, got {other:?}"),
    }
    apply(
        &mut app,
        Action::ResolveRecovery(RecoveryChoice::RestoreAll),
    );
    app.recovery.as_ref().unwrap().wait_idle();
    assert!(app.modal.is_none());
    assert_eq!(app.tabs.len(), 2);
    for doc in &docs {
        assert!(
            app.tabs.iter().any(|t| t.doc == *doc && t.is_dirty()),
            "{:?} was not restored",
            doc.meta.title
        );
    }
}
