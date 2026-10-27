//! The corpus of hand-written Argdown samples: each reads as expected,
//! and no mutation of them makes the import panic or build an invalid map.

use super::tests_import::import;
use super::{ImportNoteKind as K, from_argdown};
use crate::model::{Acceptance, BoxKind, CardStrength, SourceType};

fn corpus() -> Vec<(String, String)> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/argdown/corpus");
    let mut files: Vec<(String, String)> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .map(|p| {
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            (name, std::fs::read_to_string(&p).unwrap())
        })
        .collect();
    files.sort();
    assert!(files.len() >= 10);
    files
}

/// A deterministic xorshift generator.
struct XorShift(u64);

impl XorShift {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

/// `src` changed a little: bytes deleted, a line duplicated, or a line's
/// indentation changed.
fn mutate(src: &str, rng: &mut XorShift) -> String {
    let mut bytes = src.as_bytes().to_vec();
    for _ in 0..=rng.below(3) {
        match rng.below(3) {
            0 if !bytes.is_empty() => {
                let at = rng.below(bytes.len());
                let n = 1 + rng.below(4);
                bytes.drain(at..(at + n).min(bytes.len()));
            }
            1 => {
                let text = String::from_utf8_lossy(&bytes).into_owned();
                let mut lines: Vec<&str> = text.lines().collect();
                if !lines.is_empty() {
                    let k = rng.below(lines.len());
                    lines.insert(k, lines[k]);
                }
                bytes = lines.join("\n").into_bytes();
            }
            _ => {
                let text = String::from_utf8_lossy(&bytes).into_owned();
                let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
                if !lines.is_empty() {
                    let k = rng.below(lines.len());
                    let pad = [" ", "\t", "    ", "        "][rng.below(4)].repeat(rng.below(3));
                    lines[k] = format!("{pad}{}", lines[k].trim_start());
                }
                bytes = lines.join("\n").into_bytes();
            }
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

#[test]
fn import_never_panics_on_corpus() {
    for (name, src) in corpus() {
        // Every sample imports; the import's own guard (a debug assertion
        // in tests) would stop at an invalid map.
        from_argdown(&src).unwrap_or_else(|e| panic!("{name}: {e}"));
        let seed = name.bytes().fold(0x9E37_79B9_7F4A_7C15u64, |h, b| {
            (h ^ u64::from(b)).wrapping_mul(0x100_0000_01B3)
        });
        let mut rng = XorShift(seed | 1);
        for k in 0..2000 {
            let text = mutate(&src, &mut rng);
            if let Ok((doc, _)) = from_argdown(&text)
                && let Err(e) = doc.validate()
            {
                panic!("{name} mutation {k}: {e}\n{text}");
            }
        }
    }
}

#[test]
fn corpus_samples_read_as_expected() {
    let files = corpus();
    let get = |n: &str| &files.iter().find(|(f, _)| f.starts_with(n)).unwrap().1;
    // Statement relations: a tab counts as 4 spaces, so the last line
    // hangs under the root.
    let (d, notes) = import(get("03"));
    assert_eq!(notes, vec![]);
    let root = &d.boxes[&d.root];
    assert_eq!(root.text, "Schools should require uniforms.");
    assert_eq!(root.children.len(), 4);
    // Frontmatter: an unknown language falls back to English.
    let (d, notes) = import(get("04"));
    assert_eq!(
        (d.meta.title.as_str(), d.meta.author.as_str()),
        ("Multi-step argument", "Ann")
    );
    assert_eq!(d.meta.language, "en");
    assert!(notes.contains(&K::InferenceRuleDropped) && notes.contains(&K::IntermediateConclusion));
    // Dutch heading splits off the parking area.
    let (d, _) = import(get("06"));
    assert_eq!(d.meta.title, "Fietsen = gezond");
    assert_eq!(d.meta.language, "nl");
    assert_eq!(d.parking.len(), 3);
    // Markup, links and mentions.
    let (d, notes) = import(get("07"));
    assert_eq!(notes, vec![K::TagsDropped]);
    let root = &d.boxes[&d.root];
    assert_eq!(root.text, "Remote work makes teams more productive.");
    let texts: Vec<&str> = d.boxes.values().map(|b| b.text.as_str()).collect();
    assert!(
        texts.contains(&"A Stanford study found a 13% gain."),
        "{texts:?}"
    );
    assert!(
        texts.contains(&"Meetings over video are tiring, says Study."),
        "{texts:?}"
    );
    let study = d
        .boxes
        .values()
        .find(|b| b.text.starts_with("A Stanford"))
        .unwrap();
    assert_eq!(study.link.as_deref(), Some("https://example.org/study"));
    // Sources, data and unsafe links.
    let (d, notes) = import(get("10"));
    assert_eq!(notes, vec![K::LinkDropped]);
    let root = &d.boxes[&d.root];
    assert_eq!(root.acceptance, Some(Acceptance::Accept));
    assert_eq!(root.link.as_deref(), Some("https://example.org/bees"));
    let kinds: Vec<&BoxKind> = d.boxes.values().map(|b| &b.kind).collect();
    let oral = BoxKind::Source {
        source_type: SourceType::Other("oral history".into()),
    };
    assert!(kinds.contains(&&oral));
    assert!(
        d.boxes
            .values()
            .all(|b| b.link.as_deref() != Some("javascript:alert(1)"))
    );
    let pest = d
        .boxes
        .values()
        .find(|b| b.text == "Pesticides harm bees.")
        .unwrap();
    assert_eq!(pest.comments[0].text, "Which ones?\nNeonicotinoids?");
    let weak = d
        .cards
        .values()
        .find(|c| c.strength == Some(CardStrength::Weak))
        .unwrap();
    assert_eq!(weak.children.len(), 1);
}
