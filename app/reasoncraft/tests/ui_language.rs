//! UI tests for the interface language (View → Language, the settings page).
//!
//! The UI language is one setting for the whole process. Tests that switch
//! it live in this test binary, apart from every other test, and take turns
//! through [`lock`], which puts English back afterwards.

use std::sync::{Mutex, MutexGuard, PoisonError};

use eframe::egui::accesskit::{Role, Toggled};
use eframe::egui::{self, Key, Modifiers, epaint};
use egui_kittest::{
    Harness,
    kittest::{NodeT, Queryable},
};
use reasoncraft::actions::Action;
use reasoncraft::canvas::input::{shortcut_for, shortcut_text};
use reasoncraft::doc_tab::DocTab;
use reasoncraft::i18n::{self, FluentArgs, t, t_args, t_lang, ui_language};
use reasoncraft::settings::Settings;
use reasoncraft::{ReasonCraftApp, commands};
use reasoncraft_core::{BoxKind, CardKind, CardStrength, Document, NewLoose, Parent, SourceType};

static LANGUAGE: Mutex<()> = Mutex::new(());

/// The right to switch the UI language; English again once dropped.
struct LanguageLock(#[allow(dead_code)] MutexGuard<'static, ()>);

impl Drop for LanguageLock {
    fn drop(&mut self) {
        i18n::set_ui_language("en");
    }
}

fn lock() -> LanguageLock {
    let guard = LANGUAGE.lock().unwrap_or_else(PoisonError::into_inner);
    i18n::set_ui_language("en");
    LanguageLock(guard)
}

fn harness_with(app: ReasonCraftApp) -> Harness<'static, ReasonCraftApp> {
    Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .with_max_steps(32)
        .build_ui_state(|ui, app: &mut ReasonCraftApp| app.show(ui), app)
}

fn click(h: &mut Harness<'static, ReasonCraftApp>, label: &str) {
    h.get_by_label(label).click();
    h.run();
}

/// Alt+Q, then type `query` into the search box.
fn search_for(h: &mut Harness<'static, ReasonCraftApp>, query: &str) {
    h.key_press_modifiers(Modifiers::ALT, Key::Q);
    h.run();
    let mut args = FluentArgs::new();
    let keys = shortcut_for(&Action::OpenCommandSearch).map(shortcut_text);
    args.set("keys", keys.unwrap());
    h.get_by_label(&t_args("titlebar.search", &args))
        .type_text(query);
    h.run();
}

#[test]
fn switching_language_updates_ribbon_and_search() {
    let _lang = lock();
    let mut h = harness_with(ReasonCraftApp::new_for_test());
    h.run();
    let home_en = t_lang("en", "ribbon.tab.home");
    let home_nl = t_lang("nl", "ribbon.tab.home");
    assert_eq!(home_nl, "Start");
    h.get_by_label(&home_en);

    // View → Language → Nederlands, in its own language.
    click(&mut h, &t("ribbon.tab.view"));
    click(&mut h, &t("action.ui_language"));
    let dutch = t(commands::get("view.language.nl").label);
    assert_eq!(dutch, "Nederlands");
    click(&mut h, &dutch);
    assert_eq!(h.state().settings.ui_language.as_deref(), Some("nl"));
    assert_eq!(ui_language(), "nl");
    h.get_by_label(&home_nl);
    assert!(h.query_by_label(&home_en).is_none());
    // No document change, so nothing to undo.
    assert!(h.state().tabs[0].history.undo.is_empty());

    // Search commands finds the Home commands by the tab's Dutch name.
    search_for(&mut h, "Start");
    let home_cmd = commands::all()
        .iter()
        .find(|c| c.tab() == "ribbon.tab.home")
        .unwrap();
    let label = t(home_cmd.label);
    assert!(
        h.query_by_role_and_label(Role::ListItem, &label).is_some(),
        "\"Start\" lists {label}"
    );
    assert!(
        commands::search("Start", 500)
            .iter()
            .any(|c| c.tab() == "ribbon.tab.home")
    );
    h.key_press(Key::Escape);
    h.run();

    // Back to English: the next frame shows English again.
    click(&mut h, &t("ribbon.tab.view"));
    click(&mut h, &t("action.ui_language"));
    click(&mut h, &t(commands::get("view.language.en").label));
    assert_eq!(ui_language(), "en");
    h.get_by_label(&home_en);
}

