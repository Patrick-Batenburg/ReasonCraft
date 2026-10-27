//! The help and the Text panel's tabs in the UI language.

use eframe::egui::accesskit::Role;
use egui_kittest::{
    Harness,
    kittest::{NodeT, Queryable},
};
use reasoncraft::ReasonCraftApp;
use reasoncraft::a11y::in_group;
use reasoncraft::actions::Action;
use reasoncraft::i18n::t;

use super::{harness_with, lock};

/// The help shows the pages of the UI language, and switches with it.
#[test]
fn help_follows_the_ui_language() {
    use reasoncraft::help::{HelpPage, title};
    let _lang = lock();
    let mut h = harness_with(ReasonCraftApp::new_for_test());
    h.state_mut()
        .push(Action::OpenHelp(HelpPage::Topic("checks")));
    h.run();
    let shown = |h: &Harness<'static, ReasonCraftApp>, lang: &str| {
        h.query_all_by_label(&title(lang, "checks"))
            .any(|n| n.accesskit_node().role() == Role::Label)
    };
    assert!(shown(&h, "en") && !shown(&h, "nl"));
    h.state_mut()
        .push(Action::SetUiLanguage(Some("nl".to_owned())));
    h.run();
    assert!(shown(&h, "nl") && !shown(&h, "en"));
    // The tab and the ribbon's Help tab keep names of their own.
    let panel = in_group(&t("action.panel_right"), &t("text.tab.help"));
    h.get_by_role_and_label(Role::Tab, &panel);
    h.get_by_role_and_label(Role::Tab, &t("ribbon.tab.help"));
}

/// At the panel's default width the five Text panel tabs fit on one line
/// in Dutch too, the language with the longer labels.
#[test]
fn dutch_text_tabs_fit_on_one_line() {
    let _lang = lock();
    let mut h = harness_with(ReasonCraftApp::new_for_test());
    h.state_mut()
        .push(Action::SetUiLanguage(Some("nl".to_owned())));
    h.run();
    let keys = [
        "text.tab.outline",
        "text.tab.essay",
        "text.tab.check",
        "text.tab.snippets",
    ];
    let mut tops: Vec<f32> = keys
        .iter()
        .map(|k| h.get_by_role_and_label(Role::Tab, &t(k)).rect().top())
        .collect();
    let help = in_group(&t("action.panel_right"), &t("text.tab.help"));
    tops.push(h.get_by_role_and_label(Role::Tab, &help).rect().top());
    assert!(tops.iter().all(|y| *y == tops[0]), "{tops:?}");
}
