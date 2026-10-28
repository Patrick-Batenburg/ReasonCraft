//! Templates start in the language of new documents (File > Settings),
//! like a blank map; example maps are written in the UI language of the
//! moment they are opened (they are read alongside the interface).

use reasoncraft::ReasonCraftApp;
use reasoncraft::actions::{Action, apply};
use reasoncraft_core::examples::{Example, Template, example, template};

use super::lock;

#[test]
fn template_language_follows_new_document_language() {
    let _lang = lock();
    let mut app = ReasonCraftApp::new_for_test();
    apply(&mut app, Action::SetUiLanguage(Some("nl".to_owned())));
    // Same as the interface (the default): Dutch.
    apply(&mut app, Action::OpenTemplate(Template::BlankReasoning));
    assert_eq!(app.active_tab().doc.meta.language, "nl");
    // English chosen for new documents: English, though the UI is Dutch.
    app.settings.default_doc_language = Some("en".to_owned());
    apply(&mut app, Action::OpenTemplate(Template::EssayPlan));
    let doc = &app.active_tab().doc;
    assert_eq!(doc.meta.language, "en");
    let want = template(Template::EssayPlan, "en");
    assert_eq!(doc.boxes[&doc.root].text, want.boxes[&want.root].text);
}

#[test]
fn example_language_follows_ui() {
    let _lang = lock();
    let mut app = ReasonCraftApp::new_for_test();
    // The language of new documents does not decide it.
    app.settings.default_doc_language = Some("en".to_owned());
    apply(&mut app, Action::SetUiLanguage(Some("nl".to_owned())));
    apply(&mut app, Action::OpenExample(Example::Grouping));
    assert_eq!(app.active_tab().doc.meta.language, "nl");
    // Back in English, the next one is English; the open one stays Dutch.
    apply(&mut app, Action::SetUiLanguage(Some("en".to_owned())));
    apply(&mut app, Action::OpenExample(Example::Reasoning));
    let doc = &app.active_tab().doc;
    assert_eq!(doc.meta.language, "en");
    let want = example(Example::Reasoning, "en");
    assert_eq!(doc.boxes[&doc.root].text, want.boxes[&want.root].text);
    assert_eq!(app.tabs[0].doc.meta.language, "nl");
}