#[test]
fn system_language_follows_the_os() {
    let _lang = lock();
    let mut app = ReasonCraftApp::new_for_test();
    app.os_locale = || Some("nl-BE".to_owned());
    let mut h = harness_with(app);
    h.state_mut()
        .push(Action::SetUiLanguage(Some("en".to_owned())));
    h.run();
    assert_eq!(ui_language(), "en");
    h.state_mut().push(Action::SetUiLanguage(None));
    h.run();
    assert_eq!(h.state().settings.ui_language, None);
    assert_eq!(ui_language(), "nl");
    // A language this build does not have is refused.
    h.state_mut()
        .push(Action::SetUiLanguage(Some("fr".to_owned())));
    h.run();
    assert_eq!(h.state().settings.ui_language, None);
    assert_eq!(ui_language(), "nl");
}

#[test]
fn settings_page_picks_the_language() {
    let _lang = lock();
    let mut h = harness_with(ReasonCraftApp::new_for_test());
    h.run();
    click(&mut h, &t("ribbon.tab.file"));
    click(&mut h, &t("backstage.settings"));
    click(&mut h, &t("ui_language.nl"));
    assert_eq!(h.state().settings.ui_language.as_deref(), Some("nl"));
    assert_eq!(ui_language(), "nl");
    // The page itself now reads in Dutch (falling back to English per key).
    h.get_by_label(&t_lang("nl", "settings.ui_language"));
    // "System" also names a theme further down the page.
    h.query_all_by_label(&t("ui_language.system"))
        .next()
        .unwrap()
        .click();
    h.run();
    assert_eq!(h.state().settings.ui_language, None);
    // Tests never read the OS locale: the test app's OS has none.
    assert_eq!(ui_language(), "en");
}

