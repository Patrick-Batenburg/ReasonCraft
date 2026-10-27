//! Course-rule checks: hints about wording, structure and evaluation.
//!
//! Checks never change the document and never block an action; they only
//! list [`Issue`]s for the Check tab.

pub(crate) mod stopwords;
mod tokens;
pub(crate) mod walk;

use serde::{Deserialize, Serialize};

use crate::ids::{CardId, Item, NodeId, Parent};
use crate::model::{
    Acceptance, BoxKind, BoxNode, Card, CardKind, CardStrength, Document, MapKind, SourceRating,
};

pub use tokens::tokenise;
use walk::{Visit, rated, visit_order};

/// One course rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckKind {
    /// A term of the target statement appears in none of the card's premises.
    KeyTerm,
    /// A claim or premise joins two statements with "and"/"or".
    Compound,
    /// A claim or premise is phrased as a question.
    Question,
    /// A premise has neither a source nor a reason under it.
    Unsupported,
    /// A category directly under the root has nothing under it.
    EmptyCategory,
    /// The map is partly rated.
    IncompleteEvaluation,
    /// A rating contradicts the ratings it rests on.
    InconsistentEvaluation,
    /// An item sits in the parking area.
    NotConnected,
}

impl CheckKind {
    /// Every check, in the order issues of one item are listed.
    pub const ALL: [CheckKind; 8] = [
        CheckKind::KeyTerm,
        CheckKind::Compound,
        CheckKind::Question,
        CheckKind::Unsupported,
        CheckKind::EmptyCategory,
        CheckKind::IncompleteEvaluation,
        CheckKind::InconsistentEvaluation,
        CheckKind::NotConnected,
    ];

    /// The check's saved name (its snake_case serde name).
    pub fn key(self) -> &'static str {
        match self {
            CheckKind::KeyTerm => "key_term",
            CheckKind::Compound => "compound",
            CheckKind::Question => "question",
            CheckKind::Unsupported => "unsupported",
            CheckKind::EmptyCategory => "empty_category",
            CheckKind::IncompleteEvaluation => "incomplete_evaluation",
            CheckKind::InconsistentEvaluation => "inconsistent_evaluation",
            CheckKind::NotConnected => "not_connected",
        }
    }
}

/// Which checks run. The default runs all of them.
///
/// Names of checks this build does not know (from a newer build's settings)
/// are skipped when loading, so the rest of the settings still load.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CheckSet {
    #[serde(deserialize_with = "known_kinds")]
    pub disabled: Vec<CheckKind>,
}

fn known_kinds<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<CheckKind>, D::Error> {
    Ok(Vec::<MaybeKind>::deserialize(d)?
        .into_iter()
        .filter_map(|k| k.0)
        .collect())
}

/// A saved check name, `None` when this build does not know it. Read as an
/// identifier, like the derived [`CheckKind`] impl, so it suits every format
/// the settings are saved in.
struct MaybeKind(Option<CheckKind>);

impl<'de> Deserialize<'de> for MaybeKind {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Name;
        impl serde::de::Visitor<'_> for Name {
            type Value = MaybeKind;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a check name")
            }
            fn visit_str<E>(self, v: &str) -> Result<MaybeKind, E> {
                Ok(MaybeKind(CheckKind::ALL.into_iter().find(|k| k.key() == v)))
            }
        }
        d.deserialize_identifier(Name)
    }
}

impl CheckSet {
    /// Whether `k` runs.
    pub fn is_on(&self, k: CheckKind) -> bool {
        !self.disabled.contains(&k)
    }
}

/// A hint about one item. `terms` holds the missing terms for
/// [`CheckKind::KeyTerm`], the conjunctions found for
/// [`CheckKind::Compound`] and the number of unrated items (as text) for
/// [`CheckKind::IncompleteEvaluation`]; it is empty otherwise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    pub kind: CheckKind,
    pub item: Item,
    pub terms: Vec<String>,
}

