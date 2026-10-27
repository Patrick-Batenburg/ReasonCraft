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
    // By default it follows the interface language (English here).
    assert_eq!(a.settings.default_doc_language, None);
    apply(&mut a, Action::New);
    assert_eq!(tab(&a).doc.meta.language, "en");
    apply(&mut a, Action::SetDefaultDocLanguage(Some("nl".to_owned())));
    apply(&mut a, Action::SetDefaultDocLanguage(Some("xx".to_owned())));
    assert_eq!(a.settings.default_doc_language.as_deref(), Some("nl"));
    apply(&mut a, Action::New);
    assert_eq!(tab(&a).doc.meta.language, "nl");
    assert!(!tab(&a).is_dirty(), "a new document is clean");
    apply(&mut a, Action::SetDefaultDocLanguage(None));
    assert_eq!(a.settings.default_doc_language, None);
    apply(&mut a, Action::New);
    assert_eq!(tab(&a).doc.meta.language, "en");
    // Older saved settings follow the interface; bad values give English.
    let old: Settings = serde_json::from_str(r#"{"theme":"Light"}"#).unwrap();
    assert_eq!(old.default_doc_language, None);
    assert_eq!(DocTab::with_language("zz").doc.meta.language, "en");
}

#[test]
fn new_doc_language_follows_the_ui_language_unless_chosen() {
    let mut s = Settings::default();
    let dutch = || Some("nl-NL".to_owned());
    assert_eq!(s.new_doc_language(dutch), "nl");
    assert_eq!(s.new_doc_language(|| None), "en");
    s.ui_language = Some("en".to_owned());
    assert_eq!(s.new_doc_language(dutch), "en", "the UI shown, not the OS");
    s.default_doc_language = Some("nl".to_owned());
    assert_eq!(s.new_doc_language(|| None), "nl");
}

#[test]
fn ignore_is_one_undo_step() {
    use reasoncraft_core::checks::{CheckKind, CheckSet, run_checks};
    let mut a = ReasonCraftApp::new_for_test();
    let root = tab(&a).doc.root;
    a.tabs[0].edit(|d| {
        d.boxes[&root].text = "Robots help or hurt".into();
        Ok(())
    });
    let issue = run_checks(&tab(&a).doc, &CheckSet::default())
        .into_iter()
        .find(|i| i.kind == CheckKind::Compound)
        .unwrap();
    let before = tab(&a).history.undo.len();
    apply(&mut a, Action::IgnoreHint(issue.clone()));
    assert_eq!(tab(&a).history.undo.len(), before + 1);
    assert_eq!(tab(&a).doc.ignored_hints.len(), 1);
    // Ignoring it again is a no-op without a history entry.
    apply(&mut a, Action::IgnoreHint(issue));
    assert_eq!(tab(&a).history.undo.len(), before + 1);

    apply(&mut a, Action::Undo);
    assert!(tab(&a).doc.ignored_hints.is_empty());
    apply(&mut a, Action::Redo);
    assert_eq!(tab(&a).doc.ignored_hints.len(), 1);

    apply(&mut a, Action::RestoreHints);
    assert!(tab(&a).doc.ignored_hints.is_empty());
    assert_eq!(tab(&a).history.undo.len(), before + 2);
    apply(&mut a, Action::RestoreHints);
    assert_eq!(tab(&a).history.undo.len(), before + 2);
}

#[test]
fn unknown_check_kind_in_settings_is_skipped() {
    use reasoncraft_core::checks::CheckKind;
    let s: Settings =
        serde_json::from_str(r#"{"checks":{"disabled":["key_term","future_kind"]}}"#).unwrap();
    assert_eq!(s.checks.disabled, vec![CheckKind::KeyTerm]);
}
