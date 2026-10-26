//! The map as a numbered text outline.
//!
//! [`build_outline`] walks the main tree and then the parked fragments in
//! pre-order, using the same child order as the canvas, so numbers match what
//! users see. It uses an explicit stack, so chains of any depth are safe.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::ids::{CardId, Item, NodeId};
use crate::layout::display_children;
use crate::model::{BoxKind, CardStrength, Document, Role, SourceType};
use crate::roles::card_roles;

/// The linking word before a premise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkLabel {
    Because,
    But,
    However,
    None,
}

/// What an outline line stands for.
#[derive(Debug, Clone, PartialEq)]
pub enum LineKind {
    Claim,
    Premise {
        card: CardId,
        role: Role,
        strength: Option<CardStrength>,
    },
    Category,
    Source(SourceType),
    Note,
}

/// One line of the outline.
#[derive(Debug, Clone, PartialEq)]
pub struct OutlineLine {
    pub item: Item,
    pub depth: usize,
    pub number: String,
    pub label: LinkLabel,
    pub text: String,
    pub kind: LineKind,
    /// Belongs to a card's whole-card attacks rather than to one premise.
    pub whole_card: bool,
    /// Part of a parked fragment.
    pub parked: bool,
}

/// What the outline shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct OutlineOptions {
    pub numbering: bool,
    pub labels: bool,
    pub notes: bool,
}

impl Default for OutlineOptions {
    fn default() -> Self {
        Self {
            numbering: true,
            labels: true,
            notes: false,
        }
    }
}

/// A box or card still to be written, with where it sits.
enum Task {
    Box {
        id: NodeId,
        number: String,
        depth: usize,
        /// The card this box is a premise of.
        premise_of: Option<CardId>,
        whole_card: bool,
    },
    Card {
        id: CardId,
        number: String,
        depth: usize,
        whole_card: bool,
    },
}

