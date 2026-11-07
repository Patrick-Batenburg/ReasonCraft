use reasoncraft_core::{CardKind, CardStrength, Document, Item, Parent};

use super::FakeOpener;
use crate::actions::{Action, apply, is_enabled};
use crate::app::ReasonCraftApp;
use crate::doc_tab::DocTab;
use crate::file_ops::Modal;
use crate::i18n::t;

/// An app showing a claim with one strong reason card; returns the card's id.
fn app_with_card() -> (ReasonCraftApp, reasoncraft_core::CardId) {
    let mut d = Document::new("Claim");
    let (c, _) = d
        .add_card(Parent::Box(d.root), CardKind::Reason, "p")
        .unwrap();
    d.set_card_strength(c, Some(CardStrength::Strong)).unwrap();
    let mut a = ReasonCraftApp::new_for_test();
    a.tabs[0] = DocTab::from_document(d);
    (a, c)
}

#[test]
fn set_card_kind_keeps_strength_and_skips_no_ops() {
    let (mut a, c) = app_with_card();
    let premise = a.tabs[0].doc.cards[&c].premises[0];
    a.tabs[0].selection = Some(Item::Box(premise));
    let to_objection = Action::SetCardKind(CardKind::Objection);
    assert!(is_enabled(&a.tabs[0], &to_objection), "via its premise");
    apply(&mut a, to_objection);
    let card = &a.tabs[0].doc.cards[&c];
    assert_eq!(card.kind, CardKind::Objection);
    assert_eq!(card.strength, Some(CardStrength::Strong));
    assert_eq!(a.tabs[0].history.undo.len(), 1);
    // Same kind again: no history step.
    apply(&mut a, Action::SetCardKind(CardKind::Objection));
    assert_eq!(a.tabs[0].history.undo.len(), 1);
    // The claim is no card.
    a.tabs[0].selection = Some(Item::Box(a.tabs[0].doc.root));
    assert!(!is_enabled(
        &a.tabs[0],
        &Action::SetCardKind(CardKind::Reason)
    ));
}

#[test]
fn edit_link_opens_editor_and_set_link_saves_once() {
    let (mut a, _) = app_with_card();
    let root = a.tabs[0].doc.root;
    a.tabs[0].selection = None;
    assert!(!is_enabled(&a.tabs[0], &Action::EditLink));
    a.tabs[0].selection = Some(Item::Box(root));
    apply(&mut a, Action::EditLink);
    let edit = a.link_edit.as_ref().expect("editor open");
    assert_eq!((edit.node, edit.text.as_str()), (root, ""));

    apply(&mut a, Action::SetLink(Some("https://a.b".into())));
    assert!(a.link_edit.is_none());
    let tab = &a.tabs[0];
    assert_eq!(tab.doc.boxes[&root].link.as_deref(), Some("https://a.b"));
    assert_eq!(tab.history.undo.len(), 1);

    // Reopening shows the link; saving it unchanged records nothing.
    apply(&mut a, Action::EditLink);
    assert_eq!(a.link_edit.as_ref().unwrap().text, "https://a.b");
    apply(&mut a, Action::SetLink(Some("https://a.b".into())));
    assert_eq!(a.tabs[0].history.undo.len(), 1);

    // Cancel (Esc) closes the editor without a change.
    apply(&mut a, Action::EditLink);
    apply(&mut a, Action::CloseLinkEdit);
    assert!(a.link_edit.is_none());

    apply(&mut a, Action::SetLink(None));
    assert_eq!(a.tabs[0].doc.boxes[&root].link, None);
    assert_eq!(a.tabs[0].history.undo.len(), 2);
}

