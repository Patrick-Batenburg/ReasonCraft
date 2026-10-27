//! Argdown lines to a syntax tree: top-level elements with the relation
//! lines nested under them, and the premise-conclusion structures (PCS)
//! of arguments.

use super::lex::{Element, Line, LineKind, Rel, lex};
use super::report::{ArgdownError, MAX_NESTING};

/// What an entry of the tree is.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum EntryKind {
    Heading(String),
    /// A top-level statement or argument.
    Top,
    Relation(Rel),
    /// A numbered statement of a PCS.
    Premise,
    /// An inference line of a PCS; `rule`: it named an inference rule.
    Inference {
        rule: bool,
    },
}

/// One line of the tree.
#[derive(Clone, Debug)]
pub(super) struct Entry {
    pub line: usize,
    pub kind: EntryKind,
    /// The statement or argument on the line (empty for headings and
    /// inference lines).
    pub elem: Element,
    /// The line it hangs under: a relation's parent, a PCS step's
    /// argument.
    pub parent: Option<usize>,
    /// The relation lines under it.
    pub children: Vec<usize>,
    /// For an argument: its PCS steps, in order.
    pub steps: Vec<usize>,
}

/// The syntax tree. Entries are stored in file order.
#[derive(Debug, Default)]
pub(super) struct Tree {
    /// The frontmatter's lines.
    pub front: Vec<String>,
    pub entries: Vec<Entry>,
    /// The headings and top-level elements, in order.
    pub tops: Vec<usize>,
}

/// Parse `src`; fails only when relations nest deeper than
/// [`MAX_NESTING`].
pub(super) fn parse(src: &str) -> Result<Tree, ArgdownError> {
    let (front, lines) = lex(src);
    let mut p = Parser {
        tree: Tree {
            front,
            ..Tree::default()
        },
        ..Parser::default()
    };
    for line in lines {
        p.line(line)?;
    }
    Ok(p.tree)
}

#[derive(Default)]
struct Parser {
    tree: Tree,
    /// The lines relations can hang under, with their indentation; the
    /// first is the current top-level element or PCS step.
    stack: Vec<(usize, usize)>,
    /// The argument whose PCS is being read.
    pcs_owner: Option<usize>,
    /// The last top-level line, when it is an argument (a PCS after it is
    /// its PCS).
    last_argument: Option<usize>,
    /// The line plain text continues, if the line before was an element.
    continued: Option<usize>,
    /// The inference line of a rule written over several lines (`--`,
    /// text lines, `--`), while it is open.
    open_rule: Option<usize>,
}

impl Parser {
    fn push(
        &mut self,
        line: usize,
        kind: EntryKind,
        elem: Element,
        parent: Option<usize>,
    ) -> usize {
        self.tree.entries.push(Entry {
            line,
            kind,
            elem,
            parent,
            children: Vec::new(),
            steps: Vec::new(),
        });
        self.tree.entries.len() - 1
    }

