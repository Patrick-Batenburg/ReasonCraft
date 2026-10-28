//! UI tests for spellcheck: underlines in editing fields in the document
//! language, and the spelling pass (F7) with Change, Change all, Ignore
//! and Add to dictionary.

use crate::common::{app_with_doc, harness};
use std::time::{Duration, Instant};

use eframe::egui::{self, Key, Shape};
use egui_kittest::kittest::Queryable;
use reasoncraft::ReasonCraftApp;
use reasoncraft::actions::{Action, apply};
use reasoncraft::i18n::t;
use reasoncraft::jobs::Hold;
use reasoncraft::settings::{Settings, TextTab};
use reasoncraft::spell::SpellService;
use reasoncraft_core::{CardKind, Document, EssaySlot, Item, NodeId, Parent};

use crate::common::H;

fn harness_with(doc: Document, setup: impl FnOnce(&mut ReasonCraftApp)) -> H {
    let mut app = app_with_doc(doc);
    setup(&mut app);
    harness(app)
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

/// Run frames until the pass has its suggestions (at most 20 s).
fn wait_suggestions(h: &mut H) -> Vec<String> {
    let start = Instant::now();
    loop {
        let pass = h.state().tabs[0].spelling.as_ref().expect("a pass runs");
        if let Some(found) = &pass.suggestions {
            return found.clone();
        }
        assert!(start.elapsed() < Duration::from_secs(20), "none came");
        h.step();
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn open_pass(h: &mut H) {
    apply(h.state_mut(), Action::OpenSpelling);
    h.run();
    wait_dictionary(h, "en");
}

#[test]
fn spelling_pass_change_and_undo() {
    let (d, ids) = map("Cats are fine", &["I recieve teh mail"]);
    let hold = Hold::default();
    let mut h = harness_with(d, |app| app.spell.hold = Some(hold.clone()));
    // Start at the reason: select it, then F7.
    apply(h.state_mut(), Action::Select(Some(Item::Box(ids[1]))));
    h.key_press(Key::F7);
    h.run();
    assert!(h.state().right_panel);
    assert!(h.query_by_label(&t("spell.loading")).is_some());
    hold.release();
    wait_dictionary(&mut h, "en");
    assert_eq!(current_word(&h).as_deref(), Some("recieve"));
    assert_eq!(h.state().tabs[0].selection, Some(Item::Box(ids[1])));
    let found = wait_suggestions(&mut h);
    assert!(found.len() <= 6);
    assert!(found.contains(&"receive".to_owned()));
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
    h.get_by_label(&t("spell.close_name")).click();
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
    // A word the dictionary would misread is refused.
    assert!(h.state_mut().spell.add_word("bad/word").is_none());
    assert!(reasoncraft_spell::personal_word("bad/word").is_none());
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

#[test]
fn pass_looks_again_after_the_document_changes() {
    let (d, ids) = map("Cats are fine", &["I recieve teh mail"]);
    let mut h = harness_with(d, |_| {});
    apply(h.state_mut(), Action::Select(Some(Item::Box(ids[1]))));
    open_pass(&mut h);
    h.state_mut().tabs[0].spelling.as_mut().unwrap().replacement = "get".into();
    h.run();
    h.get_by_label(&t("spell.change")).click();
    h.run();
    assert_eq!(text(&h, ids[1]), "I get teh mail");
    assert_eq!(current_word(&h).as_deref(), Some("teh"));
    // Undo puts the longer word back: the old range of "teh" now reads "iev".
    h.key_press_modifiers(egui::Modifiers::COMMAND, Key::Z);
    h.run();
    assert_eq!(text(&h, ids[1]), "I recieve teh mail");
    let pass = h.state().tabs[0].spelling.clone().unwrap();
    assert_eq!(pass.word, "teh", "found again");
    assert_eq!(pass.current.unwrap().range, 10..13);
    // An action naming another word than the one on show does nothing.
    apply(h.state_mut(), Action::SpellAddWord("iev".into()));
    assert!(h.state().settings.spell_words.is_empty());
    h.get_by_label(&t("spell.add")).click();
    h.run();
    assert_eq!(h.state().settings.spell_words, ["teh"]);
    // A change made before the panel's next frame: the action is refused
    // and the pass looks again instead.
    apply(h.state_mut(), Action::CloseSpelling);
    apply(h.state_mut(), Action::OpenSpelling);
    assert_eq!(current_word(&h).as_deref(), Some("recieve"));
    let tab = &mut h.state_mut().tabs[0];
    tab.doc.set_text(ids[1], "Now recieve").unwrap();
    apply(h.state_mut(), Action::SpellAddWord("recieve".into()));
    assert_eq!(h.state().settings.spell_words, ["teh"], "nothing added");
    let pass = h.state().tabs[0].spelling.clone().unwrap();
    assert_eq!(pass.word, "recieve");
    assert_eq!(pass.current.unwrap().range, 4..11, "found where it is now");
}

#[test]
fn change_to_the_same_word_adds_no_history() {
    let (d, ids) = map("teh cat", &["teh dog"]);
    let mut h = harness_with(d, |_| {});
    open_pass(&mut h);
    let hit = h.state().tabs[0].spelling.clone().unwrap().current.unwrap();
    let before = undo_len(&h);
    let with = "teh".to_owned();
    apply(h.state_mut(), Action::SpellChange { hit, with });
    assert_eq!(undo_len(&h), before, "Change to the same word");
    assert_eq!(text(&h, ids[0]), "teh cat");
    assert_eq!(current_word(&h).as_deref(), Some("teh"), "on to the reason");
    let (word, with) = ("teh".to_owned(), "teh".to_owned());
    apply(h.state_mut(), Action::SpellChangeAll { word, with });
    assert_eq!(undo_len(&h), before, "Change all to the same word");
    assert!(h.state().tabs[0].spelling.as_ref().unwrap().done);
}

#[test]
fn suggestions_never_block_ui() {
    let (d, _) = map("I recieve mail", &[]);
    let mut h = harness_with(d, |_| {});
    wait_dictionary(&mut h, "en");
    // Hold every suggestion job back: the pass shows the word at once.
    let hold = Hold::default();
    h.state_mut().spell.hold = Some(hold.clone());
    // (Opening the pass waited for its suggestions if this took until the
    // hold gives up.)
    let start = Instant::now();
    apply(h.state_mut(), Action::OpenSpelling);
    assert!(start.elapsed() < Hold::MAX_WAIT / 2, "no wait");
    h.run();
    assert_eq!(current_word(&h).as_deref(), Some("recieve"));
    let pass = h.state().tabs[0].spelling.clone().unwrap();
    assert_eq!(pass.suggestions, None, "still being worked out");
    assert_eq!(pass.replacement, "recieve");
    assert!(
        h.query_by_label(&t("spell.suggesting")).is_some(),
        "shows …"
    );
    assert!(h.state_mut().spell.suggest_async("en", "recieve").is_none());
    hold.release();
    let found = wait_suggestions(&mut h);
    h.run();
    let pass = h.state().tabs[0].spelling.clone().unwrap();
    assert!(found.contains(&"receive".to_owned()), "{found:?}");
    assert!(found.len() <= 6);
    assert_eq!(pass.replacement, found[0], "the first one is picked");
    assert!(h.query_by_label(&t("spell.suggesting")).is_none());
    assert_eq!(
        h.state_mut().spell.suggest_async("en", "recieve"),
        Some(found),
        "kept"
    );
}

#[test]
fn abandoned_suggestions_leave_the_service_idle() {
    let (d, _) = map("I recieve mail", &[]);
    let mut h = harness_with(d, |_| {});
    wait_dictionary(&mut h, "en");
    let hold = Hold::default();
    h.state_mut().spell.hold = Some(hold.clone());
    apply(h.state_mut(), Action::OpenSpelling);
    assert_eq!(current_word(&h).as_deref(), Some("recieve"));
    // Ignored before its suggestions arrive: nobody asks for them again.
    apply(h.state_mut(), Action::SpellIgnore);
    assert!(h.state().tabs[0].spelling.as_ref().unwrap().done);
    assert!(h.state().spell.busy());
    hold.release();
    let start = Instant::now();
    while h.state().spell.busy() {
        assert!(start.elapsed() < Duration::from_secs(20), "busy for ever");
        h.step();
        std::thread::sleep(Duration::from_millis(5));
    }
}
