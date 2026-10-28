//! Statement and argument titles, and the order the writer follows.
//!
//! One pre-order walk over the main tree and then each parked fragment
//! (the Outline's order) names every box written as a statement and every
//! card, and remembers which cards each part holds and what each card
//! targets.

use std::collections::{HashMap, HashSet};

use crate::ids::{CardId, Item, NodeId, Parent};
use crate::model::{BoxKind, CardKind, Document};

/// The longest title made from a box's text, in characters.
const MAX_TITLE_CHARS: usize = 40;
/// How many words of the text a title takes.
const TITLE_WORDS: usize = 5;

/// The names and the order of everything the writer writes.
pub(super) struct Names {
    boxes: HashMap<NodeId, String>,
    cards: HashMap<CardId, String>,
    /// What each card targets: a box, or a whole card. Parked cards have
    /// no entry.
    pub targets: HashMap<CardId, Parent>,
    /// The cards of the main tree, in outline order.
    pub main: Vec<CardId>,
    /// The cards of each parked fragment (`doc.parking` order), in outline
    /// order, without the fragment's own card.
    pub parked: Vec<Vec<CardId>>,
}

impl Names {
    pub fn new(doc: &Document) -> Self {
        let mut names = Names {
            boxes: HashMap::new(),
            cards: HashMap::new(),
            targets: HashMap::new(),
            main: Vec::new(),
            parked: Vec::new(),
        };
        let mut used = HashSet::new();
        let mut counts = Counts::default();
        names.main = names.walk(doc, Item::Box(doc.root), &mut used, &mut counts);
        for &item in &doc.parking {
            let mut cards = names.walk(doc, item, &mut used, &mut counts);
            if let Item::Card(top) = item {
                cards.retain(|&c| c != top);
            }
            names.parked.push(cards);
        }
        names
    }

    /// The title of a statement box ("" for a box written as a note).
    pub fn of_box(&self, id: NodeId) -> &str {
        self.boxes.get(&id).map_or("", String::as_str)
    }

    /// The title of a card's argument, `R<n>` or `O<n>`.
    pub fn of_card(&self, id: CardId) -> &str {
        self.cards.get(&id).map_or("", String::as_str)
    }

    /// Names everything from `start` down; returns the cards met, in order.
    fn walk(
        &mut self,
        doc: &Document,
        start: Item,
        used: &mut HashSet<String>,
        counts: &mut Counts,
    ) -> Vec<CardId> {
        let mut cards = Vec::new();
        let mut stack = vec![start];
        while let Some(item) = stack.pop() {
            match item {
                Item::Box(id) => {
                    let Some(node) = doc.boxes.get(&id) else {
                        continue;
                    };
                    // Notes are data of their owner, except a parked one.
                    if node.kind != BoxKind::Note || item == start {
                        counts.statements += 1;
                        let title = unique(base_title(&node.text), counts.statements, used);
                        self.boxes.insert(id, title);
                    }
                    for &child in &node.children {
                        if let Item::Card(c) = child {
                            self.targets.insert(c, Parent::Box(id));
                        }
                    }
                    stack.extend(node.children.iter().rev());
                }
                Item::Card(id) => {
                    let Some(card) = doc.cards.get(&id) else {
                        continue;
                    };
                    let title = match card.kind {
                        CardKind::Reason => {
                            counts.reasons += 1;
                            format!("R{}", counts.reasons)
                        }
                        CardKind::Objection => {
                            counts.objections += 1;
                            format!("O{}", counts.objections)
                        }
                    };
                    self.cards.insert(id, title);
                    cards.push(id);
                    for &child in &card.children {
                        self.targets.insert(child, Parent::Card(id));
                    }
                    // Premises (with what hangs under them) first, then
                    // the whole-card children.
                    stack.extend(card.children.iter().rev().map(|&c| Item::Card(c)));
                    stack.extend(card.premises.iter().rev().map(|&p| Item::Box(p)));
                }
            }
        }
        cards
    }
}

#[derive(Default)]
struct Counts {
    statements: usize,
    reasons: usize,
    objections: usize,
}

/// A title from the first words of `text`: characters with a meaning in
/// Argdown titles (`[]<>{}:#@`, the backslash, and a slash that would
/// start a comment) are left out, and so are leading hyphens; whitespace
/// is collapsed and the result is cut at [`MAX_TITLE_CHARS`].
///
/// The slash is judged on what is kept, not on the original text: in
/// `x/#/y` the `#` goes, and the two slashes it leaves would start a
/// comment that hides the rest of the line (`/*`: the rest of the file).
pub(super) fn base_title(text: &str) -> String {
    let mut kept = String::with_capacity(text.len());
    for c in text.chars() {
        if "[]<>{}:#@\\".contains(c) {
            continue;
        }
        if matches!(c, '/' | '*') && kept.ends_with('/') {
            kept.pop();
        }
        kept.push(c);
    }
    // A title starting with `-` is not a statement reference in Argdown.
    let kept = kept.trim_start_matches(|c: char| c == '-' || c.is_whitespace());
    let words: Vec<&str> = kept.split_whitespace().take(TITLE_WORDS).collect();
    let title: String = words.join(" ").chars().take(MAX_TITLE_CHARS).collect();
    title.trim_end().to_owned()
}

/// `base`, or `Statement <n>` when it is empty, made unique among `used`
/// by appending " 2", " 3", ….
fn unique(base: String, n: usize, used: &mut HashSet<String>) -> String {
    let base = if base.is_empty() {
        format!("Statement {n}")
    } else {
        base
    };
    let mut title = base.clone();
    let mut k = 1;
    while used.contains(&title) {
        k += 1;
        title = format!("{base} {k}");
    }
    used.insert(title.clone());
    title
}