#[test]
fn ui_language_setting_defaults_to_the_os_and_survives_unknown_values() {
    assert_eq!(Settings::default().ui_language, None);
    let s: Settings = serde_json::from_str(r#"{"author":"Ann"}"#).unwrap();
    assert_eq!(s.ui_language, None);
    let s: Settings = serde_json::from_str(r#"{"ui_language":"nl","author":"Ann"}"#).unwrap();
    assert_eq!(s.ui_language.as_deref(), Some("nl"));
    assert_eq!(s.author, "Ann");
}

/// The texts painted in the last frame.
fn painted_texts(h: &Harness<'static, ReasonCraftApp>) -> Vec<String> {
    fn walk(s: &epaint::Shape, out: &mut Vec<String>) {
        match s {
            epaint::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
            epaint::Shape::Text(t) => out.push(t.galley.job.text.clone()),
            _ => {}
        }
    }
    let mut out = Vec::new();
    for clipped in &h.output().shapes {
        walk(&clipped.shape, &mut out);
    }
    out
}

/// A map in `lang`: a strong reason with a law under it, and a loose
/// premise in the parking area.
fn rated_map(lang: &str) -> ReasonCraftApp {
    let mut doc = Document::new("Cities should ban cars");
    doc.meta.language = lang.to_owned();
    let (card, premise) = doc
        .add_card(Parent::Box(doc.root), CardKind::Reason, "Cleaner air")
        .unwrap();
    doc.set_card_strength(card, Some(CardStrength::Strong))
        .unwrap();
    let law = BoxKind::Source {
        source_type: SourceType::Law,
    };
    doc.add_box(Parent::Box(premise), law, "Clean air act")
        .unwrap();
    doc.add_loose(NewLoose::Card(CardKind::Reason), "Loose");
    let mut app = ReasonCraftApp::new_for_test();
    app.tabs[0] = DocTab::from_document(doc);
    // No panels: the Outline would name the source in the UI language.
    app.left_panel = false;
    app.right_panel = false;
    app
}

#[test]
fn map_labels_follow_document_language_not_ui() {
    let _lang = lock();
    let words = ["rating.strong.supports", "source.law", "parking.title"];
    // UI English, document Dutch: the map speaks Dutch.
    let mut h = harness_with(rated_map("nl-NL"));
    h.run();
    h.state_mut().push(Action::JumpParking);
    h.run();
    h.state_mut().push(Action::ZoomFit);
    h.run();
    let texts = painted_texts(&h);
    for key in words {
        let (nl, en) = (t_lang("nl", key), t_lang("en", key));
        assert_ne!(nl, en, "{key} has a Dutch text");
        assert!(texts.contains(&nl), "{nl:?} in {texts:?}");
        assert!(!texts.contains(&en), "{en:?} in {texts:?}");
    }

    // UI Dutch, document English: the map speaks English.
    i18n::set_ui_language("nl");
    let mut h = harness_with(rated_map("en"));
    h.run();
    h.state_mut().push(Action::ZoomFit);
    h.run();
    let texts = painted_texts(&h);
    for key in words {
        let en = t_lang("en", key);
        assert!(texts.contains(&en), "{en:?} in {texts:?}");
        assert!(!texts.contains(&t_lang("nl", key)), "{key} in {texts:?}");
    }
}

#[test]
fn switching_language_clears_the_status_note() {
    let _lang = lock();
    let mut h = harness_with(ReasonCraftApp::new_for_test());
    h.state_mut().status_note = Some("Saved".to_owned());
    h.state_mut()
        .push(Action::SetUiLanguage(Some("nl".to_owned())));
    h.run();
    assert_eq!(h.state().status_note, None);
}

/// The one node labelled `label` that shows as selected.
fn selected_node<'h>(
    h: &'h Harness<'static, ReasonCraftApp>,
    label: &'h str,
) -> egui_kittest::Node<'h> {
    let mut on = h
        .query_all_by_label(label)
        .filter(|n| n.accesskit_node().toggled() == Some(Toggled::True));
    let node = on
        .next()
        .unwrap_or_else(|| panic!("{label:?} is not selected"));
    assert!(on.next().is_none(), "one {label:?} selected");
    node
}

#[test]
fn choosing_the_shown_language_again_changes_nothing() {
    let _lang = lock();
    for (stored, choice) in [("nl", "view.language.nl"), ("xx", "view.language.en")] {
        let mut app = ReasonCraftApp::new_for_test();
        app.settings.ui_language = Some(stored.to_owned());
        // Dutch documents: their language pickers do not select "English".
        app.tabs[0].doc.meta.language = "nl".to_owned();
        app.settings.default_doc_language = "nl".to_owned();
        let mut h = harness_with(app);
        h.run();
        h.state_mut().status_note = Some("Saved".to_owned());
        let label = t(commands::get(choice).label);
        // View → Language: the language shown is selected; choosing it
        // again pushes nothing (the note stays, the setting is kept).
        click(&mut h, &t("ribbon.tab.view"));
        click(&mut h, &t("action.ui_language"));
        selected_node(&h, &label).click();
        h.run();
        let state = h.state();
        assert_eq!(state.settings.ui_language.as_deref(), Some(stored));
        assert_eq!(state.status_note.as_deref(), Some("Saved"), "{stored}");
        // The settings page shows the same choice.
        click(&mut h, &t("ribbon.tab.file"));
        click(&mut h, &t("backstage.settings"));
        selected_node(&h, &label).click();
        h.run();
        assert_eq!(h.state().settings.ui_language.as_deref(), Some(stored));
    }
}
