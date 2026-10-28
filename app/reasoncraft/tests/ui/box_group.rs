//! UI tests for the Home ribbon's Box group: changing a card's kind or a
//! source's type, editing a box's link and opening it from its glyph.

use crate::common::{app_with_doc, harness};
use eframe::egui;
use egui_kittest::kittest::{NodeT, Queryable};
use reasoncraft::actions::FakeOpener;
use reasoncraft::canvas::links::link_label;
use reasoncraft::i18n::t;
use reasoncraft_core::{
    BoxKind, CardId, CardKind, CardStrength, Document, Item, NodeId, Parent, SourceType,
};

use crate::common::H;

/// A harness without the building panel (its items share names with the
/// ribbon's menus), with `opener` recording opened links.
fn harness_with(doc: Document, opener: &FakeOpener) -> H {
    let mut app = app_with_doc(doc);
    app.left_panel = false;
    app.link_opener = Box::new(opener.clone());
    harness(app)
}

/// A claim with a strong reason card and a web source under the claim.
fn doc() -> (Document, CardId, NodeId) {
    let mut d = Document::new("Claim");
    let root = d.root;
    let (c, _) = d
        .add_card(Parent::Box(root), CardKind::Reason, "p")
        .unwrap();
    d.set_card_strength(c, Some(CardStrength::Strong)).unwrap();
    let kind = BoxKind::Source {
        source_type: SourceType::Web,
    };
    let src = d.add_box(Parent::Box(root), kind, "s").unwrap();
    (d, c, src)
}

/// Click the lowest widget called `label` (a menu entry, below the ribbon
/// button of the same name).
fn click_lowest(h: &mut H, label: &str) {
    h.get_all_by_label(label)
        .max_by(|a, b| a.rect().top().total_cmp(&b.rect().top()))
        .expect(label)
        .click();
    h.run();
}

#[test]
fn set_card_kind_via_ribbon_keeps_strength() {
    let (d, c, _) = doc();
    let mut h = harness_with(d, &FakeOpener::default());
    h.state_mut().tabs[0].selection = Some(Item::Card(c));
    h.run();
    h.get_by_label(&t("action.box_type")).click();
    h.run();
    click_lowest(&mut h, &t("card_kind.objection"));
    let tab = &h.state().tabs[0];
    assert_eq!(tab.doc.cards[&c].kind, CardKind::Objection);
    assert_eq!(tab.doc.cards[&c].strength, Some(CardStrength::Strong));
    assert_eq!(tab.history.undo.len(), 1);
}

#[test]
fn set_source_type_via_ribbon() {
    let (d, _, src) = doc();
    let mut h = harness_with(d, &FakeOpener::default());
    h.state_mut().tabs[0].selection = Some(Item::Box(src));
    h.run();
    h.get_by_label(&t("action.box_type")).click();
    h.run();
    click_lowest(&mut h, &t("source.publication"));
    let tab = &h.state().tabs[0];
    assert_eq!(
        tab.doc.boxes[&src].kind,
        BoxKind::Source {
            source_type: SourceType::Publication
        }
    );
    assert_eq!(tab.history.undo.len(), 1);
}

#[test]
fn type_is_disabled_without_card_or_source() {
    let (d, _, _) = doc();
    let h = harness_with(d, &FakeOpener::default());
    // The claim is selected.
    let label = t("action.box_type");
    assert!(h.get_by_label(&label).accesskit_node().is_disabled());
}