/// Run the enabled checks. Issues follow the main tree in pre-order (the
/// display order of the canvas and the outline), then the parking area;
/// within one item they follow [`CheckKind::ALL`].
///
/// Unrated items are reported as ONE [`CheckKind::IncompleteEvaluation`]
/// issue on the first of them, carrying their count. Parked items (anything
/// inside a top-level parking fragment) are drafts: they get no
/// `Unsupported` or `IncompleteEvaluation` issue, their ratings do not
/// count as "rating has started", and a parked note is not reported as
/// `NotConnected`.
pub fn run_checks(doc: &Document, set: &CheckSet) -> Vec<Issue> {
    let order = visit_order(doc);
    let any_rating = order
        .iter()
        .any(|v| !v.parked && rated(doc, v.item) == Some(true));
    let run = Run::new(doc, set);
    let mut issues = Vec::new();
    // Where the single "not rated yet" issue goes, and its count.
    let mut first_unrated: Option<(usize, Item)> = None;
    let mut unrated = 0usize;
    for v in &order {
        run.check_before_rating(v, &mut issues);
        if any_rating && !v.parked && rated(doc, v.item) == Some(false) {
            unrated += 1;
            first_unrated.get_or_insert((issues.len(), v.item));
        }
        run.check_after_rating(v, &mut issues);
    }
    if let Some((at, item)) = first_unrated
        && run.on(CheckKind::IncompleteEvaluation)
    {
        let terms = vec![unrated.to_string()];
        let kind = CheckKind::IncompleteEvaluation;
        issues.insert(at, Issue { kind, item, terms });
    }
    issues.retain(|i| !doc.ignored_hints.iter().any(|h| h.matches(i)));
    issues
}

/// What every item check needs, computed once per run.
struct Run<'a> {
    doc: &'a Document,
    set: &'a CheckSet,
    lang: &'a str,
    map: MapKind,
}

impl<'a> Run<'a> {
    fn new(doc: &'a Document, set: &'a CheckSet) -> Self {
        Self {
            doc,
            set,
            lang: &doc.meta.language,
            map: doc.map_kind(),
        }
    }

    /// The item's checks listed before `IncompleteEvaluation`.
    fn check_before_rating(&self, v: &Visit, issues: &mut Vec<Issue>) {
        match v.item {
            Item::Box(id) => self.check_box(&self.doc.boxes[&id], v, issues),
            Item::Card(id) => self.check_card(&self.doc.cards[&id], v.parent, issues),
        }
    }

    /// The item's checks listed after `IncompleteEvaluation`.
    fn check_after_rating(&self, v: &Visit, issues: &mut Vec<Issue>) {
        match v.item {
            Item::Box(id) => self.check_box_rating(&self.doc.boxes[&id], v.parent, issues),
            Item::Card(id) => self.check_card_rating(&self.doc.cards[&id], issues),
        }
        let note = self.is_kind(v.item, |k| *k == BoxKind::Note);
        if v.parent == Some(Parent::Parking) && !note {
            self.emit(issues, CheckKind::NotConnected, v.item, Vec::new());
        }
    }

    fn on(&self, kind: CheckKind) -> bool {
        self.set.is_on(kind)
    }

    fn emit(&self, issues: &mut Vec<Issue>, kind: CheckKind, item: Item, terms: Vec<String>) {
        if self.on(kind) {
            issues.push(Issue { kind, item, terms });
        }
    }

    fn check_box(&self, node: &BoxNode, v: &Visit, issues: &mut Vec<Issue>) {
        let item = Item::Box(node.id);
        let parent = v.parent;
        let statement = matches!(node.kind, BoxKind::Claim | BoxKind::Premise);
        if statement && self.on(CheckKind::Compound) {
            let found = conjunctions(&tokens::without_quoted(&node.text), self.lang);
            if !found.is_empty() {
                self.emit(issues, CheckKind::Compound, item, found);
            }
        }
        if statement && node.text.trim_end().ends_with('?') {
            self.emit(issues, CheckKind::Question, item, Vec::new());
        }
        if node.kind == BoxKind::Premise
            && self.map == MapKind::Reasoning
            && !v.parked
            && !self.supported(node)
        {
            self.emit(issues, CheckKind::Unsupported, item, Vec::new());
        }
        if node.kind == BoxKind::Category
            && self.map == MapKind::Grouping
            && parent == Some(Parent::Box(self.doc.root))
            && node.children.is_empty()
        {
            self.emit(issues, CheckKind::EmptyCategory, item, Vec::new());
        }
    }

    fn check_box_rating(&self, node: &BoxNode, parent: Option<Parent>, issues: &mut Vec<Issue>) {
        let item = Item::Box(node.id);
        let is_source = matches!(node.kind, BoxKind::Source { .. });
        let under_rejected = match parent {
            Some(Parent::Box(p)) => self.doc.boxes.get(&p).is_some_and(|p| {
                p.kind == BoxKind::Premise && p.acceptance == Some(Acceptance::Reject)
            }),
            _ => false,
        };
        if is_source && node.source_rating == Some(SourceRating::Solid) && under_rejected {
            self.emit(issues, CheckKind::InconsistentEvaluation, item, Vec::new());
        }
    }

