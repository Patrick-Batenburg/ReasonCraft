//! Tests of the spell service: loading, the cache and suggestions.

use std::time::Instant;

use super::*;

/// Poll `service` until `lang` is loaded (or 20 s pass).
pub(crate) fn wait(service: &mut SpellService, lang: &str) -> Arc<Speller> {
    let start = Instant::now();
    loop {
        if let Some(s) = service.get(lang) {
            return s;
        }
        assert!(start.elapsed().as_secs() < 20, "{lang} never loaded");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn dictionary_loads_off_ui_thread() {
    let mut s = SpellService::new(&[]);
    s.delay = Some(Duration::from_millis(200));
    assert!(s.get("en").is_none(), "the first call only starts the load");
    assert!(s.is_loading("en") && s.busy());
    assert!(s.misspelled("en", "teh").is_none(), "no underlines yet");
    let speller = wait(&mut s, "en");
    assert!(!speller.check_word("recieve"));
    assert!(!s.busy());
    let found = s.misspelled("en", "I recieve").unwrap();
    assert_eq!(found.first(), Some(&(2..9)));
    assert_eq!(found.len(), 1);
}

#[test]
fn personal_and_ignored_words_are_accepted() {
    let mut s = SpellService::new(&["Reasoncraftish".to_owned()]);
    wait(&mut s, "en");
    assert!(s.misspelled("en", "Reasoncraftish").unwrap().is_empty());
    assert_eq!(s.misspelled("en", "teh blorb").unwrap().len(), 2);
    s.ignore_all("teh");
    assert_eq!(s.add_word(" blorb ").as_deref(), Some("blorb"));
    assert_eq!(s.add_word("blorb").as_deref(), Some("blorb"));
    assert_eq!(s.personal, ["Reasoncraftish", "blorb"], "kept once");
    assert!(s.misspelled("en", "teh blorb").unwrap().is_empty());
    assert!(s.add_word("a/b").is_none() && s.add_word(" ").is_none());
}

#[test]
fn unknown_language_fails_once() {
    let mut s = SpellService::new(&[]);
    assert!(s.get("fr").is_none());
    let start = Instant::now();
    while s.failure("fr").is_none() {
        assert!(start.elapsed().as_secs() < 20);
        s.poll_all();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(s.take_error().is_some());
    assert!(s.take_error().is_none(), "reported once");
    assert!(s.get("fr").is_none(), "not retried");
}

#[test]
fn typing_rechecks_one_line() {
    let mut s = SpellService::new(&[]);
    wait(&mut s, "en");
    let text = "The first line\nI recieve mail\nThe last line";
    let found = s.misspelled("en", text).unwrap();
    assert_eq!((found.len(), found[0].clone()), (1, 17..24));
    assert_eq!(s.checks_run(), 3, "each line checked once");
    // Typing in the middle line checks only that line again.
    let typed = "The first line\nI recieve maill\nThe last line";
    let found = s.misspelled("en", typed).unwrap();
    assert_eq!(found.as_slice(), [17..24, 25..30]);
    assert_eq!(s.checks_run(), 4);
}

#[test]
fn cache_keeps_the_last_256_texts() {
    let mut s = SpellService::new(&[]);
    wait(&mut s, "en");
    for i in 0..300 {
        s.misspelled("en", &format!("teh {i}"));
    }
    assert_eq!(s.checks_run(), 300);
    assert!(s.cache.len() <= CACHE_SIZE);
    s.misspelled("en", "teh 299");
    assert_eq!(s.checks_run(), 300, "recent text is cached");
    s.misspelled("en", "teh 0");
    assert_eq!(s.checks_run(), 301, "the oldest was dropped");
}
