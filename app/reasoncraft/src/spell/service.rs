//! Dictionaries per language, loaded on a worker thread the first time a
//! language is asked for; the misspellings found in a text, cached per
//! language and line; and suggestions, worked out on a worker thread.

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::ops::Range;
use std::sync::Arc;
use std::time::Duration;

use reasoncraft_spell::Speller;

use super::MAX_SUGGESTIONS;
use super::line_cache::LineCache;
use crate::jobs::{Job, JobState};

/// How many words keep their suggestions.
const SUGGESTION_CACHE: usize = 64;

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

/// The suggestions for one word: being worked out, or found.
enum Suggestions {
    Pending(Job<Vec<String>>),
    Ready(Vec<String>),
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
    /// Misspellings per (language, line hash).
    cache: LineCache,
    /// Suggestions per (language, word).
    suggestions: HashMap<(String, String), Suggestions>,
    /// How many lines have been checked (cache misses).
    checks: u64,
    /// Load failures not reported yet.
    errors: Vec<String>,
    /// Wait this long before each dictionary load and each suggestion
    /// job (tests only).
    pub delay: Option<Duration>,
    /// Let the spelling pass check this many boxes a frame instead of
    /// going by time (tests only).
    pub slice_boxes: Option<usize>,
}

impl SpellService {
    /// A service with no dictionary loaded yet; `personal` words are
    /// accepted in every language.
    pub fn new(personal: &[String]) -> Self {
        Self {
            dicts: HashMap::new(),
            personal: personal.to_vec(),
            ignored: HashSet::new(),
            cache: LineCache::default(),
            suggestions: HashMap::new(),
            checks: 0,
            errors: Vec::new(),
            delay: None,
            slice_boxes: None,
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

    /// Whether any dictionary is still loading, or suggestions are being
    /// worked out.
    pub fn busy(&self) -> bool {
        let loading = |d: &Dict| matches!(d, Dict::Loading { .. });
        let pending = |s: &Suggestions| matches!(s, Suggestions::Pending(_));
        self.dicts.values().any(loading) || self.suggestions.values().any(pending)
    }

    /// Pick up every dictionary that finished loading, and every
    /// suggestion job that ended (also those for words no longer on
    /// show, so none keeps the service busy).
    pub fn poll_all(&mut self) {
        let langs: Vec<String> = self.dicts.keys().cloned().collect();
        for lang in langs {
            self.poll(&lang);
        }
        for entry in self.suggestions.values_mut() {
            if let Suggestions::Pending(job) = entry
                && let Some(found) = finished(job)
            {
                *entry = Suggestions::Ready(found);
            }
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
    /// Found once per line: a line seen before hits the cache, so typing
    /// checks only the line typed in.
    pub fn misspelled(&mut self, lang: &str, text: &str) -> Option<Ranges> {
        let speller = self.get(lang)?;
        let mut found = Vec::new();
        let mut at = 0;
        for line in text.split('\n') {
            let ranges = self.line_misspelled(&speller, lang, line);
            found.extend(ranges.iter().map(|r| r.start + at..r.end + at));
            at += line.len() + 1;
        }
        Some(Arc::new(found))
    }

    /// The misspelled byte ranges of one `line`, from the cache when it
    /// was checked before.
    fn line_misspelled(&mut self, speller: &Speller, lang: &str, line: &str) -> Ranges {
        let mut h = DefaultHasher::new();
        line.hash(&mut h);
        let key = (lang.to_owned(), h.finish());
        if let Some(ranges) = self.cache.get(&key) {
            return ranges;
        }
        self.checks += 1;
        let ranges: Ranges = Arc::new(
            speller
                .misspelled(line)
                .into_iter()
                .filter(|r| !self.ignored.contains(&line[r.clone()]))
                .collect(),
        );
        self.cache.insert(key, ranges.clone());
        ranges
    }

    /// Up to [`MAX_SUGGESTIONS`] suggestions for `word` in `lang`, worked
    /// out on a worker thread: `None` while that runs (the first call
    /// starts it) and while the dictionary loads. Never blocks.
    pub fn suggest_async(&mut self, lang: &str, word: &str) -> Option<Vec<String>> {
        let key = (lang.to_owned(), word.to_owned());
        if let Some(entry) = self.suggestions.get_mut(&key) {
            if let Suggestions::Pending(job) = entry {
                *entry = Suggestions::Ready(finished(job)?);
            }
            return match entry {
                Suggestions::Ready(found) => Some(found.clone()),
                Suggestions::Pending(_) => None,
            };
        }
        let speller = self.get(lang)?;
        if self.suggestions.len() >= SUGGESTION_CACHE {
            self.suggestions
                .retain(|_, s| matches!(s, Suggestions::Pending(_)));
        }
        let owned = word.to_owned();
        let job = Job::spawn(self.delay, move || speller.suggest(&owned, MAX_SUGGESTIONS));
        self.suggestions.insert(key, Suggestions::Pending(job));
        None
    }

    /// Accept `word` everywhere until the app closes.
    pub fn ignore_all(&mut self, word: &str) {
        self.ignored.insert(word.to_owned());
        self.cache.clear();
    }

    /// Add `word` to the personal dictionary of every language and return
    /// it as kept (trimmed); `None`, adding nothing, for a word no
    /// dictionary can hold (see [`reasoncraft_spell::personal_word`]).
    pub fn add_word(&mut self, word: &str) -> Option<String> {
        let word = reasoncraft_spell::personal_word(word)?.to_owned();
        let mut taken = true;
        for dict in self.dicts.values_mut() {
            if let Dict::Ready(s) = dict {
                match Arc::get_mut(s) {
                    Some(s) => taken &= s.add_word(&word),
                    // A suggestion job still holds the dictionary: accept
                    // the word for the session (and from the next start
                    // through the personal list).
                    None => {
                        self.ignored.insert(word.clone());
                    }
                }
            }
        }
        if !taken {
            return None;
        }
        if !self.personal.contains(&word) {
            self.personal.push(word.clone());
        }
        self.cache.clear();
        Some(word)
    }

    /// How many lines were checked against a dictionary so far (each
    /// cache miss counts once).
    pub fn checks_run(&self) -> u64 {
        self.checks
    }
}

/// A suggestion job's result once it ended (none when it was lost).
fn finished(job: &Job<Vec<String>>) -> Option<Vec<String>> {
    match job.poll() {
        JobState::Running => None,
        JobState::Done(found) => Some(found),
        JobState::Lost => Some(Vec::new()),
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
#[path = "service_tests.rs"]
pub(crate) mod tests;
