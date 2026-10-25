//! Course-rule checks: hints about wording, structure and evaluation.
//!
//! Checks never change the document and never block an action; they only
//! list [`Issue`]s for the Check tab.

mod stopwords;
mod tokens;

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::ids::{CardId, Item, NodeId, Parent};
use crate::layout::display_children;
use crate::model::{
    Acceptance, BoxKind, BoxNode, Card, CardKind, CardStrength, Document, MapKind, SourceRating,
};

pub use tokens::tokenise;

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
    /// A category has no subcategories.
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
}

/// Which checks run. The default runs all of them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CheckSet {
    pub disabled: Vec<CheckKind>,
}

impl CheckSet {
    /// Whether `k` runs.
    pub fn is_on(&self, k: CheckKind) -> bool {
        !self.disabled.contains(&k)
    }
}

/// A hint about one item. `terms` holds the missing terms for
/// [`CheckKind::KeyTerm`] and the conjunctions found for
/// [`CheckKind::Compound`]; it is empty otherwise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    pub kind: CheckKind,
    pub item: Item,
    pub terms: Vec<String>,
}

/// Run the enabled checks. Issues follow the main tree in pre-order (the
/// display order of the canvas and the outline), then the parking area;
/// within one item they follow [`CheckKind::ALL`].
pub fn run_checks(doc: &Document, set: &CheckSet) -> Vec<Issue> {
    let run = Run::new(doc, set);
    let mut issues = Vec::new();
    // `seen` keeps a damaged, cyclic structure from looping forever.
    let mut seen = HashSet::with_capacity(doc.boxes.len() + doc.cards.len());
    let tops = std::iter::once((Item::Box(doc.root), None))
        .chain(doc.parking.iter().map(|&i| (i, Some(Parent::Parking))));
    let mut stack: Vec<(Item, Option<Parent>)> = Vec::new();
    for top in tops {
        stack.push(top);
        while let Some((item, parent)) = stack.pop() {
            if !seen.insert(item) {
                continue;
            }
            let start = stack.len();
            match item {
                Item::Box(id) => {
                    let Some(node) = doc.boxes.get(&id) else {
                        continue;
                    };
                    run.check_box(node, parent, &mut issues);
                    let me = Some(Parent::Box(id));
                    stack.extend(display_children(doc, id).into_iter().map(|c| (c, me)));
                }
                Item::Card(id) => {
                    let Some(card) = doc.cards.get(&id) else {
                        continue;
                    };
                    run.check_card(card, parent, &mut issues);
                    let me = Some(Parent::Card(id));
                    let premises = card.premises.iter().map(|&p| (Item::Box(p), me));
                    let whole = card.children.iter().map(|&c| (Item::Card(c), me));
                    stack.extend(premises.chain(whole));
                }
            }
            if parent == Some(Parent::Parking) {
                run.emit(&mut issues, CheckKind::NotConnected, item, Vec::new());
            }
            stack[start..].reverse();
        }
    }
    issues
}

/// What every item check needs, computed once per run.
struct Run<'a> {
    doc: &'a Document,
    set: &'a CheckSet,
    lang: &'a str,
    map: MapKind,
    any_rating: bool,
}

impl<'a> Run<'a> {
    fn new(doc: &'a Document, set: &'a CheckSet) -> Self {
        let any_rating = doc
            .boxes
            .values()
            .any(|b| b.acceptance.is_some() || b.source_rating.is_some())
            || doc.cards.values().any(|c| c.strength.is_some());
        Self {
            doc,
            set,
            lang: &doc.meta.language,
            map: doc.map_kind(),
            any_rating,
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

    fn check_box(&self, node: &BoxNode, parent: Option<Parent>, issues: &mut Vec<Issue>) {
        let item = Item::Box(node.id);
        let statement = matches!(node.kind, BoxKind::Claim | BoxKind::Premise);
        if statement && self.on(CheckKind::Compound) {
            let found = conjunctions(&node.text, self.lang);
            if !found.is_empty() {
                self.emit(issues, CheckKind::Compound, item, found);
            }
        }
        if statement && node.text.trim_end().ends_with('?') {
            self.emit(issues, CheckKind::Question, item, Vec::new());
        }
        if node.kind == BoxKind::Premise && self.map == MapKind::Reasoning && !self.supported(node)
        {
            self.emit(issues, CheckKind::Unsupported, item, Vec::new());
        }
        if node.kind == BoxKind::Category
            && self.map == MapKind::Grouping
            && !node
                .children
                .iter()
                .any(|&c| self.is_kind(c, |k| *k == BoxKind::Category))
        {
            self.emit(issues, CheckKind::EmptyCategory, item, Vec::new());
        }
        let is_source = matches!(node.kind, BoxKind::Source { .. });
        let unrated =
            (statement && node.acceptance.is_none()) || (is_source && node.source_rating.is_none());
        if self.any_rating && unrated {
            self.emit(issues, CheckKind::IncompleteEvaluation, item, Vec::new());
        }
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
        if self.any_rating && card.strength.is_none() {
            self.emit(issues, CheckKind::IncompleteEvaluation, item, Vec::new());
        }
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
/// "and"/"or" in English, "en"/"of" in Dutch.
fn conjunctions(text: &str, lang: &str) -> Vec<String> {
    let list: [&str; 2] = if lang == "nl" {
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
