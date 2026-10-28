//! Argdown to a map, step one: the syntax tree to a graph of statements
//! and arguments (equivalence classes by title) and the relations
//! between them. [`super::build`] then lays the graph out as a map.

use std::collections::{HashMap, HashSet};

use super::graph::{Arg, Edge, Graph, Node, Stmt, Tgt};

use super::data::{self, Value};
use super::lex::Op;
use super::parse::{EntryKind, Tree, parse};
use super::report::{ArgdownError, ImportNoteKind as K, ImportReport, MAX_ARGDOWN_BYTES};
use super::text::clean;
use crate::lang::not_connected_forms;
use crate::link::safe_link;
use crate::model::{CardKind, Document};

/// Read an Argdown text as a map, with a report of what could not be
/// kept as written. See `docs/argdown.md`.
pub fn from_argdown(src: &str) -> Result<(Document, ImportReport), ArgdownError> {
    if src.len() > MAX_ARGDOWN_BYTES {
        return Err(ArgdownError::TooLarge);
    }
    let tree = parse(src)?;
    let mut report = ImportReport::default();
    let graph = Reader::new(&tree, &mut report).read();
    let mut doc = super::build::build(&graph, &mut report)?;
    // Comments, frontmatter and links from data keep the caps a file's
    // open applies, so an imported map is not cut when it is opened again.
    if doc.cap_texts() {
        report.note(1, super::ImportNoteKind::TextCut);
    }
    // A bug guard: the build only makes what the rules allow. Tests (debug
    // builds) stop here; a release build refuses the file instead.
    if let Err(e) = doc.validate() {
        debug_assert!(false, "an imported map is invalid: {e}");
        return Err(ArgdownError::Empty);
    }
    Ok((doc, report))
}

struct Reader<'a> {
    tree: &'a Tree,
    report: &'a mut ImportReport,
    g: Graph,
    /// The class of each entry with an element.
    class: Vec<Option<Node>>,
    by_title: HashMap<(bool, String), usize>,
    /// The final conclusion entries, with their argument.
    conclusions: HashMap<usize, usize>,
    /// Final conclusion entries that only carry relations or a placeholder.
    carriers: HashSet<usize>,
    /// The relations added so far, as (target, source).
    seen: HashSet<(Tgt, Node)>,
    /// Tags were noted (once is enough).
    tags_noted: bool,
}

impl<'a> Reader<'a> {
    fn new(tree: &'a Tree, report: &'a mut ImportReport) -> Self {
        Reader {
            tree,
            report,
            g: Graph::default(),
            class: vec![None; tree.entries.len()],
            by_title: HashMap::new(),
            conclusions: HashMap::new(),
            carriers: HashSet::new(),
            seen: HashSet::new(),
            tags_noted: false,
        }
    }

    fn read(mut self) -> Graph {
        (self.g.title, self.g.author, self.g.language) =
            super::fields::frontmatter(&self.tree.front);
        for i in 0..self.tree.entries.len() {
            self.intern(i);
        }
        for s in &mut self.g.stmts {
            if s.text.is_none() {
                let title = s.title.clone().unwrap_or_default();
                s.text = Some(title.clone());
                self.report.note(s.line, K::UnknownReference(title));
            }
        }
        for a in &self.g.args {
            if !a.defined {
                let title = a.title.clone().unwrap_or_default();
                self.report.note(a.line, K::UnknownReference(title));
            }
        }
        let tree = self.tree;
        for &top in &tree.tops {
            self.pcs(top);
        }
        for i in 0..self.tree.entries.len() {
            self.edges(i);
        }
        self.parking();
        self.g
    }

