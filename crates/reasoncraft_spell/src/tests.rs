use super::*;

fn flagged<'a>(s: &Speller, text: &'a str) -> Vec<&'a str> {
    s.misspelled(text).into_iter().map(|r| &text[r]).collect()
}

#[test]
fn english_basics() {
    let s = Speller::load("en").unwrap();
    assert!(flagged(&s, "The quick brown fox jumps").is_empty());
    assert_eq!(flagged(&s, "I recieve teh mail"), ["recieve", "teh"]);
    assert!(s.suggest("recieve", 5).contains(&"receive".to_string()));
}

#[test]
fn dutch_basics() {
    let s = Speller::load("nl").unwrap();
    assert!(flagged(&s, "Dit is een fietsenstalling bij het station").is_empty());
    for w in ["tijdsgebonden", "ijsje", "e-mail", "zo’n"] {
        assert!(flagged(&s, w).is_empty(), "{w} should be accepted");
    }
    assert_eq!(flagged(&s, "fietz"), ["fietz"]);
}

#[test]
fn unsupported_language() {
    assert!(matches!(
        Speller::load("fr"),
        Err(SpellError::Unsupported(_))
    ));
}

#[test]
fn tokenizer_skips_urls_numbers_acronyms() {
    let text = "See https://x.nl, NATO and 2024 or a@b.nl";
    let got: Vec<&str> = words(text).into_iter().map(|r| &text[r]).collect();
    assert_eq!(got, ["See", "and", "or"]);
}

#[test]
fn tokenizer_keeps_internal_connectors_only() {
    let text = "'quoted' don’t well-known -dash- x1 www.site.nl A";
    let got: Vec<&str> = words(text).into_iter().map(|r| &text[r]).collect();
    assert_eq!(got, ["quoted", "don’t", "well-known", "dash"]);
}

#[test]
fn add_word_accepts() {
    let mut s = Speller::load("en").unwrap();
    assert_eq!(flagged(&s, "Reasoncraftish"), ["Reasoncraftish"]);
    assert!(s.add_word("Reasoncraftish"));
    assert!(flagged(&s, "Reasoncraftish").is_empty());
}

#[test]
fn add_word_refuses_affix_flags_and_blanks() {
    let mut s = Speller::load("en").unwrap();
    for bad in ["Reasoncraftish/S", "/", "", "  "] {
        assert!(!s.add_word(bad), "{bad:?} must be refused");
    }
    assert_eq!(
        flagged(&s, "Reasoncraftish"),
        ["Reasoncraftish"],
        "nothing added"
    );
}

#[test]
fn ranges_on_char_boundaries() {
    let s = Speller::load("en").unwrap();
    let text = "😀 café 😀 teh é";
    for r in words(text) {
        assert!(text.is_char_boundary(r.start) && text.is_char_boundary(r.end));
    }
    // The English dictionary may or may not know "café"; either way the slices are exact.
    let bad = flagged(&s, text);
    assert!(bad.contains(&"teh") && bad.iter().all(|w| ["café", "teh"].contains(w)));
}

#[test]
#[ignore = "timing; run with --release"]
fn load_speed() {
    use std::time::Instant;
    for lang in LANGUAGES {
        let t = Instant::now();
        let s = Speller::load(lang).unwrap();
        assert!(
            t.elapsed().as_secs_f64() < 1.5,
            "{lang} load {:?}",
            t.elapsed()
        );
        let t = Instant::now();
        for i in 0..10_000 {
            let _ = s.check_word(if i % 2 == 0 { "the" } else { "huis" });
        }
        assert!(
            t.elapsed().as_millis() < 200,
            "{lang} check {:?}",
            t.elapsed()
        );
    }
}

#[test]
fn no_suggestions_for_words_past_hunspells_length() {
    // Suggesting takes time superlinear in the length (346 characters
    // took 28 s, uncancellable): past MAX_SUGGEST_CHARS there are none.
    let s = Speller::load("nl").unwrap();
    let long = "ziekenhuisparkeerplaats".repeat(15) + "q";
    let start = std::time::Instant::now();
    assert!(s.suggest(&long, 5).is_empty());
    assert!(start.elapsed().as_secs() < 2, "{:?}", start.elapsed());
    let at_cap = "x".repeat(MAX_SUGGEST_CHARS + 1);
    assert!(s.suggest(&at_cap, 5).is_empty());
    assert!(!s.suggest("ziekenhuisparkeerplaatz", 5).is_empty());
}
