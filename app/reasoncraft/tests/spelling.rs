//! UI tests for spellcheck: underlines in editing fields in the document
//! language, and the spelling pass (F7) with Change, Change all, Ignore
//! and Add to dictionary.

use std::time::{Duration, Instant};

use eframe::egui::{self, Key, Shape};
use egui_kittest::{Harness, kittest::Queryable};
use reasoncraft::ReasonCraftApp;
use reasoncraft::actions::{Action, apply};
use reasoncraft::doc_tab::DocTab;
use reasoncraft::i18n::t;
use reasoncraft::settings::{Settings, TextTab};
use reasoncraft::spell::SpellService;
use reasoncraft_core::{CardKind, Document, EssaySlot, Item, NodeId, Parent};

type H = Harness<'static, ReasonCraftApp>;

fn harness_with(doc: Document, setup: impl FnOnce(&mut ReasonCraftApp)) -> H {
    let mut app = ReasonCraftApp::new_for_test();
    app.tabs[0] = DocTab::from_document(doc);
    setup(&mut app);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .with_max_steps(32)
        .build_ui_state(|ui, app: &mut ReasonCraftApp| app.show(ui), app);
    h.run();
    h
}

/// A document whose essay introduction reads `intro`, in `lang`.
fn essay_doc(intro: &str, lang: &str) -> Document {
    let mut d = Document::new("Claim");
    d.meta.language = lang.to_owned();
    d.set_essay_text(EssaySlot::Intro, intro).unwrap();
    d
}

/// Run frames until `lang`'s dictionary is loaded (at most 20 s).
fn wait_dictionary(h: &mut H, lang: &str) {
    let start = Instant::now();
    while h.state_mut().spell.get(lang).is_none() {
        assert!(
            start.elapsed() < Duration::from_secs(20),
            "{lang} never loaded"
        );
        h.step();
        std::thread::sleep(Duration::from_millis(5));
    }
    h.run();
}