    /// Give entry `i`'s element its class, merging its text and data in.
    fn intern(&mut self, i: usize) {
        let tree = self.tree;
        let entry = &tree.entries[i];
        if matches!(
            entry.kind,
            EntryKind::Heading(_) | EntryKind::Inference { .. }
        ) {
            return;
        }
        let e = &entry.elem;
        let line = entry.line;
        let key = e.title.clone().map(|t| (e.argument, t));
        let existing = key.as_ref().and_then(|k| self.by_title.get(k)).copied();
        let cleaned = e.defines.then(|| clean(&e.text));
        let data_link = data::get(&e.data, "link").and_then(Value::as_str);
        let md_link = cleaned.as_ref().and_then(|c| c.link.as_deref());
        if [data_link, md_link]
            .into_iter()
            .flatten()
            .any(|l| !l.trim().is_empty() && safe_link(l).is_none())
        {
            self.report.note(line, K::LinkDropped);
        }
        if cleaned.as_ref().is_some_and(|c| c.tags) && !self.tags_noted {
            self.tags_noted = true;
            self.report.note(line, K::TagsDropped);
        }
        let node = if e.argument {
            let id = existing.unwrap_or_else(|| {
                self.g.args.push(Arg {
                    title: e.title.clone(),
                    line,
                    ..Arg::default()
                });
                self.g.args.len() - 1
            });
            let a = &mut self.g.args[id];
            a.defined |= e.defines || entry.kind == EntryKind::Top;
            match (&a.desc, cleaned.filter(|c| !c.text.is_empty())) {
                (None, Some(c)) => a.desc = Some(c.text),
                (Some(d), Some(c)) if *d != c.text => {
                    self.report.note(line, K::ExtraEquivalentWording);
                }
                _ => {}
            }
            super::fields::merge(&mut a.data, &mut a.keys, &e.data);
            Node::A(id)
        } else {
            let id = existing.unwrap_or_else(|| {
                self.g.stmts.push(Stmt {
                    title: e.title.clone(),
                    line,
                    ..Stmt::default()
                });
                self.g.stmts.len() - 1
            });
            let s = &mut self.g.stmts[id];
            s.top |= entry.kind == EntryKind::Top;
            if let Some(c) = cleaned {
                match &s.text {
                    None => s.text = Some(c.text),
                    Some(t) if *t != c.text && !c.text.is_empty() => {
                        self.report.note(line, K::ExtraEquivalentWording);
                    }
                    Some(_) => {}
                }
                if s.link.is_none() {
                    s.link = c.link;
                }
            }
            super::fields::merge(&mut s.data, &mut s.keys, &e.data);
            Node::S(id)
        };
        if let (Some(k), Node::S(id) | Node::A(id)) = (key, node) {
            self.by_title.insert(k, id);
        }
        self.class[i] = Some(node);
    }

    /// Read the PCS of top-level argument `top`: premises, intermediate
    /// conclusions, rules, and how the final conclusion is used.
    fn pcs(&mut self, top: usize) {
        let tree = self.tree;
        let entry = &tree.entries[top];
        let (Some(Node::A(a)), false) = (self.class[top], entry.steps.is_empty()) else {
            return;
        };
        if std::mem::replace(&mut self.g.args[a].pcs_read, true) {
            // A second PCS of the same argument is not read.
            self.report.note(entry.line, K::ExtraEquivalentWording);
            return;
        }
        let steps = &entry.steps;
        let mut after_inference = false;
        let mut premises = Vec::new();
        for (k, &step) in steps.iter().enumerate() {
            let e = &tree.entries[step];
            match (&e.kind, self.class[step]) {
                (EntryKind::Inference { rule }, _) => {
                    if *rule {
                        self.report.note(e.line, K::InferenceRuleDropped);
                    }
                    after_inference = true;
                }
                (_, Some(Node::S(s))) if after_inference && k + 1 == steps.len() => {
                    self.conclusion(a, step, s);
                }
                (_, Some(Node::S(s))) if after_inference => {
                    self.report.note(e.line, K::IntermediateConclusion);
                    self.g.stmts[s].consumed = true;
                    after_inference = false;
                }
                (_, Some(Node::S(s))) => premises.push(s),
                _ => {}
            }
        }
        for &p in &premises {
            self.g.stmts[p].premise = true;
        }
        self.g.args[a].premises = premises;
    }

