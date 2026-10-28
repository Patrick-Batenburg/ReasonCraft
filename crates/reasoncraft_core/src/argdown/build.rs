//! Argdown to a map, step two: lay the graph of [`super::import`] out as
//! a document. The root is placed first with everything that hangs under
//! it; the other top-level trees follow in the parking area.
//!
//! Each statement and argument is placed once with what hangs under it.
//! Met again elsewhere it becomes a copy of its text; met again among its
//! own ancestors, the relation is left out (it would close a cycle).

use std::collections::HashMap;

mod nodes;
use nodes::new_item;

use super::fields;
use super::graph::{Edge, Graph, Node, Tgt};
use super::lex::Op;
use super::report::{ArgdownError, ImportNoteKind as K, ImportReport};
use crate::ids::{Item, NodeId, Parent};
use crate::model::{BoxKind, BoxNode, CardKind, Document};
use crate::rules::{NewItem, can_attach};

/// The map laid out from `g`.
pub(super) fn build(g: &Graph, report: &mut ImportReport) -> Result<Document, ArgdownError> {
    if g.stmts.is_empty() && g.args.is_empty() {
        return Err(ArgdownError::Empty);
    }
    let root = super::graph::root(g);
    let mut text = root
        .and_then(|s| g.stmts[s].text.clone())
        .unwrap_or_default();
    if let Some(s) = root
        && nodes::cut_text(&mut text)
    {
        report.note(g.stmts[s].line, K::TextCut);
    }
    let mut doc = Document::new(&text);
    doc.meta.title = g.title.clone();
    doc.meta.author = g.author.clone();
    doc.meta.language = g.language.clone();
    let mut b = Builder {
        g,
        report,
        placed_s: vec![false; g.stmts.len()],
        placed_a: vec![false; g.args.len()],
        doc,
        chains: Vec::new(),
        tasks: Vec::new(),
        too_many: false,
        refused: HashMap::new(),
    };
    if let Some(s) = root {
        let id = b.doc.root;
        b.placed_s[s] = true;
        b.apply_data(id, s);
        let chain = b.chain(Node::S(s), None);
        b.tasks.push(Task {
            tgt: Tgt::S(s),
            parent: Parent::Box(id),
            chain,
        });
        b.run();
    }
    for &node in &g.parked_tops {
        b.park(node);
    }
    // Then whatever is left: whole trees first, then the rest.
    let mut rest: Vec<(usize, Node)> = (g.stmts.iter().enumerate())
        .filter(|(_, s)| !s.premise && !s.dropped())
        .map(|(k, s)| (s.line, Node::S(k)))
        .chain(g.args.iter().enumerate().map(|(k, a)| (a.line, Node::A(k))))
        .collect();
    rest.sort_by_key(|&(line, _)| line);
    for pass_sinks in [true, false] {
        for &(_, node) in &rest {
            if g.has_out.contains(&node) != pass_sinks {
                b.park(node);
            }
        }
    }
    if b.too_many {
        return Err(ArgdownError::TooManyBoxes);
    }
    Ok(b.doc)
}

/// Place what points at `tgt` under `parent`.
struct Task {
    tgt: Tgt,
    parent: Parent,
    /// The path down to `tgt` (an index into `Builder::chains`).
    chain: usize,
}

struct Builder<'a> {
    g: &'a Graph,
    report: &'a mut ImportReport,
    doc: Document,
    placed_s: Vec<bool>,
    placed_a: Vec<bool>,
    /// Paths from the top: a node and the path above it.
    chains: Vec<(Node, Option<usize>)>,
    /// Work still to do, last first.
    tasks: Vec<Task>,
    too_many: bool,
    /// Items whose place in the file was refused, with that line.
    refused: HashMap<Node, usize>,
}

