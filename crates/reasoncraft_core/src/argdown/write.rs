//! Writing a document as Argdown text (the mapping in `docs/argdown.md`).

use super::titles::Names;
use crate::ids::{CardId, Item, NodeId, Parent};
use crate::model::{
    Acceptance, BoxKind, BoxNode, CardKind, CardStrength, Comment, Document, SourceRating, Verdict,
};
use crate::words::not_connected_heading;

/// One level of relation indentation.
const INDENT: &str = "    ";

/// The document as Argdown text: frontmatter, the main tree, its cards'
/// arguments and then the parking area under a "Not connected" heading.
/// Lines end in `\n`.
pub fn to_argdown(doc: &Document) -> String {
    let w = Writer {
        doc,
        names: Names::new(doc),
    };
    let mut blocks = vec![w.frontmatter(), w.statement_block(doc.root)];
    blocks.extend(w.names.main.iter().map(|&c| w.argument(c)));
    if !doc.parking.is_empty() {
        blocks.push(format!("# {}", not_connected_heading(&doc.meta.language)));
        for (&item, cards) in doc.parking.iter().zip(&w.names.parked) {
            blocks.push(match item {
                Item::Box(id) => w.statement_block(id),
                Item::Card(id) => w.argument(id),
            });
            blocks.extend(cards.iter().map(|&c| w.argument(c)));
        }
    }
    let mut out = blocks.join("\n\n");
    out.push('\n');
    out
}

struct Writer<'a> {
    doc: &'a Document,
    names: Names,
}

impl Writer<'_> {
    fn frontmatter(&self) -> String {
        let meta = &self.doc.meta;
        format!(
            "===\ntitle: {}\nauthor: {}\nlanguage: {}\ngenerator: {}\n===",
            front(&meta.title),
            front(&meta.author),
            front(&meta.language),
            front("ReasonCraft"),
        )
    }

    /// A top-level statement and the relations under it.
    fn statement_block(&self, id: NodeId) -> String {
        let mut lines = vec![self.statement(id, "")];
        self.relations(id, 1, &mut lines);
        lines.join("\n")
    }

    /// A card's argument: its title with its kind (and strength), its
    /// premises with what hangs under them, the inference line with the
    /// whole-card children, and the conclusion (a placeholder marked
    /// `{target: none}` for a parked card, which targets nothing).
    fn argument(&self, id: CardId) -> String {
        let Some(card) = self.doc.cards.get(&id) else {
            return String::new();
        };
        let kind = match card.kind {
            CardKind::Reason => "reason",
            CardKind::Objection => "objection",
        };
        let mut header = format!("<{}> {{kind: {kind}", self.names.of_card(id));
        if let Some(s) = card.strength {
            header.push_str(&format!(", strength: {}", strength(s)));
        }
        header.push('}');
        let mut lines = vec![header, String::new()];
        for (k, &p) in card.premises.iter().enumerate() {
            lines.push(self.statement(p, &format!("({}) ", k + 1)));
            self.relations(p, 1, &mut lines);
        }
        lines.push("----".to_owned());
        for &child in &card.children {
            let Some(c) = self.doc.cards.get(&child) else {
                continue;
            };
            let sign = match c.kind {
                CardKind::Reason => "<+",
                CardKind::Objection => "<_",
            };
            lines.push(format!("{INDENT}{sign} <{}>", self.names.of_card(child)));
        }
        let n = card.premises.len() + 1;
        match (self.names.targets.get(&id), &card.kind) {
            (Some(Parent::Box(b)), CardKind::Reason) => {
                lines.push(format!("({n}) [{}]", self.names.of_box(*b)));
            }
            (Some(Parent::Box(b)), CardKind::Objection) => {
                let title = self.names.of_box(*b);
                lines.push(format!("({n}) Not: {}", escape(title)));
                lines.push(format!("{INDENT}-> [{title}]"));
            }
            (Some(Parent::Card(c)), kind) => {
                let holds = match kind {
                    CardKind::Reason => "holds",
                    CardKind::Objection => "does not hold",
                };
                let target = self.names.of_card(*c);
                lines.push(format!("({n}) The inference in @<{target}> {holds}."));
            }
            (Some(Parent::Parking) | None, _) => {
                lines.push(format!("({n}) No target. {{target: none}}"));
            }
        }
        lines.join("\n")
    }

    /// The relation lines under box `parent` at `depth`: one per card, per
    /// source and per category (with the categories under it). Notes are
    /// data of their owner, not lines.
    fn relations(&self, parent: NodeId, depth: usize, lines: &mut Vec<String>) {
        let Some(node) = self.doc.boxes.get(&parent) else {
            return;
        };
        let mut stack: Vec<(Item, usize)> =
            node.children.iter().rev().map(|&i| (i, depth)).collect();
        while let Some((item, depth)) = stack.pop() {
            let pad = INDENT.repeat(depth);
            match item {
                Item::Card(c) => {
                    let Some(card) = self.doc.cards.get(&c) else {
                        continue;
                    };
                    let sign = match card.kind {
                        CardKind::Reason => "<+",
                        CardKind::Objection => "<-",
                    };
                    lines.push(format!("{pad}{sign} <{}>", self.names.of_card(c)));
                }
                Item::Box(b) => {
                    let Some(child) = self.doc.boxes.get(&b) else {
                        continue;
                    };
                    match child.kind {
                        BoxKind::Note => continue,
                        BoxKind::Category => {
                            let below = child.children.iter().rev();
                            stack.extend(below.map(|&i| (i, depth + 1)));
                        }
                        _ => {}
                    }
                    lines.push(self.statement(b, &format!("{pad}<+ ")));
                }
            }
        }
    }

    /// `<prefix>[Title]: text {data}`.
    fn statement(&self, id: NodeId, prefix: &str) -> String {
        let Some(node) = self.doc.boxes.get(&id) else {
            return String::new();
        };
        let mut line = format!("{prefix}[{}]:", self.names.of_box(id));
        let text = escape(&node.text);
        if !text.is_empty() {
            line.push(' ');
            line.push_str(&text);
        }
        if let Some(data) = self.data(node) {
            line.push(' ');
            line.push_str(&data);
        }
        line
    }

    /// A statement's data, `{key: value, …}`, or `None` when it has none.
    fn data(&self, node: &BoxNode) -> Option<String> {
        let mut entries = Vec::new();
        match &node.kind {
            BoxKind::Category => entries.push("category: true".to_owned()),
            BoxKind::Source { source_type } => {
                entries.push(format!("source: {}", scalar(source_type.name())));
            }
            BoxKind::Note => entries.push("note: true".to_owned()),
            BoxKind::Claim | BoxKind::Premise => {}
        }
        if let Some(a) = node.acceptance {
            entries.push(format!("acceptance: {}", acceptance(a)));
        }
        if let Some(r) = node.source_rating {
            entries.push(format!("rating: {}", rating(r)));
        }
        if let Some(link) = &node.link {
            entries.push(format!("link: {}", quote(link)));
        }
        let notes: Vec<String> = node
            .children
            .iter()
            .filter_map(|i| match i {
                Item::Box(b) => self.doc.boxes.get(b),
                Item::Card(_) => None,
            })
            .filter(|n| n.kind == BoxKind::Note)
            .map(|n| quote(&n.text))
            .collect();
        if !notes.is_empty() {
            entries.push(format!("notes: [{}]", notes.join(", ")));
        }
        if !node.comments.is_empty() {
            let comments: Vec<String> = node.comments.iter().map(comment).collect();
            entries.push(format!("comments: [{}]", comments.join(", ")));
        }
        (!entries.is_empty()).then(|| format!("{{{}}}", entries.join(", ")))
    }
}

