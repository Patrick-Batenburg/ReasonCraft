//! New documents start in the interface language until the user picks a
//! language for them (Settings → Language of new documents).

use eframe::egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use reasoncraft::ReasonCraftApp;
use reasoncraft::actions::Action;
use reasoncraft::i18n::t;
use reasoncraft::settings::Settings;

use super::{harness_with, lock};

fn dutch_os() -> Option<String> {
    Some("nl-NL".to_owned())
}

/// The language of the active document after Ctrl+N.
fn ctrl_n(h: &mut Harness<'static, ReasonCraftApp>) -> String {
    let before = h.state().tabs.len();
    h.key_press_modifiers(Modifiers::COMMAND, Key::N);
    h.run();
    assert_eq!(h.state().tabs.len(), before + 1, "Ctrl+N opens a tab");
    h.state().active_tab().doc.meta.language.clone()
}

#[test]
fn first_run_on_a_dutch_computer_makes_dutch_documents() {
    let _lang = lock();
    let app = ReasonCraftApp::new_for_test_with_locale(Settings::default(), dutch_os);
    assert_eq!(app.tabs[0].doc.meta.language, "nl", "the first tab");
    let mut h = harness_with(app);
    h.run();
    assert_eq!(ctrl_n(&mut h), "nl");
    // File → New → Blank map.
    h.get_by_label(&t("ribbon.tab.file")).click();
    h.run();
    h.get_by_label(&t("action.new")).click();
    h.run();
    h.get_by_label(&t("backstage.new.blank")).click();
    h.run();
    assert_eq!(h.state().tabs.len(), 3);
    assert_eq!(h.state().active_tab().doc.meta.language, "nl");
}

#[test]
fn a_chosen_language_wins_until_reset() {
    let _lang = lock();
    let settings = Settings {
        default_doc_language: Some("en".to_owned()),
        ..Settings::default()
    };
    let app = ReasonCraftApp::new_for_test_with_locale(settings, dutch_os);
    assert_eq!(app.tabs[0].doc.meta.language, "en", "the first tab");
    let mut h = harness_with(app);
    h.run();
    assert_eq!(ctrl_n(&mut h), "en");
    // Reset to defaults: new documents follow the (Dutch) interface again.
    h.state_mut().push(Action::ConfirmResetSettings);
    h.run();
    assert_eq!(h.state().settings.default_doc_language, None);
    assert_eq!(ctrl_n(&mut h), "nl");
}
