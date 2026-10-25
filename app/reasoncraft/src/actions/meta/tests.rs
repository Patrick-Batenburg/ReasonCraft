use crate::actions::{Action, apply};
use crate::app::ReasonCraftApp;
use crate::doc_tab::{DocTab, EditSession};
use crate::settings::Settings;

fn tab(app: &ReasonCraftApp) -> &DocTab {
    &app.tabs[app.active]
}

fn meta(title: &str, author: &str) -> Action {
    Action::SetDocMeta {
        title: title.to_owned(),
        author: author.to_owned(),
    }
}

#[test]
fn typing_title_is_live_and_one_history_step() {
    let mut a = ReasonCraftApp::new_for_test();
    apply(&mut a, meta("R", ""));
    apply(&mut a, meta("Ro", ""));
    apply(&mut a, meta("Ro ", "Ann"));
    assert_eq!(tab(&a).doc.meta.title, "Ro ", "live, with the space");
    assert!(tab(&a).history.undo.is_empty());
    assert!(tab(&a).is_dirty());
    apply(&mut a, Action::ZoomFit);
    assert_eq!(tab(&a).doc.meta.title, "Ro", "trimmed on commit");
    assert_eq!(tab(&a).doc.meta.author, "Ann");
    assert_eq!(tab(&a).history.undo.len(), 1);
    apply(&mut a, Action::Undo);
    assert_eq!(tab(&a).doc.meta.title, "");
    assert_eq!(tab(&a).doc.meta.author, "");
}

#[test]
fn unchanged_or_reverted_meta_records_nothing() {
    let mut a = ReasonCraftApp::new_for_test();
    apply(&mut a, meta("", ""));
    assert!(tab(&a).session() != Some(&EditSession::Meta));
    apply(&mut a, meta("x", ""));
    apply(&mut a, meta("", ""));
    assert!(!tab(&a).is_dirty());
    apply(&mut a, Action::ZoomFit);
    assert!(tab(&a).history.undo.is_empty());
}

#[test]
fn save_commits_a_running_meta_edit() {
    let mut a = ReasonCraftApp::new_for_test();
    apply(&mut a, meta("T", ""));
    a.tabs[0].commit_session();
    assert_eq!(a.tabs[0].history.undo.len(), 1);
}

#[test]
fn language_is_validated_and_one_step() {
    let mut a = ReasonCraftApp::new_for_test();
    apply(&mut a, Action::SetDocLanguage("en".to_owned()));
    apply(&mut a, Action::SetDocLanguage("fr".to_owned()));
    assert!(tab(&a).history.undo.is_empty());
    apply(&mut a, Action::SetDocLanguage("nl".to_owned()));
    assert_eq!(tab(&a).doc.meta.language, "nl");
    assert_eq!(tab(&a).history.undo.len(), 1);
    apply(&mut a, Action::Undo);
    assert_eq!(tab(&a).doc.meta.language, "en");
}

#[test]
fn new_doc_uses_default_language() {
    let mut a = ReasonCraftApp::new_for_test();
    apply(&mut a, Action::New);
    assert_eq!(tab(&a).doc.meta.language, "en");
    apply(&mut a, Action::SetDefaultDocLanguage("nl".to_owned()));
    apply(&mut a, Action::SetDefaultDocLanguage("xx".to_owned()));
    assert_eq!(a.settings.default_doc_language, "nl");
    apply(&mut a, Action::New);
    assert_eq!(tab(&a).doc.meta.language, "nl");
    assert!(!tab(&a).is_dirty(), "a new document is clean");
    // Older saved settings and bad values fall back to English.
    let old: Settings = serde_json::from_str(r#"{"theme":"Light"}"#).unwrap();
    assert_eq!(old.default_doc_language, "en");
    assert_eq!(DocTab::with_language("zz").doc.meta.language, "en");
}
