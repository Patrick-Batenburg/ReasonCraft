//! Example maps and templates are written in the UI language of the
//! moment they are opened.

use reasoncraft::ReasonCraftApp;
use reasoncraft::actions::{Action, apply};
use reasoncraft_core::examples::{Example, Template, example, template};

use super::lock;

#[test]
fn template_language_follows_ui() {
    let _lang = lock();
    let mut app = ReasonCraftApp::new_for_test();
    // The default language of new documents does not decide it.
    app.settings.default_doc_language = Some("en".to_owned());
    apply(&mut app, Action::SetUiLanguage(Some("nl".to_owned())));
    apply(&mut app, Action::OpenTemplate(Template::BlankReasoning));
    let doc = &app.active_tab().doc;
    assert_eq!(doc.meta.language, "nl");
    let want = template(Template::BlankReasoning, "nl");
    assert_eq!(doc.boxes[&doc.root].text, want.boxes[&want.root].text);
    apply(&mut app, Action::OpenExample(Example::Grouping));
    assert_eq!(app.active_tab().doc.meta.language, "nl");
    // Back in English, the next one is English; the open ones stay Dutch.
    apply(&mut app, Action::SetUiLanguage(Some("en".to_owned())));
    apply(&mut app, Action::OpenExample(Example::Reasoning));
    let doc = &app.active_tab().doc;
    assert_eq!(doc.meta.language, "en");
    let want = example(Example::Reasoning, "en");
    assert_eq!(doc.boxes[&doc.root].text, want.boxes[&want.root].text);
    assert_eq!(app.tabs[0].doc.meta.language, "nl");
}
