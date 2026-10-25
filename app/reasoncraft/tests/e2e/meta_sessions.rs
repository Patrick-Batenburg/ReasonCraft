//! A title typed on the Document page (a running title/author session,
//! not yet committed) is committed as one history step by switching tabs,
//! closing its tab and closing the window, so the unsaved-changes prompt
//! sees it.

use reasoncraft::actions::{Action, UnsavedChoice};
use reasoncraft::app::ReasonCraftApp;
use reasoncraft::doc_tab::EditSession;
use reasoncraft::file_ops::Modal;

use super::run;

/// Two tabs, the first active, with "Draft " typed as its title.
fn app_with_typed_title() -> ReasonCraftApp {
    let mut app = ReasonCraftApp::new_for_test();
    run(
        &mut app,
        [
            Action::New,
            Action::ActivateTab(0),
            Action::SetDocMeta {
                title: "Draft ".into(),
                author: String::new(),
            },
        ],
    );
    let tab = &app.tabs[0];
    assert!(
        tab.session() == Some(&EditSession::Meta),
        "the session is running"
    );
    assert!(tab.history.undo.is_empty(), "nothing committed yet");
    assert!(tab.is_dirty(), "a running session counts as unsaved");
    app
}

/// Tab `i`'s session was committed: one step, the title trimmed.
fn committed(app: &ReasonCraftApp, i: usize) {
    let tab = &app.tabs[i];
    assert!(tab.session() != Some(&EditSession::Meta));
    assert_eq!(tab.history.undo.len(), 1);
    assert_eq!(tab.doc.meta.title, "Draft");
    assert!(tab.is_dirty());
}

fn unsaved_prompt(app: &ReasonCraftApp) -> Option<(usize, bool)> {
    match app.modal {
        Some(Modal::Unsaved { tab, window, .. }) => Some((tab, window)),
        _ => None,
    }
}

#[test]
fn switching_tabs_commits_the_title() {
    let mut app = app_with_typed_title();
    run(&mut app, [Action::ActivateTab(1)]);
    committed(&app, 0);
    // So a session only ever runs in the active tab.
    assert!(
        app.tabs
            .iter()
            .all(|t| t.session() != Some(&EditSession::Meta))
    );
}

#[test]
fn closing_the_tab_commits_the_title_and_asks() {
    let mut app = app_with_typed_title();
    run(&mut app, [Action::CloseTab(0)]);
    committed(&app, 0);
    assert_eq!(unsaved_prompt(&app), Some((0, false)));
    run(&mut app, [Action::ResolveUnsaved(UnsavedChoice::Cancel)]);
    assert_eq!(app.tabs.len(), 2, "cancel keeps the tab");
    assert_eq!(app.tabs[0].doc.meta.title, "Draft");
}

#[test]
fn closing_the_window_commits_the_title_and_asks() {
    let mut app = app_with_typed_title();
    run(&mut app, [Action::CloseWindow]);
    committed(&app, 0);
    assert_eq!(unsaved_prompt(&app), Some((0, true)));
}

#[test]
fn closing_an_inactive_tab_commits_its_title_and_asks() {
    // Through actions, switching away already commits (see above); closing
    // tab 0 from tab 1 then finds it dirty.
    let mut app = app_with_typed_title();
    run(&mut app, [Action::ActivateTab(1), Action::CloseTab(0)]);
    committed(&app, 0);
    assert_eq!(unsaved_prompt(&app), Some((0, false)));

    // Defence in depth: a session left running in a background tab (not
    // reachable through actions) is committed by closing that tab too.
    let mut app = ReasonCraftApp::new_for_test();
    run(&mut app, [Action::New]);
    assert_eq!(app.active, 1);
    app.tabs[0].edit_meta_live("Draft ", "");
    assert!(app.tabs[0].session() == Some(&EditSession::Meta));
    run(&mut app, [Action::CloseTab(0)]);
    committed(&app, 0);
    assert_eq!(unsaved_prompt(&app), Some((0, false)));
    run(&mut app, [Action::ResolveUnsaved(UnsavedChoice::Discard)]);
    assert_eq!(app.tabs.len(), 1, "discard closes it");
}
