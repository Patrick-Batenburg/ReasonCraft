//! UI tests for the app shell, driven through the accessibility tree.

use eframe::egui;
use egui_kittest::{Harness, kittest::Queryable};
use reasoncraft::ReasonCraftApp;
use reasoncraft::i18n::t;

fn harness() -> Harness<'static, ReasonCraftApp> {
    Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .with_max_steps(32)
        .build_ui_state(
            |ui, app: &mut ReasonCraftApp| app.show(ui),
            ReasonCraftApp::new_for_test(),
        )
}

#[test]
fn ribbon_add_reason_adds_card() {
    let mut h = harness();
    h.run();
    assert_eq!(h.state().tabs[0].doc.cards.len(), 0);
    let label = t("action.add_reason");
    assert_eq!(label, "Reason");
    h.get_by_label(&label).click();
    h.run();
    let tab = &h.state().tabs[0];
    assert_eq!(tab.doc.cards.len(), 1);
    assert_eq!(tab.history.undo.len(), 1);
}

#[test]
fn theme_switch_applies_dark_visuals() {
    let mut h = harness();
    h.run();
    // Start from light so the switch is observable.
    let view = t("ribbon.tab.view");
    h.get_by_label(&view).click();
    h.run();
    h.get_by_label(&t("theme.light")).click();
    h.run();
    assert!(!h.ctx.global_style().visuals.dark_mode);
    h.get_by_label(&t("theme.dark")).click();
    h.run();
    assert!(h.ctx.global_style().visuals.dark_mode);
    assert_eq!(h.state().theme, reasoncraft::theme::ThemeChoice::Dark);
}

#[test]
fn new_tab_button_opens_second_document() {
    let mut h = harness();
    h.run();
    assert_eq!(h.state().tabs.len(), 1);
    h.get_by_label(&t("tabs.new")).click();
    h.run();
    assert_eq!(h.state().tabs.len(), 2);
    assert_eq!(h.state().active, 1);
}

#[test]
fn missing_i18n_key_returns_key() {
    assert_eq!(t("no.such.key"), "no.such.key");
    assert_eq!(t("untitled"), "Untitled");
    assert_eq!(t("ribbon.coming_soon"), "Coming soon");
}