#[test]
fn link_modal_saves_link_and_glyph_opens_it() {
    let (d, _, src) = doc();
    let opener = FakeOpener::default();
    let mut h = harness_with(d, &opener);
    h.state_mut().tabs[0].selection = Some(Item::Box(src));
    h.run();
    h.get_by_label(&t("action.link")).click();
    h.run();
    assert!(h.state().link_edit.is_some());

    // An invalid link stays in the editor with a message.
    h.get_by_label(&t("link.url"))
        .type_text("javascript:alert(1)");
    h.run();
    h.get_by_label(&t("link.save_link")).click();
    h.run();
    h.get_by_label(&t("link.invalid"));
    assert!(h.state().link_edit.is_some());
    assert_eq!(h.state().tabs[0].history.undo.len(), 0);

    // A bare domain is saved with https://.
    h.state_mut().link_edit.as_mut().unwrap().text = "example.org/page".into();
    h.run();
    h.get_by_label(&t("link.save_link")).click();
    h.run();
    assert!(h.state().link_edit.is_none());
    let url = "https://example.org/page";
    let tab = &h.state().tabs[0];
    assert_eq!(tab.doc.boxes[&src].link.as_deref(), Some(url));
    assert_eq!(tab.history.undo.len(), 1);

    // The glyph opens the link and leaves the selection alone.
    h.state_mut().tabs[0].selection = None;
    h.run();
    h.get_by_label(&link_label(url)).click();
    h.run();
    assert_eq!(*opener.opened.borrow(), vec![url.to_owned()]);
    assert_eq!(h.state().tabs[0].selection, None);
    assert_eq!(h.state().modal, None);

    // Remove link.
    h.state_mut().tabs[0].selection = Some(Item::Box(src));
    h.run();
    h.get_by_label(&t("action.link")).click();
    h.run();
    h.get_by_label(&t("link.remove")).click();
    h.run();
    assert_eq!(h.state().tabs[0].doc.boxes[&src].link, None);
    assert!(h.query_by_label(&link_label(url)).is_none());
}

#[test]
fn link_modal_esc_cancels() {
    let (d, _, src) = doc();
    let mut h = harness_with(d, &FakeOpener::default());
    h.state_mut().tabs[0].selection = Some(Item::Box(src));
    h.run();
    h.get_by_label(&t("action.link")).click();
    h.run();
    h.get_by_label(&t("link.url")).type_text("a.b");
    h.run();
    h.key_press(egui::Key::Escape);
    h.run();
    assert!(h.state().link_edit.is_none());
    let tab = &h.state().tabs[0];
    assert_eq!(tab.doc.boxes[&src].link, None);
    assert_eq!(tab.history.undo.len(), 0);
}

#[test]
fn failed_open_shows_error() {
    let (mut d, _, src) = doc();
    d.set_link(src, Some("https://a.b".into())).unwrap();
    let opener = FakeOpener::default();
    opener.fail.replace(Some("no handler".into()));
    let mut h = harness_with(d, &opener);
    h.get_by_label(&link_label("https://a.b")).click();
    h.run();
    h.get_by_label(&t("link.open_failed"));
    h.get_by_label(&t("modal.ok")).click();
    h.run();
    assert_eq!(h.state().modal, None);
}

/// A file may hold any link: clicking the glyph of one that is not a web or
/// e-mail link shows an error and never reaches the opener.
#[test]
fn glyph_click_never_opens_unsafe_stored_links() {
    let stored = [
        "file:///C:/Windows/System32/calc.exe",
        "javascript:alert(1)",
        r"\server\share\x",
        "JAVASCRIPT:x",
        "https://ok.example",
    ];
    for url in stored {
        let (mut d, _, src) = doc();
        d.boxes.get_mut(&src).unwrap().link = Some(url.to_owned());
        let opener = FakeOpener::default();
        let mut h = harness_with(d, &opener);
        h.get_by_label(&link_label(url)).click();
        h.run();
        let safe = url == "https://ok.example";
        if safe {
            assert_eq!(*opener.opened.borrow(), vec![url.to_owned()]);
            assert_eq!(h.state().modal, None);
        } else {
            assert!(opener.opened.borrow().is_empty(), "{url:?} was opened");
            assert!(h.state().modal.is_some(), "{url:?}: no error shown");
            assert_ne!(link_label(url), link_label("https://ok.example"));
        }
    }
}
