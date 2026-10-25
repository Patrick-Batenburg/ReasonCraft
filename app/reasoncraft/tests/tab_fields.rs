//! Text fields belong to their tab: a field's editing state (undo history
//! included) never carries over to the same field in another document.

use eframe::egui::{self, Key, Modifiers};
use egui_kittest::{Harness, kittest::Queryable};
use reasoncraft::ReasonCraftApp;
use reasoncraft::actions::Action;
use reasoncraft::app::RibbonTab;
use reasoncraft::doc_tab::DocTab;
use reasoncraft::i18n::t;
use reasoncraft::settings::TextTab;
use reasoncraft_core::{Document, EssaySlot};

type H = Harness<'static, ReasonCraftApp>;

/// Two tabs; the first is active and empty, the second has an
/// introduction and a title of its own.
fn two_tabs() -> H {
    let mut app = ReasonCraftApp::new_for_test();
    app.tabs[0] = DocTab::from_document(Document::new("Claim"));
    let mut b = Document::new("Claim");
    b.set_essay_text(EssaySlot::Intro, "Own intro").unwrap();
    b.meta.title = "Own title".to_owned();
    app.tabs.push(DocTab::from_document(b));
    app.settings.text_tab = TextTab::Essay;
    Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .with_max_steps(32)
        .build_ui_state(|ui, app: &mut ReasonCraftApp| app.show(ui), app)
}

/// Type `text` into the field labelled `label`, then leave it.
fn type_into(h: &mut H, label: &str, text: &str) {
    h.get_by_label(label).click();
    h.run();
    h.get_by_label(label).type_text(text);
    h.run();
    h.key_press(Key::Escape);
    h.run();
}

/// Focus the field labelled `label` and press Ctrl+Z in it.
fn undo_in(h: &mut H, label: &str) {
    h.get_by_label(label).click();
    h.run();
    h.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    h.run();
}

#[test]
fn essay_field_undo_stays_in_its_tab() {
    let mut h = two_tabs();
    h.run();
    let intro = t("essay.intro.field");
    type_into(&mut h, &intro, "Written in A");
    assert_eq!(
        h.state().tabs[0].doc.essay.text(EssaySlot::Intro),
        Some("Written in A")
    );

    h.state_mut().pending.push(Action::ActivateTab(1));
    h.run();
    // Nothing was typed in B's field: there is nothing to undo there.
    undo_in(&mut h, &intro);
    assert_eq!(
        h.state().tabs[1].doc.essay.text(EssaySlot::Intro),
        Some("Own intro")
    );
    assert_eq!(
        h.state().tabs[0].doc.essay.text(EssaySlot::Intro),
        Some("Written in A")
    );
}

#[test]
fn title_field_undo_stays_in_its_tab() {
    let mut h = two_tabs();
    h.state_mut().ribbon_tab = RibbonTab::File;
    h.run();
    h.get_by_label(&t("backstage.document")).click();
    h.run();
    let title = t("document.title_field");
    type_into(&mut h, &title, "Title A");
    assert_eq!(h.state().tabs[0].doc.meta.title, "Title A");

    h.state_mut().pending.push(Action::ActivateTab(1));
    h.run();
    undo_in(&mut h, &title);
    assert_eq!(h.state().tabs[1].doc.meta.title, "Own title");
    assert_eq!(h.state().tabs[0].doc.meta.title, "Title A");
}