/// `a`, `b`, … `z`, `aa`, `ab`, …
fn letter(mut i: usize) -> String {
    let mut out = Vec::new();
    loop {
        out.push(b'a' + (i % 26) as u8);
        if i < 26 {
            break;
        }
        i = i / 26 - 1;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

fn join(prefix: &str, part: &str) -> String {
    if prefix.is_empty() {
        part.to_owned()
    } else {
        format!("{prefix}.{part}")
    }
}

fn label_for(role: Role) -> LinkLabel {
    match role {
        Role::Supports => LinkLabel::Because,
        Role::Opposes => LinkLabel::But,
        Role::Rebuts => LinkLabel::However,
    }
}

/// The outline of the main tree followed by the parked fragments.
pub fn build_outline(doc: &Document, opts: &OutlineOptions) -> Vec<OutlineLine> {
    let roles = card_roles(doc);
    let hidden = |item: &Item| match item {
        Item::Box(b) => !opts.notes && doc.boxes.get(b).is_some_and(|n| n.kind == BoxKind::Note),
        Item::Card(_) => false,
    };
    let mut lines = Vec::with_capacity(doc.boxes.len());
    let mut stack: Vec<Task> = Vec::new();

    let parked: Vec<Task> = doc
        .parking
        .iter()
        .filter(|i| !hidden(i))
        .enumerate()
        .map(|(n, item)| {
            let number = format!("P{}", n + 1);
            match *item {
                Item::Box(id) => Task::Box {
                    id,
                    number,
                    depth: 0,
                    premise_of: None,
                    whole_card: false,
                },
                Item::Card(id) => Task::Card {
                    id,
                    number,
                    depth: 0,
                    whole_card: false,
                },
            }
        })
        .collect();
    let main = vec![Task::Box {
        id: doc.root,
        number: String::new(),
        depth: 0,
        premise_of: None,
        whole_card: false,
    }];

    for (tasks, is_parked) in [(main, false), (parked, true)] {
        stack.extend(tasks.into_iter().rev());
        while let Some(task) = stack.pop() {
            let mut next: Vec<Task> = Vec::new();
            match task {
                Task::Card {
                    id,
                    number,
                    depth,
                    whole_card,
                } => {
                    let Some(card) = doc.cards.get(&id) else {
                        continue;
                    };
                    let many = card.premises.len() > 1;
                    for (k, &pid) in card.premises.iter().enumerate() {
                        next.push(Task::Box {
                            id: pid,
                            number: if many {
                                format!("{number}{}", letter(k))
                            } else {
                                number.clone()
                            },
                            depth,
                            premise_of: Some(id),
                            whole_card,
                        });
                    }
                    for (k, &child) in card.children.iter().enumerate() {
                        next.push(Task::Card {
                            id: child,
                            number: format!("{number}.w{}", k + 1),
                            depth: depth + 1,
                            whole_card: true,
                        });
                    }
                }
                Task::Box {
                    id,
                    number,
                    depth,
                    premise_of,
                    whole_card,
                } => {
                    let Some(node) = doc.boxes.get(&id) else {
                        continue;
                    };
                    let (label, kind) = match (&node.kind, premise_of) {
                        (_, Some(card)) => {
                            let role = roles.get(&card).copied().unwrap_or(Role::Opposes);
                            let strength = doc.cards.get(&card).and_then(|c| c.strength);
                            let kind = LineKind::Premise {
                                card,
                                role,
                                strength,
                            };
                            (label_for(role), kind)
                        }
                        (BoxKind::Claim, None) => (LinkLabel::None, LineKind::Claim),
                        (BoxKind::Source { source_type }, None) => {
                            (LinkLabel::None, LineKind::Source(source_type.clone()))
                        }
                        (BoxKind::Note, None) => (LinkLabel::None, LineKind::Note),
                        (BoxKind::Category | BoxKind::Premise, None) => {
                            (LinkLabel::None, LineKind::Category)
                        }
                    };
                    let mut count = 0;
                    for item in display_children(doc, id).iter().filter(|i| !hidden(i)) {
                        let numbered = match *item {
                            Item::Card(_) => true,
                            Item::Box(b) => doc
                                .boxes
                                .get(&b)
                                .is_some_and(|n| n.kind == BoxKind::Category),
                        };
                        let child_number = if numbered {
                            count += 1;
                            join(&number, &count.to_string())
                        } else {
                            String::new()
                        };
                        next.push(match *item {
                            Item::Card(c) => Task::Card {
                                id: c,
                                number: child_number,
                                depth: depth + 1,
                                whole_card: false,
                            },
                            Item::Box(b) => Task::Box {
                                id: b,
                                number: child_number,
                                depth: depth + 1,
                                premise_of: None,
                                whole_card: false,
                            },
                        });
                    }
                    lines.push(OutlineLine {
                        item: Item::Box(id),
                        depth,
                        number,
                        label,
                        text: node.text.clone(),
                        kind,
                        whole_card,
                        parked: is_parked,
                    });
                }
            }
            stack.extend(next.into_iter().rev());
        }
    }
    lines
}

/// The outline number of every numbered box ("1", "1.2a", "P1"), exactly
/// as [`build_outline`] numbers it with the default options. The claim,
/// sources and notes have none.
pub fn box_numbers(doc: &Document) -> HashMap<NodeId, String> {
    build_outline(doc, &OutlineOptions::default())
        .into_iter()
        .filter_map(|line| match line.item {
            Item::Box(id) if !line.number.is_empty() => Some((id, line.number)),
            _ => None,
        })
        .collect()
}

/// The outline as indented plain text, two spaces per depth. Further lines
/// of a multi-line box text are indented like its first line; empty parts
/// (no number, no label, no text) leave no stray spaces. The words are
/// passed in so the core stays free of translations.
pub fn outline_plain_text(
    lines: &[OutlineLine],
    opts: &OutlineOptions,
    label_word: &dyn Fn(LinkLabel) -> String,
    source_word: &dyn Fn(&SourceType) -> String,
) -> String {
    let mut out = String::new();
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let indent = "  ".repeat(line.depth);
        let mut parts: Vec<String> = Vec::with_capacity(3);
        if let LineKind::Source(source_type) = &line.kind {
            parts.push(format!("[{}]", source_word(source_type)));
        } else {
            if opts.numbering {
                parts.push(line.number.clone());
            }
            if opts.labels && line.label != LinkLabel::None {
                parts.push(label_word(line.label));
            }
        }
        let mut text = line.text.lines();
        parts.push(text.next().unwrap_or_default().to_owned());
        parts.retain(|p| !p.is_empty());
        out.push_str(&indent);
        out.push_str(&parts.join(" "));
        for more in text {
            out.push('\n');
            if !more.is_empty() {
                out.push_str(&indent);
                out.push_str(more);
            }
        }
    }
    out
}