    fn check_card(&self, card: &Card, parent: Option<Parent>, issues: &mut Vec<Issue>) {
        let item = Item::Card(card.id);
        if let Some(Parent::Box(target)) = parent
            && self.map == MapKind::Reasoning
            && self.on(CheckKind::KeyTerm)
        {
            let missing = self.missing_terms(card, target);
            if !missing.is_empty() {
                self.emit(issues, CheckKind::KeyTerm, item, missing);
            }
        }
    }

    fn check_card_rating(&self, card: &Card, issues: &mut Vec<Issue>) {
        let item = Item::Card(card.id);
        let all_premises = |a: Acceptance| {
            !card.premises.is_empty()
                && card.premises.iter().all(|p| {
                    self.doc
                        .boxes
                        .get(p)
                        .is_some_and(|b| b.acceptance == Some(a))
                })
        };
        let inconsistent = match card.strength {
            Some(CardStrength::Strong) => all_premises(Acceptance::Reject),
            Some(CardStrength::Nil) => all_premises(Acceptance::Accept) && !self.objected(card),
            Some(CardStrength::Weak) | None => false,
        };
        if inconsistent {
            self.emit(issues, CheckKind::InconsistentEvaluation, item, Vec::new());
        }
    }

    /// Whether `item` is a box whose kind passes `test`.
    fn is_kind(&self, item: Item, test: impl Fn(&BoxKind) -> bool) -> bool {
        match item {
            Item::Box(b) => self.doc.boxes.get(&b).is_some_and(|n| test(&n.kind)),
            Item::Card(_) => false,
        }
    }

    fn is_card_of(&self, id: CardId, kind: CardKind) -> bool {
        self.doc.cards.get(&id).is_some_and(|c| c.kind == kind)
    }

    /// A premise is supported by a source or a reason card under it.
    fn supported(&self, node: &BoxNode) -> bool {
        node.children.iter().any(|&c| match c {
            Item::Card(id) => self.is_card_of(id, CardKind::Reason),
            Item::Box(_) => self.is_kind(c, |k| matches!(k, BoxKind::Source { .. })),
        })
    }

    /// Whether an objection attacks the card as a whole or one of its
    /// premises.
    fn objected(&self, card: &Card) -> bool {
        let objection = |id| self.is_card_of(id, CardKind::Objection);
        card.children.iter().any(|&c| objection(c))
            || card.premises.iter().any(|p| {
                self.doc.boxes.get(p).is_some_and(|b| {
                    b.children
                        .iter()
                        .any(|&c| matches!(c, Item::Card(id) if objection(id)))
                })
            })
    }

    /// Terms of the target box that no premise of `card` mentions, in
    /// order and without repeats.
    fn missing_terms(&self, card: &Card, target: NodeId) -> Vec<String> {
        let Some(target) = self.doc.boxes.get(&target) else {
            return Vec::new();
        };
        let mut present: Vec<String> = Vec::new();
        for p in card.premises.iter().filter_map(|p| self.doc.boxes.get(p)) {
            for word in tokens::words(&p.text) {
                let joined = word.joined();
                present.extend(joined.into_iter().chain(word.parts));
            }
        }
        present.retain(|t| tokens::is_term(t, self.lang));
        let found = |t: &str| present.iter().any(|p| tokens::same_term(t, p));
        let mut missing: Vec<String> = Vec::new();
        for word in tokens::words(&target.text) {
            let joined_found = word.joined().is_some_and(|j| found(&j));
            for part in word.parts {
                if tokens::is_term(&part, self.lang)
                    && !joined_found
                    && !found(&part)
                    && !missing.contains(&part)
                {
                    missing.push(part);
                }
            }
        }
        missing
    }
}

/// The distinct standalone conjunctions in `text`, in order of appearance:
/// "and"/"or" in English, "en"/"of" in Dutch (see [`stopwords::is_dutch`]).
fn conjunctions(text: &str, lang: &str) -> Vec<String> {
    let list: [&str; 2] = if stopwords::is_dutch(lang) {
        ["en", "of"]
    } else {
        ["and", "or"]
    };
    let mut found: Vec<String> = Vec::new();
    for word in tokens::words(text) {
        if let [part] = word.parts.as_slice()
            && list.contains(&part.as_str())
            && !found.contains(part)
        {
            found.push(part.clone());
        }
    }
    found
}
