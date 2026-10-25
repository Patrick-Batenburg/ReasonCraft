use reasoncraft_core::{Item, MAX_SNIPPET_CHARS, SnippetSource, SourceType, Uuid};

use crate::actions::{Action, apply, is_enabled};
use crate::app::ReasonCraftApp;
use crate::doc_tab::DocTab;

fn tab(app: &ReasonCraftApp) -> &DocTab {
    &app.tabs[app.active]
}

/// An app whose document has snippets with these texts (no history).
fn app_with(texts: &[&str]) -> (ReasonCraftApp, Vec<Uuid>) {
    let mut a = ReasonCraftApp::new_for_test();
    let doc = &mut a.tabs[0].doc;
    let ids = texts.iter().map(|s| doc.add_snippet(s)).collect();
    (a, ids)
}

fn edit(id: Uuid, text: &str) -> Action {
    Action::EditSnippetText {
        id,
        text: text.to_owned(),
    }
}

fn texts(a: &ReasonCraftApp) -> Vec<String> {
    tab(a).doc.snippets.iter().map(|s| s.text.clone()).collect()
}

fn source(st: SourceType, citation: &str, link: Option<&str>) -> Option<SnippetSource> {
    Some(SnippetSource {
        source_type: st,
        citation: citation.to_owned(),
        link: link.map(Into::into),
    })
}

fn set_source(id: Uuid, source: Option<SnippetSource>) -> Action {
    Action::SetSnippetSource { id, source }
}

fn undo_len(a: &ReasonCraftApp) -> usize {
    tab(a).history.undo.len()
}

#[test]
fn snippet_edit_is_one_history_entry() {
    let (mut a, ids) = app_with(&["Robots"]);
    let rev = tab(&a).revision();
    for text in ["Robots ", "Robots h", "Robots help"] {
        apply(&mut a, edit(ids[0], text));
    }
    assert_eq!(texts(&a), ["Robots help"]);
    assert_eq!(undo_len(&a), 0, "typing is live");
    assert!(tab(&a).revision() > rev, "derived views see the typing");
    assert!(tab(&a).is_dirty());
    assert!(is_enabled(tab(&a), &Action::Undo));

    apply(&mut a, Action::StopEditing);
    assert_eq!(undo_len(&a), 1);
    assert!(tab(&a).snippet_edit.is_none());
    apply(&mut a, Action::Undo);
    assert_eq!(texts(&a), ["Robots"]);
}

#[test]
fn unchanged_snippet_text_records_nothing() {
    let (mut a, ids) = app_with(&["Same"]);
    let rev = tab(&a).revision();
    apply(&mut a, edit(ids[0], "Same"));
    apply(&mut a, Action::StopEditing);
    assert_eq!(undo_len(&a), 0);
    assert_eq!(tab(&a).revision(), rev);
    // Typed and typed back: no step either.
    apply(&mut a, edit(ids[0], "Samey"));
    apply(&mut a, edit(ids[0], "Same"));
    assert!(!tab(&a).is_dirty());
    apply(&mut a, Action::StopEditing);
    assert_eq!(undo_len(&a), 0);
}

#[test]
fn non_text_action_commits_snippet_session() {
    let (mut a, ids) = app_with(&["a", "b"]);
    apply(&mut a, edit(ids[0], "a1"));
    // Editing another snippet ends the first session.
    apply(&mut a, edit(ids[1], "b1"));
    assert_eq!(undo_len(&a), 1);
    let root = tab(&a).doc.root;
    apply(&mut a, Action::Select(Some(Item::Box(root))));
    assert_eq!(undo_len(&a), 2);
    apply(&mut a, Action::Undo);
    assert_eq!(texts(&a), ["a1", "b"]);
}

