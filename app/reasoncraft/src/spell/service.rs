//! Dictionaries per language, loaded on a worker thread the first time a
//! language is asked for, and the misspellings found in a text, cached
//! per language and text.

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::ops::Range;
use std::sync::Arc;
use std::time::Duration;

use reasoncraft_spell::Speller;

use crate::jobs::{Job, JobState};

/// How many texts keep their misspellings cached.
const CACHE_SIZE: usize = 256;

/// The misspelled byte ranges of one text.
pub type Ranges = Arc<Vec<Range<usize>>>;

/// One language's dictionary.
enum Dict {
    Loading {
        job: Job<Result<Speller, String>>,
        /// How many personal words the job was given.
        words: usize,
    },
    Ready(Arc<Speller>),
    Failed(String),
}

/// The dictionaries of every language asked for so far, the user's
/// personal words, the words ignored for this session and the cache of
/// misspellings found.
pub struct SpellService {
    dicts: HashMap<String, Dict>,
    /// The personal dictionary (kept in the settings), added to every
    /// dictionary as it loads.
    personal: Vec<String>,
    /// "Ignore all" words, until the app closes.
    ignored: HashSet<String>,
    /// Misspellings per (language, text hash), with when each was last used.
    cache: HashMap<(String, u64), (u64, Ranges)>,
    tick: u64,
    /// How many texts have been checked (cache misses).
    checks: u64,
    /// Load failures not reported yet.
    errors: Vec<String>,
    /// Wait this long before each dictionary loads (tests only).
    pub delay: Option<Duration>,
}

impl SpellService {
    /// A service with no dictionary loaded yet; `personal` words are
    /// accepted in every language.
    pub fn new(personal: &[String]) -> Self {
        Self {
            dicts: HashMap::new(),
            personal: personal.to_vec(),
            ignored: HashSet::new(),
            cache: HashMap::new(),
            tick: 0,
            checks: 0,
            errors: Vec::new(),
            delay: None,
        }
    }

    /// The dictionary for `lang`, once it is loaded. The first call starts
    /// loading it on a worker thread and returns `None`, as do calls while
    /// it loads and after it failed. Never blocks.
    pub fn get(&mut self, lang: &str) -> Option<Arc<Speller>> {
        self.poll(lang);
        match self.dicts.get(lang) {
            Some(Dict::Ready(s)) => Some(s.clone()),
            Some(_) => None,
            None => {
                let words = self.personal.clone();
                let n = words.len();
                let owned = lang.to_owned();
                let job = Job::spawn(self.delay, move || load(&owned, &words));
                self.dicts
                    .insert(lang.to_owned(), Dict::Loading { job, words: n });
                None
            }
        }
    }

    /// Whether `lang`'s dictionary was asked for (loading, loaded or failed).
    pub fn requested(&self, lang: &str) -> bool {
        self.dicts.contains_key(lang)
    }

    /// Whether `lang`'s dictionary is being loaded.
    pub fn is_loading(&self, lang: &str) -> bool {
        matches!(self.dicts.get(lang), Some(Dict::Loading { .. }))
    }

    /// Why `lang`'s dictionary failed to load, if it did.
    pub fn failure(&self, lang: &str) -> Option<&str> {
        match self.dicts.get(lang) {
            Some(Dict::Failed(e)) => Some(e),
            _ => None,
        }
    }

    /// Whether any dictionary is still loading.
    pub fn busy(&self) -> bool {
        self.dicts
            .values()
            .any(|d| matches!(d, Dict::Loading { .. }))
    }

    /// Pick up every dictionary that finished loading.
    pub fn poll_all(&mut self) {
        let langs: Vec<String> = self.dicts.keys().cloned().collect();
        for lang in langs {
            self.poll(&lang);
        }
    }

    /// The next load failure not reported yet.
    pub fn take_error(&mut self) -> Option<String> {
        (!self.errors.is_empty()).then(|| self.errors.remove(0))
    }

