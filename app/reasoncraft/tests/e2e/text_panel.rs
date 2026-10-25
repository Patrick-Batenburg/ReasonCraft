//! Step 3 end to end: snippets (one tagged with a source), a selection
//! dropped onto a premise, the document language and the outline
//! switches, through actions only; then save, reopen, and compare the
//! document, the outline text and the issues.

use reasoncraft::actions::{Action, Insertable, apply};
use reasoncraft::app::ReasonCraftApp;
use reasoncraft::file_ops::FakeDialogs;
use reasoncraft::settings::{OutlineToggle, TextTab};
use reasoncraft_core::checks::{CheckKind, Issue, run_checks};
use reasoncraft_core::outline::{LinkLabel, build_outline, outline_plain_text};
use reasoncraft_core::{BoxKind, Item, Parent, SnippetSource, SourceType};

use super::{editing, run, typed};

/// The outline as the app would copy it, with stand-in words (the words
/// only need to be the same before and after).
fn outline_text(app: &ReasonCraftApp) -> String {
    let opts = app.settings.outline;
    let lines = build_outline(&app.active_tab().doc, &opts);
    outline_plain_text(
        &lines,
        &opts,
        &|l: LinkLabel| format!("{l:?}"),
        &|s: &SourceType| s.key().to_owned(),
    )
}

fn issues(app: &ReasonCraftApp) -> Vec<Issue> {
    run_checks(&app.active_tab().doc, &app.settings.checks)
}

#[test]
fn e2e_snippets_language_outline_save_reopen() {
    let fake = FakeDialogs::default();
    let mut app = ReasonCraftApp::new_for_test_with(Box::new(fake.clone()));
    let root = app.active_tab().doc.root;

    // A claim that is compound only in Dutch ("of"), and one reason.
    run(
        &mut app,
        [
            typed("Robots of mensen moeten het zware werk doen"),
            Action::AddReason,
            typed("Robots worden nooit moe"),
        ],
    );
    let premise = editing(&app);
    run(
        &mut app,
        [Action::StopEditing, Action::SetTextTab(TextTab::Snippets)],
    );

    // Two snippets; the first is tagged with a source.
    let quote = "Uit onderzoek blijkt dat robots zwaar werk overnemen zonder pauze.";
    run(
        &mut app,
        [
            Action::AddSnippet(quote.into()),
            Action::AddSnippet("Mensen raken sneller vermoeid dan machines.".into()),
        ],
    );
    let (s1, s2) = {
        let sn = &app.active_tab().doc.snippets;
        assert_eq!(sn.len(), 2);
        (sn[0].id, sn[1].id)
    };
    let source = SnippetSource {
        source_type: SourceType::Statistic,
        citation: "CBS (2024)".into(),
        link: Some("https://example.org/cbs".into()),
    };
    run(
        &mut app,
        [Action::SetSnippetSource {
            id: s1,
            source: Some(source.clone()),
        }],
    );
    // The second snippet moves to the top.
    run(&mut app, [Action::MoveSnippet { id: s2, index: 0 }]);
    let ids: Vec<_> = app.active_tab().doc.snippets.iter().map(|s| s.id).collect();
    assert_eq!(ids, vec![s2, s1]);

    // A selection from the first snippet, dropped on the premise: a new
    // reason under it with the snippet's source, in one history step.
    let selection = "robots zwaar werk overnemen zonder pauze";
    assert!(quote.contains(selection));
    let steps = app.active_tab().history.undo.len();
    run(
        &mut app,
        [Action::Insert {
            what: Insertable::Text {
                text: format!("  {selection}\n"),
                source: Some(source.clone()),
            },
            to: Some(Parent::Box(premise)),
            on: None,
        }],
    );
    assert!(app.modal.is_none(), "drop refused: {:?}", app.modal);
    assert_eq!(app.active_tab().history.undo.len(), steps + 1, "one step");
    let dropped = {
        let doc = &app.active_tab().doc;
        let Some(Item::Box(new)) = app.active_tab().selection else {
            panic!("the dropped box is selected")
        };
        assert_eq!(doc.boxes[&new].text, selection, "trimmed");
        assert_eq!(doc.boxes[&new].kind, BoxKind::Premise);
        let under: Vec<_> = doc
            .boxes
            .values()
            .filter(|b| b.text == "CBS (2024)")
            .collect();
        assert_eq!(under.len(), 1);
        assert_eq!(
            under[0].kind,
            BoxKind::Source {
                source_type: SourceType::Statistic
            }
        );
        assert_eq!(under[0].link.as_deref(), Some("https://example.org/cbs"));
        assert_eq!(doc.owner_of(Item::Box(under[0].id)), Some(Parent::Box(new)));
        new
    };
    // Undo takes the box and its source away together; redo restores both.
    let before_drop = app.active_tab().doc.boxes.len();
    run(&mut app, [Action::Undo]);
    assert_eq!(app.active_tab().doc.boxes.len(), before_drop - 2);
    run(&mut app, [Action::Redo]);
    assert!(app.active_tab().doc.boxes.contains_key(&dropped));

    // Dutch, and the outline switches.
    let english = issues(&app);
    run(&mut app, [Action::SetDocLanguage("nl".into())]);
    assert_eq!(app.active_tab().doc.meta.language, "nl");
    let dutch = issues(&app);
    let compound = |v: &[Issue]| {
        v.iter()
            .any(|i| i.kind == CheckKind::Compound && i.item == Item::Box(root))
    };
    assert!(
        !compound(&english) && compound(&dutch),
        "\"of\" joins claims in Dutch"
    );
    run(
        &mut app,
        [
            Action::SetOutlineOption(OutlineToggle::Labels, false),
            Action::SetOutlineOption(OutlineToggle::Notes, true),
            Action::SetOutlineOption(OutlineToggle::Indent, false),
        ],
    );
    assert!(!app.settings.outline.labels && app.settings.outline.notes);
    assert!(!app.settings.outline_indent && app.settings.outline.numbering);
    let outline = outline_text(&app);
    assert!(outline.contains(selection), "{outline}");
    assert!(!outline.contains("Because"), "labels are off: {outline}");

    let built = app.active_tab().doc.clone();
    let built_issues = issues(&app);
    assert!(app.active_tab().is_dirty());

    // Save, close, open again.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("robots.rcraft");
    fake.set_save(path.clone());
    run(&mut app, [Action::Save]);
    assert!(app.modal.is_none(), "save failed: {:?}", app.modal);
    run(&mut app, [Action::CloseTab(0)]);
    assert!(app.modal.is_none(), "a saved tab closes without asking");
    fake.set_open(path);
    apply(&mut app, Action::Open);
    assert!(app.modal.is_none(), "open failed: {:?}", app.modal);

    let reopened = &app.active_tab().doc;
    assert_eq!(reopened, &built, "snippets, sources and language included");
    assert_eq!(reopened.snippets.len(), 2);
    assert_eq!(reopened.snippets[1].source.as_ref(), Some(&source));
    assert_eq!(reopened.meta.language, "nl");
    assert_eq!(outline_text(&app), outline);
    assert_eq!(issues(&app), built_issues);
    // The tab's caches serve the reopened document too.
    let opts = app.settings.outline;
    let set = app.settings.checks.clone();
    let tab = &mut app.tabs[app.active];
    assert_eq!(tab.issues(&set), built_issues.as_slice());
    assert_eq!(tab.outline(opts).len(), build_outline(&built, &opts).len());
}