#[test]
fn box_edit_and_snippet_edit_are_exclusive() {
    let (mut a, ids) = app_with(&["s"]);
    let root = tab(&a).doc.root;
    apply(&mut a, Action::Edit);
    apply(
        &mut a,
        Action::EditText {
            node: root,
            text: "Claim".into(),
        },
    );
    // Starting a snippet session commits the box edit.
    apply(&mut a, edit(ids[0], "s1"));
    assert_eq!(tab(&a).editing, None);
    assert_eq!(undo_len(&a), 1);
    assert!(tab(&a).snippet_edit.is_some());
    // Starting a box edit commits the snippet session.
    apply(&mut a, Action::TypeText("!".into()));
    assert!(tab(&a).snippet_edit.is_none());
    assert_eq!(tab(&a).editing, Some(root));
    assert_eq!(undo_len(&a), 2);
    apply(&mut a, Action::StopEditing);
    assert_eq!(tab(&a).doc.boxes[&root].text, "Claim!");
    assert_eq!(texts(&a), ["s1"]);
    assert_eq!(undo_len(&a), 3);
}

#[test]
fn new_snippet_and_its_first_text_are_one_step() {
    let (mut a, _) = app_with(&[]);
    apply(&mut a, Action::AddSnippet(String::new()));
    let id = tab(&a).doc.snippets[0].id;
    assert_eq!(
        tab(&a).snippet_focus,
        Some(id),
        "the new snippet takes focus"
    );
    assert!(tab(&a).is_dirty());
    apply(&mut a, edit(id, "Quote"));
    apply(&mut a, Action::StopEditing);
    assert_eq!(undo_len(&a), 1);
    apply(&mut a, Action::Undo);
    assert!(tab(&a).doc.snippets.is_empty());
    assert!(tab(&a).snippet_focus.is_none());

    // An empty snippet left alone still counts as one step.
    apply(&mut a, Action::AddSnippet(String::new()));
    apply(&mut a, Action::StopEditing);
    assert_eq!(undo_len(&a), 1);
    assert_eq!(tab(&a).doc.snippets.len(), 1);
}

#[test]
fn pasted_snippet_is_one_step() {
    let (mut a, _) = app_with(&[]);
    apply(&mut a, Action::AddSnippet("Pasted text".into()));
    assert_eq!(texts(&a), ["Pasted text"]);
    assert_eq!(undo_len(&a), 1);
    assert!(tab(&a).snippet_capped.is_none());
    assert!(tab(&a).snippet_edit.is_none());
    let id = tab(&a).doc.snippets[0].id;
    assert_eq!(tab(&a).snippet_focus, Some(id), "shown and focused");
}

#[test]
fn oversized_paste_is_capped() {
    let huge = "x".repeat(120_000);
    let (mut a, _) = app_with(&[]);
    apply(&mut a, Action::AddSnippet(huge.clone()));
    let id = tab(&a).doc.snippets[0].id;
    let len = |a: &ReasonCraftApp| tab(a).doc.snippets[0].text.chars().count();
    assert_eq!(len(&a), MAX_SNIPPET_CHARS);
    assert_eq!(tab(&a).snippet_capped, Some(id));

    let (mut a, ids) = app_with(&["short"]);
    apply(&mut a, edit(ids[0], &huge));
    assert_eq!(len(&a), MAX_SNIPPET_CHARS);
    assert_eq!(tab(&a).snippet_capped, Some(ids[0]));
    // A later edit within the limit clears the notice.
    apply(&mut a, edit(ids[0], "short again"));
    assert!(tab(&a).snippet_capped.is_none());
    // Characters are counted, not bytes.
    let wide = "é".repeat(MAX_SNIPPET_CHARS + 5);
    apply(&mut a, edit(ids[0], &wide));
    assert_eq!(len(&a), MAX_SNIPPET_CHARS);
}