    /// The final conclusion `step` (class `s`) of argument `a`.
    fn conclusion(&mut self, a: usize, step: usize, s: usize) {
        let tree = self.tree;
        let e = &tree.entries[step];
        let outgoing = e.children.iter().any(|&c| self.concludes_out(c));
        let placeholder = data::get(&e.elem.data, "target").and_then(Value::as_str) == Some("none");
        let inference = super::fields::inference_mention(&e.elem.text);
        if outgoing || placeholder || inference.is_some() {
            self.g.stmts[s].consumed = true;
            self.carriers.insert(step);
        }
        if outgoing {
            let attacks = e.children.iter().any(|&c| {
                self.concludes_out(c)
                    && matches!(tree.entries[c].kind, EntryKind::Relation(r) if r.op != Op::Support)
            });
            if attacks {
                self.g.args[a].shape = Some(CardKind::Objection);
            }
        }
        if let Some((title, holds)) = inference {
            let kind = if holds {
                CardKind::Reason
            } else {
                CardKind::Objection
            };
            self.g.args[a].shape = Some(kind);
            if let Some(&target) = self.by_title.get(&(true, title)) {
                let op = if holds { Op::Support } else { Op::Undercut };
                self.add(Node::A(a), Tgt::Card(target), op, e.line);
            }
        }
        self.conclusions.insert(step, a);
    }

    /// Whether relation entry `c` under a conclusion relates the
    /// argument to something (`->`, `- X`, `+>`, `_>`).
    fn concludes_out(&self, c: usize) -> bool {
        matches!(self.tree.entries[c].kind, EntryKind::Relation(r)
            if r.outgoing || (r.plain && r.op == Op::Attack))
    }

    /// The relations of entry `i`: its relation line, or its use as a
    /// final conclusion.
    fn edges(&mut self, i: usize) {
        let tree = self.tree;
        let e = &tree.entries[i];
        let line = e.line;
        if let Some(&a) = self.conclusions.get(&i) {
            if let Some(Node::S(s)) = self.class[i].filter(|_| !self.carriers.contains(&i)) {
                self.add(Node::A(a), Tgt::S(s), Op::Support, line);
            }
            return;
        }
        let EntryKind::Relation(rel) = e.kind else {
            return;
        };
        let (Some(p), Some(child)) = (e.parent, self.class[i]) else {
            return;
        };
        if rel.op == Op::Contradict {
            self.report.note(line, K::Contradiction);
            return;
        }
        let parent = &tree.entries[p];
        // The line above as a source and as a target.
        let (above, above_tgt) = match (&parent.kind, self.class[p]) {
            (EntryKind::Inference { .. }, _) => {
                let Some(Some(Node::A(a))) = parent.parent.map(|o| self.class[o]) else {
                    return;
                };
                (Node::A(a), Tgt::Card(a))
            }
            (_, Some(node)) => match self.conclusions.get(&p) {
                Some(&a) if self.concludes_out(i) => (Node::A(a), Tgt::Card(a)),
                _ => (node, tgt(node)),
            },
            _ => return,
        };
        if rel.outgoing || (self.conclusions.contains_key(&p) && self.concludes_out(i)) {
            self.add(above, tgt(child), rel.op, line);
        } else {
            self.add(child, above_tgt, rel.op, line);
        }
    }

    fn add(&mut self, src: Node, dst: Tgt, mut op: Op, line: usize) {
        if matches!(dst, Tgt::S(s) if self.g.stmts[s].dropped()) {
            self.report.note(line, K::RelationDropped);
            return;
        }
        if matches!(dst, Tgt::S(_)) && op == Op::Undercut {
            self.report.note(line, K::UndercutOnStatement);
            op = Op::Attack;
        }
        if tgt(src) == dst {
            self.report.note(line, K::CycleBroken);
            return;
        }
        if self.seen.insert((dst, src)) {
            let list = self.g.incoming.entry(dst).or_default();
            list.push(Edge { src, op, line });
            self.g.has_out.insert(src);
        }
    }

    /// Find the "Not connected" heading and the items after it.
    fn parking(&mut self) {
        let mut after = false;
        let tree = self.tree;
        for &top in &tree.tops {
            let e = &tree.entries[top];
            match (&e.kind, self.class[top]) {
                (EntryKind::Heading(h), _) if !after => {
                    if not_connected_forms().any(|n| n.eq_ignore_ascii_case(h)) {
                        after = true;
                        self.g.heading = Some(e.line);
                    }
                }
                (_, Some(node)) if after => self.g.parked_tops.push(node),
                _ => {}
            }
        }
    }
}

fn tgt(node: Node) -> Tgt {
    match node {
        Node::S(s) => Tgt::S(s),
        Node::A(a) => Tgt::Card(a),
    }
}
