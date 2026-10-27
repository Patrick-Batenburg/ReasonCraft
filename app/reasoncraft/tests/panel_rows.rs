//! The Essay and Check tabs of the Text panel lay out only the rows near
//! what they show, so a large map scrolls smoothly.

use eframe::egui::{self, accesskit::Role};
use egui_kittest::{
    Harness,
    kittest::{NodeT, Queryable},
};
use reasoncraft::ReasonCraftApp;
use reasoncraft::doc_tab::DocTab;
use reasoncraft::i18n::t;
use reasoncraft::settings::TextTab;
use reasoncraft_core::fixtures::sample_iot_like;
use reasoncraft_core::{Document, EssaySlot, NewLoose};

type H = Harness<'static, ReasonCraftApp>;

/// The most rows a tab may lay out in one frame.
const MAX_ROWS: usize = 60;

fn harness_with(doc: Document, tab: TextTab) -> H {
    let mut app = ReasonCraftApp::new_for_test();
    app.tabs[0] = DocTab::from_document(doc);
    app.settings.text_tab = tab;
    Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .with_max_steps(32)
        .build_ui_state(|ui, app: &mut ReasonCraftApp| app.show(ui), app)
}

/// Scroll the wheel by `dy` points over `at`.
fn wheel(h: &mut H, at: egui::Pos2, dy: f32) {
    h.event(egui::Event::PointerMoved(at));
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, dy),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
        modifiers: egui::Modifiers::default(),
    });
}

/// The labels of the list rows the panel shows.
fn rows(h: &H) -> Vec<String> {
    h.query_all_by_role(Role::ListItem)
        .filter_map(|n| n.accesskit_node().label())
        .collect()
}

/// The name of the claim's row in the introduction.
fn intro_claim(h: &H) -> String {
    let doc = &h.state().tabs[0].doc;
    reasoncraft::a11y::in_group(&t("essay.intro"), &doc.boxes[&doc.root].text)
}

#[test]
fn essay_tab_lays_out_only_visible_rows() {
    let doc = sample_iot_like(400);
    assert!(doc.boxes.len() >= 400);
    let mut h = harness_with(doc, TextTab::Essay);
    h.run();
    let laid_out = h.state().tabs[0].essay_heights.laid_out();
    assert!(laid_out > 5 && laid_out <= MAX_ROWS, "{laid_out} rows");
    let claim = intro_claim(&h);
    let first = rows(&h);
    assert!(first.contains(&claim), "{first:?}");
    let at = h
        .get_all_by_role_and_label(Role::ListItem, &claim)
        .next()
        .unwrap()
        .rect()
        .center();
    for _ in 0..10 {
        wheel(&mut h, at, -2000.0);
        h.step();
        let laid_out = h.state().tabs[0].essay_heights.laid_out();
        assert!(laid_out <= MAX_ROWS, "{laid_out} rows while scrolling");
    }
    h.run();
    let later = rows(&h);
    assert!(!later.is_empty());
    assert!(later.iter().all(|r| !first.contains(r)), "{later:?}");
}

#[test]
fn check_tab_virtualised() {
    let mut doc = Document::new("Claim");
    for i in 0..400 {
        doc.add_loose(NewLoose::Category, &format!("Parked {i}"));
    }
    let mut h = harness_with(doc, TextTab::Check);
    h.run();
    let laid_out = h.state().tabs[0].check_heights.laid_out();
    assert!(laid_out > 5 && laid_out <= MAX_ROWS, "{laid_out} rows");
    assert!(rows(&h).contains(&"Parked 0".to_owned()));
    let at = h
        .get_by_role_and_label(Role::ListItem, "Parked 0")
        .rect()
        .center();
    for _ in 0..10 {
        wheel(&mut h, at, -2000.0);
        h.step();
        let laid_out = h.state().tabs[0].check_heights.laid_out();
        assert!(laid_out <= MAX_ROWS, "{laid_out} rows while scrolling");
    }
    h.run();
    let shown = rows(&h);
    assert!(!shown.contains(&"Parked 0".to_owned()), "{shown:?}");
    assert!(shown.iter().any(|r| r.starts_with("Parked ")), "{shown:?}");
}

/// The paragraph being typed in keeps the keyboard focus when it scrolls
/// out of view: its row stays laid out.
#[test]
fn focused_paragraph_survives_scrolling_away() {
    let mut h = harness_with(sample_iot_like(400), TextTab::Essay);
    h.run();
    let first = rows(&h);
    let field = t("essay.intro.field");
    h.get_by_role_and_label(Role::MultilineTextInput, &field)
        .click();
    h.run();
    let claim = intro_claim(&h);
    let at = h
        .get_all_by_role_and_label(Role::ListItem, &claim)
        .next()
        .unwrap()
        .rect()
        .center();
    for _ in 0..10 {
        wheel(&mut h, at, -2000.0);
        h.step();
    }
    h.run();
    // The introduction (the claim's statement and the field) stays, with
    // its next rows (so Tab goes on); the other rows around it are gone.
    let later = rows(&h);
    assert!(later.contains(&claim), "{later:?}");
    let kept = later.iter().filter(|r| first.contains(r)).count();
    assert!(kept < later.len() && kept <= 3, "{later:?}");
    h.event(egui::Event::Text("Typed".into()));
    h.run();
    let doc = &h.state().tabs[0].doc;
    assert_eq!(doc.essay.text(EssaySlot::Intro), Some("Typed"));
}

/// A Check tab row with the keyboard focus keeps it when it scrolls out
/// of view: its row stays laid out.
#[test]
fn focused_check_row_survives_scrolling_away() {
    let mut doc = Document::new("Claim");
    for i in 0..400 {
        doc.add_loose(NewLoose::Category, &format!("Parked {i}"));
    }
    let mut h = harness_with(doc, TextTab::Check);
    h.run();
    let row = h.get_by_role_and_label(Role::ListItem, "Parked 0");
    let at = row.rect().center();
    row.focus();
    h.run();
    for _ in 0..10 {
        wheel(&mut h, at, -2000.0);
        h.step();
    }
    h.run();
    // Its neighbours stay too, so Tab goes on to the next row.
    let shown = rows(&h);
    assert!(shown.contains(&"Parked 1".to_owned()), "{shown:?}");
    assert!(!shown.contains(&"Parked 5".to_owned()), "scrolled away");
    let row = h.get_by_role_and_label(Role::ListItem, "Parked 0");
    assert!(row.is_focused(), "keeps the focus");
}