#[test]
fn invalid_snippet_link_not_stored() {
    let (mut a, ids) = app_with(&["s"]);
    for bad in ["javascript:alert(1)", "file:///c:/x", "not a link"] {
        let s = source(SourceType::Web, "Doe", Some(bad));
        apply(&mut a, set_source(ids[0], s));
        assert_eq!(tab(&a).doc.snippets[0].source, None, "{bad}");
    }
    assert_eq!(undo_len(&a), 0);

    let s = source(SourceType::Web, "Doe", Some("example.org/x"));
    apply(&mut a, set_source(ids[0], s));
    let s = tab(&a).doc.snippets[0].source.clone().unwrap();
    assert_eq!(s.link.as_deref(), Some("https://example.org/x"));
    assert_eq!(undo_len(&a), 1);
}

#[test]
fn snippet_source_is_trimmed_and_empty_means_none() {
    let (mut a, ids) = app_with(&["s"]);
    let s = source(SourceType::Data, "  CBS 2024 ", None);
    apply(&mut a, set_source(ids[0], s));
    let s = tab(&a).doc.snippets[0].source.clone().unwrap();
    assert_eq!(s.citation, "CBS 2024");
    assert_eq!(undo_len(&a), 1);
    // The same source again is no change.
    let rev = tab(&a).revision();
    let s = source(SourceType::Data, "CBS 2024", Some(" "));
    apply(&mut a, set_source(ids[0], s));
    assert_eq!(undo_len(&a), 1);
    assert_eq!(tab(&a).revision(), rev);
    // Whitespace citation, no link, generic type: no source at all.
    let s = source(SourceType::Source, "   ", None);
    apply(&mut a, set_source(ids[0], s));
    assert_eq!(tab(&a).doc.snippets[0].source, None);
    assert_eq!(undo_len(&a), 2);
    // A type alone is a source.
    apply(
        &mut a,
        set_source(ids[0], source(SourceType::Quote, "", None)),
    );
    let s = tab(&a).doc.snippets[0].source.clone().unwrap();
    assert_eq!(
        (s.source_type, s.citation.as_str()),
        (SourceType::Quote, "")
    );
}

#[test]
fn snippet_reorder() {
    let (mut a, ids) = app_with(&["a", "b", "c"]);
    apply(
        &mut a,
        Action::MoveSnippet {
            id: ids[2],
            index: 0,
        },
    );
    assert_eq!(texts(&a), ["c", "a", "b"]);
    assert_eq!(undo_len(&a), 1);
    // Where it already is (or past the end when last): nothing happens.
    let rev = tab(&a).revision();
    apply(
        &mut a,
        Action::MoveSnippet {
            id: ids[2],
            index: 0,
        },
    );
    apply(
        &mut a,
        Action::MoveSnippet {
            id: ids[1],
            index: 99,
        },
    );
    assert_eq!(undo_len(&a), 1);
    assert_eq!(tab(&a).revision(), rev);
    apply(&mut a, Action::Undo);
    assert_eq!(texts(&a), ["a", "b", "c"]);
}

#[test]
fn remove_snippet_is_one_step() {
    let (mut a, ids) = app_with(&["a", "b"]);
    apply(&mut a, edit(ids[0], "a1"));
    apply(&mut a, Action::RemoveSnippet(ids[0]));
    assert_eq!(texts(&a), ["b"]);
    assert_eq!(undo_len(&a), 2, "the edit, then the removal");
    apply(&mut a, Action::RemoveSnippet(ids[0]));
    assert_eq!(undo_len(&a), 2, "gone already: nothing");
    apply(&mut a, Action::Undo);
    assert_eq!(texts(&a), ["a1", "b"]);
}

fn cite(id: Uuid, citation: &str) -> Action {
    Action::EditSnippetCitation {
        id,
        citation: citation.to_owned(),
    }
}

fn link(id: Uuid, link: &str) -> Action {
    Action::EditSnippetLink {
        id,
        link: link.to_owned(),
    }
}

fn stored_link(a: &ReasonCraftApp) -> Option<String> {
    tab(a).doc.snippets[0].source.as_ref()?.link.clone()
}