    fn line(&mut self, line: Line) -> Result<(), ArgdownError> {
        let Line {
            number,
            indent,
            kind,
        } = line;
        // A rule over several lines is plain text up to its closing `--`.
        // A blank or structural line ends it too (and is read as usual), so
        // a stray `--` cannot swallow the rest of the file.
        if let Some(rule) = self.open_rule {
            match kind {
                LineKind::InferenceOpen | LineKind::Inference(_) => {
                    self.open_rule = None;
                    return Ok(());
                }
                LineKind::Text(_) => {
                    self.tree.entries[rule].kind = EntryKind::Inference { rule: true };
                    return Ok(());
                }
                _ => self.open_rule = None,
            }
        }
        let continued = self.continued.take();
        match kind {
            LineKind::Blank => {}
            LineKind::Heading(text) => {
                let id = self.push(number, EntryKind::Heading(text), Element::default(), None);
                self.tree.tops.push(id);
                (self.stack, self.pcs_owner, self.last_argument) = (Vec::new(), None, None);
            }
            LineKind::Element(elem) => self.top(number, indent, elem),
            LineKind::Text(text) => match continued {
                Some(entry) => {
                    let more = super::lex::element(&text);
                    let elem = &mut self.tree.entries[entry].elem;
                    if !more.text.is_empty() {
                        if !elem.text.is_empty() {
                            elem.text.push(' ');
                        }
                        elem.text.push_str(&more.text);
                    }
                    if elem.data.is_empty() {
                        elem.data = more.data;
                    }
                    self.continued = Some(entry);
                }
                None => self.top(number, indent, super::lex::element(&text)),
            },
            LineKind::Premise(elem) => {
                let id = self.step(number, indent, EntryKind::Premise, elem);
                self.continued = Some(id);
            }
            LineKind::Inference(rule) => {
                let kind = EntryKind::Inference {
                    rule: rule.is_some(),
                };
                self.step(number, indent, kind, Element::default());
            }
            LineKind::InferenceOpen => {
                let kind = EntryKind::Inference { rule: false };
                let id = self.step(number, indent, kind, Element::default());
                self.open_rule = Some(id);
            }
            LineKind::Relation(rel, elem) => {
                if self.stack.is_empty() {
                    // Nothing to hang under: read it as an element.
                    self.top(number, indent, elem);
                    return Ok(());
                }
                while self.stack.len() > 1 && self.stack.last().is_some_and(|&(i, _)| i >= indent) {
                    self.stack.pop();
                }
                if self.stack.len() > MAX_NESTING {
                    return Err(ArgdownError::TooDeep { line: number });
                }
                let parent = self.stack.last().map(|&(_, id)| id);
                let id = self.push(number, EntryKind::Relation(rel), elem, parent);
                if let Some(p) = parent {
                    self.tree.entries[p].children.push(id);
                }
                self.stack.push((indent, id));
                self.continued = Some(id);
            }
        }
        Ok(())
    }

    /// A top-level statement or argument.
    fn top(&mut self, number: usize, indent: usize, elem: Element) {
        let argument = elem.argument;
        let id = self.push(number, EntryKind::Top, elem, None);
        self.tree.tops.push(id);
        self.stack = vec![(indent, id)];
        self.pcs_owner = None;
        self.last_argument = argument.then_some(id);
        self.continued = Some(id);
    }

    /// A PCS step: it belongs to the PCS being read, or else to the
    /// argument just before it, or else to a new untitled argument.
    fn step(&mut self, number: usize, indent: usize, kind: EntryKind, elem: Element) -> usize {
        let owner = match self.pcs_owner.or(self.last_argument) {
            Some(owner) => owner,
            None => {
                let anonymous = Element {
                    argument: true,
                    defines: true,
                    ..Element::default()
                };
                let id = self.push(number, EntryKind::Top, anonymous, None);
                self.tree.tops.push(id);
                id
            }
        };
        self.pcs_owner = Some(owner);
        self.last_argument = None;
        let id = self.push(number, kind, elem, Some(owner));
        self.tree.entries[owner].steps.push(id);
        self.stack = vec![(indent, id)];
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nests_relations_and_pcs() {
        let t = parse("[A]: a\n    <+ [B]: b\n        <- [C]\n    <+ <R>\n\n<R>\n\n(1) [P]: p\n    <- <O>\n----\n(2) [A]\n").unwrap();
        let tops: Vec<usize> = t.tops.clone();
        assert_eq!(tops.len(), 2);
        let a = &t.entries[tops[0]];
        assert_eq!(a.children.len(), 2);
        assert_eq!(t.entries[a.children[0]].children.len(), 1);
        let r = &t.entries[tops[1]];
        assert_eq!(r.steps.len(), 3);
        assert_eq!(t.entries[r.steps[0]].children.len(), 1);
    }

    #[test]
    fn too_deep_is_refused() {
        let mut s = String::from("[A]\n");
        for d in 1..=MAX_NESTING + 1 {
            s.push_str(&format!("{}<+ [B{d}]\n", "    ".repeat(d)));
        }
        assert!(matches!(parse(&s), Err(ArgdownError::TooDeep { line: 66 })));
        let ok: String = s
            .lines()
            .take(MAX_NESTING + 1)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(parse(&ok).is_ok());
    }

    #[test]
    fn multi_line_rule_and_continuation() {
        let t = parse("<A>\n(1) one\ntwo\n--\nModus ponens\n--\n(2) three\n").unwrap();
        let a = &t.entries[t.tops[0]];
        assert_eq!(a.steps.len(), 3);
        assert_eq!(t.entries[a.steps[0]].elem.text, "one two");
        assert_eq!(
            t.entries[a.steps[1]].kind,
            EntryKind::Inference { rule: true }
        );
    }
}
