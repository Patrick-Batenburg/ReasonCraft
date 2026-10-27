//! The graph an Argdown file is read into: statement and argument classes
//! (one per title) and the relations between them, and the choice of the
//! map's root.

use std::collections::{HashMap, HashSet};

use super::data::{self, Data};
use super::lex::Op;
use crate::model::CardKind;

/// A statement or an argument class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Node {
    S(usize),
    A(usize),
}

/// What a relation points at: a statement, or an argument as a whole
/// card (its inference).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Tgt {
    S(usize),
    Card(usize),
}

/// A relation: `src` supports or attacks `dst`.
#[derive(Clone, Copy, Debug)]
pub(super) struct Edge {
    pub src: Node,
    pub op: Op,
    pub line: usize,
}

/// A statement class: every occurrence of one title (or one untitled
/// statement).
#[derive(Debug, Default)]
pub(super) struct Stmt {
    pub title: Option<String>,
    /// The first definition's text.
    pub text: Option<String>,
    pub link: Option<String>,
    pub data: Data,
    /// The keys of `data`.
    pub keys: HashSet<String>,
    pub line: usize,
    /// Written as a top-level statement somewhere.
    pub top: bool,
    /// A premise of some argument's PCS.
    pub premise: bool,
    /// An intermediate conclusion, or a final one that only carried
    /// relations or a placeholder.
    pub consumed: bool,
}

impl Stmt {
    /// Left out of the map: only ever a conclusion that was left out, not
    /// also a top-level statement or a premise.
    pub fn dropped(&self) -> bool {
        self.consumed && !self.top && !self.premise
    }
}

/// An argument class.
#[derive(Debug, Default)]
pub(super) struct Arg {
    pub title: Option<String>,
    pub desc: Option<String>,
    pub data: Data,
    /// The keys of `data`.
    pub keys: HashSet<String>,
    pub line: usize,
    pub defined: bool,
    pub premises: Vec<usize>,
    /// Its PCS was read (a second one is ignored).
    pub pcs_read: bool,
    /// The kind its conclusion's shape tells.
    pub shape: Option<CardKind>,
}

/// The graph of a file.
#[derive(Debug, Default)]
pub(super) struct Graph {
    pub title: String,
    pub author: String,
    pub language: String,
    pub stmts: Vec<Stmt>,
    pub args: Vec<Arg>,
    /// The relations into each target, in file order, one per source.
    pub incoming: HashMap<Tgt, Vec<Edge>>,
    /// Sources with a relation out of them.
    pub has_out: HashSet<Node>,
    /// The first line of the "Not connected" heading, if any.
    pub heading: Option<usize>,
    /// Top-level items after that heading, in order.
    pub parked_tops: Vec<Node>,
}

/// The main root: of the top-level statements and the statements not
/// related to anything, the one with the most relations pointing at it
/// (the first one on a tie). Only statements before a "Not connected"
/// heading count.
pub(super) fn root(g: &Graph) -> Option<usize> {
    let mut best: Option<(usize, usize)> = None;
    for (k, s) in g.stmts.iter().enumerate() {
        let special = ["source", "category", "note"]
            .iter()
            .any(|key| data::get(&s.data, key).is_some());
        let before_heading = g.heading.is_none_or(|h| s.line < h);
        let tree = s.top || !g.has_out.contains(&Node::S(k));
        if s.premise || s.dropped() || special || !before_heading || !tree {
            continue;
        }
        let score = g.incoming.get(&Tgt::S(k)).map_or(0, Vec::len);
        if best.is_none_or(|(_, b)| score > b) {
            best = Some((k, score));
        }
    }
    best.map(|(k, _)| k)
}