#[test]
fn citation_typing_is_live_and_one_step() {
    let (mut a, ids) = app_with(&["s"]);
    let rev = tab(&a).revision();
    for c in ["D", "Do", "Doe 2020 "] {
        apply(&mut a, cite(ids[0], c));
    }
    let citation = |a: &ReasonCraftApp| {
        let s = tab(a).doc.snippets[0].source.as_ref();
        s.map(|s| s.citation.clone())
    };
    assert_eq!(citation(&a).as_deref(), Some("Doe 2020 "), "live");
    assert_eq!(undo_len(&a), 0);
    assert!(tab(&a).revision() > rev);
    assert!(tab(&a).is_dirty());
    // Text typed in the same snippet joins the session.
    apply(&mut a, edit(ids[0], "s2"));
    apply(&mut a, Action::StopEditing);
    assert_eq!(undo_len(&a), 1);
    assert_eq!(
        citation(&a).as_deref(),
        Some("Doe 2020"),
        "trimmed on commit"
    );
    apply(&mut a, Action::Undo);
    assert_eq!(tab(&a).doc.snippets[0].source, None);
    assert_eq!(texts(&a), ["s"]);
}

#[test]
fn blank_citation_session_records_nothing() {
    let (mut a, ids) = app_with(&["s"]);
    apply(&mut a, cite(ids[0], "  "));
    assert!(!tab(&a).is_dirty(), "a blank citation is no source");
    apply(&mut a, Action::StopEditing);
    assert_eq!(tab(&a).doc.snippets[0].source, None);
    assert_eq!(undo_len(&a), 0);
}

#[test]
fn typed_link_is_stored_only_while_valid() {
    let (mut a, ids) = app_with(&["s"]);
    let old = source(SourceType::Web, "Doe", Some("https://old.example"));
    apply(&mut a, set_source(ids[0], old));
    assert_eq!(undo_len(&a), 1);

    apply(&mut a, link(ids[0], "new.example"));
    assert_eq!(stored_link(&a).as_deref(), Some("https://new.example"));
    // Invalid while typing: the link from before the session stays.
    apply(&mut a, link(ids[0], "new.example/a b"));
    assert_eq!(stored_link(&a).as_deref(), Some("https://old.example"));
    assert!(!tab(&a).has_pending_snippet_edit());
    apply(&mut a, Action::StopEditing);
    assert_eq!(undo_len(&a), 1, "an invalid link is no change");

    apply(&mut a, link(ids[0], "javascript:alert(1)"));
    apply(&mut a, link(ids[0], "mailto:ann@example.org"));
    apply(&mut a, Action::StopEditing);
    assert_eq!(stored_link(&a).as_deref(), Some("mailto:ann@example.org"));
    assert_eq!(undo_len(&a), 2);
    apply(&mut a, Action::Undo);
    assert_eq!(stored_link(&a).as_deref(), Some("https://old.example"));
}

#[test]
fn undo_during_snippet_session_undoes_the_typing() {
    let (mut a, ids) = app_with(&["a"]);
    apply(&mut a, edit(ids[0], "ab"));
    apply(&mut a, Action::Undo);
    assert_eq!(texts(&a), ["a"]);
    assert!(tab(&a).snippet_edit.is_none());
    assert_eq!(tab(&a).history.redo.len(), 1);
    apply(&mut a, Action::Redo);
    assert_eq!(texts(&a), ["ab"]);
}

#[test]
fn closing_the_window_while_typing_a_citation_asks_to_save() {
    let (mut a, ids) = app_with(&["s"]);
    apply(&mut a, cite(ids[0], "Doe"));
    // The typing is in a tab that is not active.
    a.tabs.push(DocTab::new());
    a.active = 1;
    crate::file_ops::request_close_window(&mut a);
    assert!(a.modal.is_some(), "the unsaved tab is asked about");
    assert!(!a.close_confirmed());
    assert_eq!(a.tabs[0].history.undo.len(), 1, "the session was committed");
}