#[test]
fn open_link_uses_stored_url() {
    let (mut a, _) = app_with_card();
    let opener = FakeOpener::default();
    a.link_opener = Box::new(opener.clone());
    let root = a.tabs[0].doc.root;
    let open = Action::OpenLink(root);
    assert!(!is_enabled(&a.tabs[0], &open), "no link yet");
    apply(&mut a, open.clone());
    assert!(opener.opened.borrow().is_empty());

    a.tabs[0].selection = Some(Item::Box(root));
    apply(&mut a, Action::SetLink(Some("mailto:x@y.z".into())));
    assert!(is_enabled(&a.tabs[0], &open));
    apply(&mut a, open.clone());
    assert_eq!(*opener.opened.borrow(), vec!["mailto:x@y.z".to_owned()]);
    assert_eq!(a.modal, None);
    assert_eq!(a.tabs[0].history.undo.len(), 1, "opening is no edit");

    // A failure shows an error.
    opener.fail.replace(Some("no browser".into()));
    apply(&mut a, open);
    match &a.modal {
        Some(Modal::Error { title, message }) => {
            assert_eq!(title, &t("link.open_failed"));
            assert!(message.contains("mailto:x@y.z"), "{message}");
            assert!(message.contains("no browser"), "{message}");
        }
        other => panic!("expected an error, got {other:?}"),
    }
}

/// Stored links that are not web or e-mail links (as a hand-edited file
/// may hold) never reach the opener; only the https one does.
const STORED: [&str; 6] = [
    "file:///C:/Windows/System32/calc.exe",
    "javascript:alert(1)",
    r"\server\share\x",
    "JAVASCRIPT:x",
    "https://a\u{202E}b.example/exe.txt",
    "https://ok.example",
];

#[test]
fn open_link_refuses_unsafe_stored_links() {
    for url in STORED {
        let (mut a, _) = app_with_card();
        let opener = FakeOpener::default();
        a.link_opener = Box::new(opener.clone());
        let root = a.tabs[0].doc.root;
        // Written straight into the document, as opening a file would.
        a.tabs[0].doc.boxes.get_mut(&root).unwrap().link = Some(url.to_owned());
        let open = Action::OpenLink(root);
        let safe = url == "https://ok.example";
        assert_eq!(is_enabled(&a.tabs[0], &open), safe, "{url:?}");
        apply(&mut a, open);
        if safe {
            assert_eq!(*opener.opened.borrow(), vec![url.to_owned()]);
            assert_eq!(a.modal, None);
        } else {
            assert!(opener.opened.borrow().is_empty(), "{url:?} was opened");
            match &a.modal {
                Some(Modal::Error { title, .. }) => assert_eq!(title, &t("link.open_failed")),
                other => panic!("{url:?}: expected an error, got {other:?}"),
            }
        }
        assert!(a.tabs[0].history.undo.is_empty());
    }
}

#[test]
fn set_link_refuses_invalid_links() {
    let (mut a, _) = app_with_card();
    let root = a.tabs[0].doc.root;
    a.tabs[0].selection = Some(Item::Box(root));
    for bad in [
        "file:///C:/x.exe",
        r"\server\share",
        "javascript:x",
        "a\u{202E}.b/x",
    ] {
        let set = Action::SetLink(Some(bad.to_owned()));
        assert!(!is_enabled(&a.tabs[0], &set), "{bad:?}");
        apply(&mut a, set);
        assert_eq!(a.tabs[0].doc.boxes[&root].link, None, "{bad:?}");
        assert!(a.tabs[0].history.undo.is_empty(), "{bad:?}");
    }
    // A bare domain is normalized on the way in.
    apply(&mut a, Action::SetLink(Some("a.b/c".into())));
    let link = a.tabs[0].doc.boxes[&root].link.clone();
    assert_eq!(link.as_deref(), Some("https://a.b/c"));
}

/// On Windows a missing file or folder is reported before the Shell is
/// asked (whose own error message would wait for the user, on the UI
/// thread if it ran there). Nothing is opened: the check comes first.
#[cfg(windows)]
#[test]
fn system_opener_reports_a_missing_path_without_opening() {
    use super::{LinkOpener, SystemOpener};
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("gone").join("print.pdf");
    let err = SystemOpener.open(&missing.to_string_lossy());
    assert!(err.is_err(), "{err:?}");
}
