//! UI tests for the Text panel on the right.

use eframe::egui;
use egui_kittest::{
    Harness,
    kittest::{NodeT, Queryable},
};
use reasoncraft::i18n::t;
use reasoncraft::settings::{Settings, TextTab};
use reasoncraft::{ReasonCraftApp, actions::Action};

fn harness() -> Harness<'static, ReasonCraftApp> {
    Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .with_max_steps(32)
        .build_ui_state(
            |ui, app: &mut ReasonCraftApp| app.show(ui),
            ReasonCraftApp::new_for_test(),
        )
}

fn tab_selected(h: &Harness<'static, ReasonCraftApp>, key: &str) -> bool {
    h.get_by_role_and_label(egui::accesskit::Role::Tab, &t(key))
        .accesskit_node()
        .toggled()
        == Some(egui::accesskit::Toggled::True)
}

#[test]
fn text_panel_tabs_switch_and_persist() {
    let mut h = harness();
    h.run();
    for key in [
        "text.tab.outline",
        "text.tab.essay",
        "text.tab.check",
        "text.tab.snippets",
    ] {
        assert_ne!(t(key), key, "missing ftl entry");
    }
    assert_eq!(h.state().settings.text_tab, TextTab::Outline);
    assert!(tab_selected(&h, "text.tab.outline"));
    assert!(!tab_selected(&h, "text.tab.check"));

    h.get_by_role_and_label(egui::accesskit::Role::Tab, &t("text.tab.check"))
        .click();
    h.run();
    assert_eq!(h.state().settings.text_tab, TextTab::Check);
    assert!(tab_selected(&h, "text.tab.check"));
    assert!(!tab_selected(&h, "text.tab.outline"));

    // Essay shows its notice for this whole step.
    h.get_by_role_and_label(egui::accesskit::Role::Tab, &t("text.tab.essay"))
        .click();
    h.run();
    assert_eq!(h.state().settings.text_tab, TextTab::Essay);
    assert_ne!(t("text.essay.coming"), "text.essay.coming");
    h.get_by_label(&t("text.essay.coming"));

    // The tab is a setting: it survives a save and restore.
    h.state_mut()
        .pending
        .push(Action::SetTextTab(TextTab::Snippets));
    h.run();
    let json = serde_json::to_string(&h.state().settings).unwrap();
    let restored: Settings = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.text_tab, TextTab::Snippets);
    // Settings saved before the Text panel existed open on Outline.
    let old: Settings = serde_json::from_str(r#"{"theme":"Light","author":"A"}"#).unwrap();
    assert_eq!(old.text_tab, TextTab::Outline);
    assert_eq!(old.author, "A");
}