fn comment(c: &Comment) -> String {
    let mut out = format!("{{author: {}, text: {}", quote(&c.author), quote(&c.text));
    if let Some(v) = c.verdict {
        out.push_str(&format!(", verdict: {}", verdict(v)));
    }
    out.push('}');
    out
}

fn acceptance(a: Acceptance) -> &'static str {
    match a {
        Acceptance::Accept => "accept",
        Acceptance::Unsure => "unsure",
        Acceptance::Reject => "reject",
    }
}

fn strength(s: CardStrength) -> &'static str {
    match s {
        CardStrength::Strong => "strong",
        CardStrength::Weak => "weak",
        CardStrength::Nil => "nil",
    }
}

fn rating(r: SourceRating) -> &'static str {
    match r {
        SourceRating::Solid => "solid",
        SourceRating::Shaky => "shaky",
        SourceRating::Nil => "nil",
    }
}

fn verdict(v: Verdict) -> &'static str {
    match v {
        Verdict::Positive => "positive",
        Verdict::NeedsWork => "needs_work",
        Verdict::Problem => "problem",
    }
}

/// Box text as Argdown statement text: on one line (line breaks become
/// spaces), with a backslash before every character that would otherwise
/// mean something (`\ [ ] < > { } # @ * _`, and a `/` that would start a
/// comment), so it reads back exactly.
pub(super) fn escape(text: &str) -> String {
    let one_line = text
        .split(['\n', '\r'])
        .filter(|l| !l.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let mut out = String::with_capacity(one_line.len() + 8);
    let mut chars = one_line.chars().peekable();
    while let Some(c) = chars.next() {
        let comment_start = c == '/' && matches!(chars.peek(), Some('/' | '*'));
        if comment_start || "\\[]<>{}#@*_".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// A YAML double-quoted string. Line breaks and other control characters
/// are escaped, so data values keep them; so are braces, because Argdown
/// ends a data block at a `}` even inside quotes.
pub(super) fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '{' => out.push_str("\\u007B"),
            '}' => out.push_str("\\u007D"),
            c if c.is_control() => out.push_str(&format!("\\u{:04X}", u32::from(c))),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A frontmatter value: [`quote`]d, with `=` escaped too, so a value
/// never contains the `===` that ends the frontmatter.
fn front(s: &str) -> String {
    quote(s).replace('=', "\\u003D")
}

/// A plain YAML scalar when `s` is a simple name (`[a-z0-9_]`), quoted
/// otherwise.
fn scalar(s: &str) -> String {
    let simple = !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    if simple { s.to_owned() } else { quote(s) }
}