impl Builder<'_> {
    fn chain(&mut self, node: Node, above: Option<usize>) -> usize {
        self.chains.push((node, above));
        self.chains.len() - 1
    }

    fn on_path(&self, chain: usize, node: Node) -> bool {
        let mut cur = Some(chain);
        while let Some(c) = cur {
            let (n, above) = self.chains[c];
            if n == node {
                return true;
            }
            cur = above;
        }
        false
    }

    fn run(&mut self) {
        while let Some(task) = self.tasks.pop() {
            if self.too_many {
                return;
            }
            let edges = self.g.incoming.get(&task.tgt).cloned().unwrap_or_default();
            let mut next = Vec::new();
            for edge in edges {
                if self.too_many {
                    return;
                }
                // Only a placed source can be an ancestor: the walk up the
                // path is skipped for all the others.
                let placed = match edge.src {
                    Node::S(x) => self.placed_s[x],
                    Node::A(a) => self.placed_a[a],
                };
                if placed && self.on_path(task.chain, edge.src) {
                    self.report.note(edge.line, K::CycleBroken);
                    continue;
                }
                self.child(task.parent, edge, task.chain, &mut next);
            }
            self.tasks.extend(next.into_iter().rev());
        }
    }

    /// Place the source of `edge` under `parent`.
    fn child(&mut self, parent: Parent, edge: Edge, chain: usize, next: &mut Vec<Task>) {
        match edge.src {
            Node::S(x) => {
                let special = fields::special_kind(&self.g.stmts[x].data);
                if let Some(kind) = special.filter(|_| matches!(parent, Parent::Box(_)))
                    && can_attach(&self.doc, parent, new_item(&kind))
                {
                    self.statement_box(x, kind, parent, Some(chain), edge.line, next);
                    return;
                }
                let kind = match edge.op {
                    Op::Support => CardKind::Reason,
                    _ => CardKind::Objection,
                };
                if !can_attach(&self.doc, parent, NewItem::Card(kind.clone())) {
                    self.refused.entry(edge.src).or_insert(edge.line);
                    return;
                }
                if let Some(p) = self.premise(x, edge.line, chain, next) {
                    self.new_card(kind, vec![p], None, parent);
                }
            }
            Node::A(a) => {
                let kind = fields::card_kind(&self.g.args[a], Some(edge.op));
                if can_attach(&self.doc, parent, NewItem::Card(kind.clone())) {
                    self.arg_card(a, kind, parent, Some(chain), edge.line, next);
                } else {
                    self.refused.entry(edge.src).or_insert(edge.line);
                }
            }
        }
    }

    /// A box of `kind` for statement `x` under `parent`, with what hangs
    /// under it queued (or a copy of its text when it is placed already).
    fn statement_box(
        &mut self,
        x: usize,
        kind: BoxKind,
        parent: Parent,
        chain: Option<usize>,
        line: usize,
        next: &mut Vec<Task>,
    ) {
        let fresh = !self.claim_s(x, line);
        let Some(id) = self.new_box(kind.clone(), fresh.then_some(x), x) else {
            return;
        };
        self.attach(parent, Item::Box(id));
        if fresh && kind != BoxKind::Note {
            let chain = self.chain(Node::S(x), chain);
            let parent = Parent::Box(id);
            next.push(Task {
                tgt: Tgt::S(x),
                parent,
                chain,
            });
        }
    }

    /// Mark statement `x` placed; true (and a note) when it already was.
    fn claim_s(&mut self, x: usize, line: usize) -> bool {
        let copy = self.placed_s[x];
        if copy {
            self.report.note(line, K::SharedStatementCopied);
        }
        self.placed_s[x] = true;
        copy
    }

    /// A premise box for statement `x`, with what hangs under it queued
    /// (or a copy of its text when it is placed already).
    fn premise(
        &mut self,
        x: usize,
        line: usize,
        chain: usize,
        next: &mut Vec<Task>,
    ) -> Option<NodeId> {
        let fresh = !self.claim_s(x, line);
        let id = self.new_box(BoxKind::Premise, fresh.then_some(x), x)?;
        if fresh {
            let chain = self.chain(Node::S(x), Some(chain));
            let parent = Parent::Box(id);
            next.push(Task {
                tgt: Tgt::S(x),
                parent,
                chain,
            });
        }
        Some(id)
    }

    /// The card of argument `a` under `parent`: its PCS premises, else one
    /// premise with its description or title. Met again, it is a copy of
    /// its premises' texts.
    fn arg_card(
        &mut self,
        a: usize,
        kind: CardKind,
        parent: Parent,
        chain: Option<usize>,
        line: usize,
        next: &mut Vec<Task>,
    ) {
        let g = self.g;
        let arg = &g.args[a];
        let copy = self.placed_a[a];
        if copy {
            self.report.note(line, K::SharedStatementCopied);
        }
        self.placed_a[a] = true;
        let chain = self.chain(Node::A(a), chain);
        let mut premises = Vec::new();
        if arg.premises.is_empty() {
            let text = arg.desc.clone().or_else(|| arg.title.clone());
            let Some(text) = text.filter(|t| !t.trim().is_empty()) else {
                self.report.note(arg.line, K::EmptyArgument);
                return;
            };
            let id = NodeId::new();
            if !self.insert_box(BoxNode::new(id, BoxKind::Premise, &text), &text, arg.line) {
                return;
            }
            premises.push(Some(id));
        }
        for &p in &arg.premises {
            let made = if copy {
                self.new_box(BoxKind::Premise, None, p)
            } else {
                self.premise(p, line, chain, next)
            };
            premises.push(made);
        }
        let Some(premises) = premises.into_iter().collect::<Option<Vec<_>>>() else {
            return;
        };
        let strength = fields::strength(&arg.data);
        let id = self.new_card(kind, premises, strength, parent);
        if !copy {
            let parent = Parent::Card(id);
            next.push(Task {
                tgt: Tgt::Card(a),
                parent,
                chain,
            });
        }
    }

    /// Place a top-level item in the parking area, unless placed already.
    fn park(&mut self, node: Node) {
        let mut next = Vec::new();
        match node {
            Node::S(x)
                if !self.placed_s[x] && !self.g.stmts[x].dropped() && !self.g.stmts[x].premise =>
            {
                let kind = fields::special_kind(&self.g.stmts[x].data).unwrap_or(BoxKind::Claim);
                self.statement_box(x, kind, Parent::Parking, None, 0, &mut next);
            }
            Node::A(a) if !self.placed_a[a] => {
                let kind = fields::card_kind(&self.g.args[a], None);
                let line = self.g.args[a].line;
                self.arg_card(a, kind, Parent::Parking, None, line, &mut next);
            }
            _ => return,
        }
        // It could not go where the file put it.
        if let Some(&line) = self.refused.get(&node) {
            self.report.note(line, K::MovedToParking);
        }
        self.tasks.extend(next.into_iter().rev());
        self.run();
    }
}