    fn poll(&mut self, lang: &str) {
        let Some(Dict::Loading { job, words }) = self.dicts.get(lang) else {
            return;
        };
        let result = match job.poll() {
            JobState::Running => return,
            JobState::Done(r) => r,
            JobState::Lost => Err("the loader stopped".to_owned()),
        };
        let since = *words;
        let dict = match result {
            Ok(mut s) => {
                // Words added while it loaded.
                for w in &self.personal[since..] {
                    s.add_word(w);
                }
                Dict::Ready(Arc::new(s))
            }
            Err(e) => {
                self.errors.push(e.clone());
                Dict::Failed(e)
            }
        };
        self.dicts.insert(lang.to_owned(), dict);
    }

    /// The misspelled byte ranges of `text` in `lang`, leaving out words
    /// ignored for the session; `None` until the dictionary is loaded.
    /// Found once per text: later calls with the same text hit the cache.
    pub fn misspelled(&mut self, lang: &str, text: &str) -> Option<Ranges> {
        let speller = self.get(lang)?;
        let mut h = DefaultHasher::new();
        text.hash(&mut h);
        let key = (lang.to_owned(), h.finish());
        self.tick += 1;
        if let Some((used, ranges)) = self.cache.get_mut(&key) {
            *used = self.tick;
            return Some(ranges.clone());
        }
        self.checks += 1;
        let ranges: Ranges = Arc::new(
            speller
                .misspelled(text)
                .into_iter()
                .filter(|r| !self.ignored.contains(&text[r.clone()]))
                .collect(),
        );
        if self.cache.len() >= CACHE_SIZE
            && let Some(oldest) = self
                .cache
                .iter()
                .min_by_key(|(_, (used, _))| *used)
                .map(|(k, _)| k.clone())
        {
            self.cache.remove(&oldest);
        }
        self.cache.insert(key, (self.tick, ranges.clone()));
        Some(ranges)
    }

    /// Up to `max` suggestions for `word` in `lang` (none while loading).
    pub fn suggest(&mut self, lang: &str, word: &str, max: usize) -> Vec<String> {
        self.get(lang)
            .map(|s| s.suggest(word, max))
            .unwrap_or_default()
    }

    /// Accept `word` everywhere until the app closes.
    pub fn ignore_all(&mut self, word: &str) {
        self.ignored.insert(word.to_owned());
        self.cache.clear();
    }

    /// Add `word` to the personal dictionary of every language. False,
    /// adding nothing, for a word no dictionary can hold (blank, or with
    /// a `/`).
    pub fn add_word(&mut self, word: &str) -> bool {
        let word = word.trim();
        if word.is_empty() || word.contains('/') {
            return false;
        }
        for dict in self.dicts.values_mut() {
            if let Dict::Ready(s) = dict {
                match Arc::get_mut(s) {
                    Some(s) => {
                        s.add_word(word);
                    }
                    // Only the service keeps a dictionary between frames,
                    // so this does not happen; still accept the word.
                    None => {
                        self.ignored.insert(word.to_owned());
                    }
                }
            }
        }
        self.personal.push(word.to_owned());
        self.cache.clear();
        true
    }

    /// How many texts were checked against a dictionary so far (each
    /// cache miss counts once).
    pub fn checks_run(&self) -> u64 {
        self.checks
    }
}

/// Load `lang`'s dictionary with the personal `words` added.
fn load(lang: &str, words: &[String]) -> Result<Speller, String> {
    let mut s = Speller::load(lang).map_err(|e| e.to_string())?;
    for w in words {
        s.add_word(w);
    }
    Ok(s)
}

#[cfg(test)]
pub(crate) mod tests {
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
        assert!(s.add_word("blorb"));
        assert!(s.misspelled("en", "teh blorb").unwrap().is_empty());
        assert!(!s.add_word("a/b") && !s.add_word(" "));
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
}