/// The text of every underlined section painted last frame.
fn underlined(h: &H) -> Vec<String> {
    fn walk(shape: &Shape, out: &mut Vec<String>) {
        match shape {
            Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
            Shape::Text(t) => {
                let job = &t.galley.job;
                for s in &job.sections {
                    if s.format.underline.width > 0.0 {
                        out.push(job.text[s.byte_range.start.0..s.byte_range.end.0].to_owned());
                    }
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    for clipped in &h.output().shapes {
        walk(&clipped.shape, &mut out);
    }
    out
}

fn essay_tab(app: &mut ReasonCraftApp) {
    app.settings.text_tab = TextTab::Essay;
}

#[test]
fn essay_field_underlines_misspelling() {
    let mut h = harness_with(essay_doc("I recieve the mail", "en"), essay_tab);
    wait_dictionary(&mut h, "en");
    assert_eq!(underlined(&h), ["recieve"]);
}

#[test]
fn spellcheck_off_no_underlines() {
    let mut h = harness_with(essay_doc("I recieve the mail", "en"), |app| {
        essay_tab(app);
        app.settings.spellcheck = false;
    });
    for _ in 0..5 {
        h.step();
    }
    assert!(underlined(&h).is_empty());
    assert!(!h.state().spell.requested("en"), "no dictionary is loaded");
    h.key_press(Key::F7);
    h.run();
    assert!(h.state().tabs[0].spelling.is_none(), "F7 does nothing");
    // Turned back on: the underlines come back.
    apply(h.state_mut(), Action::SetSpellcheck(true));
    h.run();
    wait_dictionary(&mut h, "en");
    assert_eq!(underlined(&h), ["recieve"]);
}

#[test]
fn dutch_doc_uses_dutch_dictionary() {
    let text = "Dit is een fietsenstalling bij het station, met een fietz";
    let mut h = harness_with(essay_doc(text, "nl"), essay_tab);
    wait_dictionary(&mut h, "nl");
    assert_eq!(underlined(&h), ["fietz"]);
    assert!(!h.state().spell.requested("en"), "English is never loaded");
}

/// A claim reading `claim` with a reason under it for each of `reasons`.
fn map(claim: &str, reasons: &[&str]) -> (Document, Vec<NodeId>) {
    let mut d = Document::new(claim);
    let root = Parent::Box(d.root);
    let mut ids = vec![d.root];
    for r in reasons {
        ids.push(d.add_card(root, CardKind::Reason, r).unwrap().1);
    }
    (d, ids)
}

fn text(h: &H, node: NodeId) -> String {
    h.state().tabs[0].doc.boxes[&node].text.clone()
}

fn undo_len(h: &H) -> usize {
    h.state().tabs[0].history.undo.len()
}

/// The word the pass is on (`None` once it is done or while it waits).
fn current_word(h: &H) -> Option<String> {
    let pass = h.state().tabs[0].spelling.as_ref()?;
    pass.current.as_ref().map(|_| pass.word.clone())
}

fn open_pass(h: &mut H) {
    apply(h.state_mut(), Action::OpenSpelling);
    h.run();
    wait_dictionary(h, "en");
}

#[test]
fn spelling_pass_change_and_undo() {
    let (d, ids) = map("Cats are fine", &["I recieve teh mail"]);
    let mut h = harness_with(d, |app| app.spell.delay = Some(Duration::from_millis(300)));
    // Start at the reason: select it, then F7.
    apply(h.state_mut(), Action::Select(Some(Item::Box(ids[1]))));
    h.key_press(Key::F7);
    h.run();
    assert!(h.state().right_panel);
    assert!(h.query_by_label(&t("spell.loading")).is_some());
    wait_dictionary(&mut h, "en");
    assert_eq!(current_word(&h).as_deref(), Some("recieve"));
    assert_eq!(h.state().tabs[0].selection, Some(Item::Box(ids[1])));
    let pass = h.state().tabs[0].spelling.clone().unwrap();
    assert!(pass.suggestions.len() <= 6);
    assert!(pass.suggestions.contains(&"receive".to_owned()));
    h.state_mut().tabs[0].spelling.as_mut().unwrap().replacement = "receive".into();
    h.run();
    let before = undo_len(&h);
    h.get_by_label(&t("spell.change")).click();
    h.run();
    assert_eq!(text(&h, ids[1]), "I receive teh mail");
    assert_eq!(undo_len(&h), before + 1, "Change is one history step");
    assert_eq!(current_word(&h).as_deref(), Some("teh"));
    h.key_press_modifiers(egui::Modifiers::COMMAND, Key::Z);
    h.run();
    assert_eq!(text(&h, ids[1]), "I recieve teh mail", "undone");
    h.get_by_label(&t("spell.ignore")).click();
    h.run();
    // The claim (reached by wrapping around) has none: the pass ends.
    assert!(h.state().tabs[0].spelling.as_ref().unwrap().done);
    assert!(h.query_by_label(&t("spell.done")).is_some());
    h.get_by_label(&t("spell.close")).click();
    h.run();
    assert!(h.state().tabs[0].spelling.is_none());
}

#[test]
fn change_all_one_history_entry() {
    let (d, ids) = map("teh cat", &["teh dog and teh bird", "Teh end", "tehx"]);
    let mut h = harness_with(d, |_| {});
    open_pass(&mut h);
    assert_eq!(current_word(&h).as_deref(), Some("teh"));
    let before = undo_len(&h);
    apply(
        h.state_mut(),
        Action::SpellChangeAll {
            word: "teh".into(),
            with: "the".into(),
        },
    );
    h.run();
    assert_eq!(undo_len(&h), before + 1, "Change all is one history step");
    assert_eq!(text(&h, ids[0]), "the cat");
    assert_eq!(text(&h, ids[1]), "the dog and the bird");
    assert_eq!(text(&h, ids[2]), "Teh end", "match case");
    assert_eq!(text(&h, ids[3]), "tehx", "whole word");
    assert_eq!(current_word(&h).as_deref(), Some("Teh"), "on to the next");
}

#[test]
fn add_to_dictionary_persists_in_settings() {
    let (d, _) = map("Reasoncraftish rocks", &[]);
    let mut h = harness_with(d, |_| {});
    open_pass(&mut h);
    assert_eq!(current_word(&h).as_deref(), Some("Reasoncraftish"));
    h.get_by_label(&t("spell.add")).click();
    h.run();
    let settings = h.state().settings.clone();
    assert_eq!(settings.spell_words, ["Reasoncraftish"]);
    assert!(h.state().tabs[0].spelling.as_ref().unwrap().done);
    // Saved and restored, the word is still known.
    let json = serde_json::to_string(&settings).unwrap();
    let restored: Settings = serde_json::from_str(&json).unwrap();
    let mut service = SpellService::new(&restored.spell_words);
    let start = Instant::now();
    while service.misspelled("en", "x").is_none() {
        assert!(start.elapsed() < Duration::from_secs(20));
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        service
            .misspelled("en", "Reasoncraftish")
            .unwrap()
            .is_empty()
    );
    // A word the dictionary would misread is refused, with a note.
    apply(h.state_mut(), Action::SpellAddWord("bad/word".into()));
    assert_eq!(h.state().settings.spell_words, ["Reasoncraftish"]);
    assert_eq!(h.state().status_note, Some(t("spell.add_refused")));
}

#[test]
fn box_editor_and_snippet_underline_but_map_does_not() {
    let (mut d, _) = map("I recieve mail", &[]);
    d.add_snippet("A snippet with a mistaek");
    let mut h = harness_with(d, |_| {});
    wait_dictionary(&mut h, "en");
    assert!(
        underlined(&h).is_empty(),
        "the painted map has no underlines"
    );
    apply(h.state_mut(), Action::Edit);
    h.run();
    assert_eq!(underlined(&h), ["recieve"], "the box editor has");
    apply(h.state_mut(), Action::StopEditing);
    apply(h.state_mut(), Action::SetTextTab(TextTab::Snippets));
    h.run();
    assert_eq!(underlined(&h), ["mistaek"], "snippet text");
}
