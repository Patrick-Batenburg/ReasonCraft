//! What frontmatter and data `{…}` mean in a map: box kinds, ratings,
//! links, comments, notes and card kinds (the keys in `docs/argdown.md`).
//! Other keys are ignored.

use std::collections::HashSet;

use super::data::{self, Data, Value};
use super::graph::{Arg, Stmt};
use super::lex::Op;
use crate::link::safe_link;
use crate::meta::language_code;
use crate::model::{
    Acceptance, BoxKind, BoxNode, CardKind, CardStrength, Comment, SourceRating, SourceType,
    Verdict, now_rfc3339,
};

/// Title, author and language from the frontmatter's `key: value` lines.
/// A Dutch language tag (`nl`, `nl-NL`, …) reads as `nl`, anything else
/// as `en`.
pub(super) fn frontmatter(lines: &[String]) -> (String, String, String) {
    let (mut title, mut author, mut language) = (String::new(), String::new(), String::new());
    for line in lines {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = data::scalar(value);
        match key.trim() {
            "title" => title = value,
            "author" => author = value,
            "language" => language = value,
            _ => {}
        }
    }
    (title, author, language_code(language.trim()).to_owned())
}

/// A source, category or note box kind, from `source`, `category: true`
/// or `note: true`.
pub(super) fn special_kind(d: &Data) -> Option<BoxKind> {
    if let Some(t) = data::get(d, "source").and_then(Value::as_str) {
        return Some(BoxKind::Source {
            source_type: SourceType::from_name(t),
        });
    }
    if data::get(d, "category").is_some_and(Value::is_true) {
        return Some(BoxKind::Category);
    }
    data::get(d, "note")
        .is_some_and(Value::is_true)
        .then_some(BoxKind::Note)
}

/// Statement `st`'s acceptance, rating, link and comments on `node`,
/// where its kind takes them. Returns the texts of its notes, which the
/// caller adds as boxes (a note takes none).
pub(super) fn set_fields(node: &mut BoxNode, st: &Stmt) -> Vec<String> {
    let get = |key| data::get(&st.data, key);
    let text = |v: &Value| v.as_str().map(str::to_owned);
    if matches!(node.kind, BoxKind::Claim | BoxKind::Premise) {
        node.acceptance = match get("acceptance").and_then(Value::as_str) {
            Some("accept") => Some(Acceptance::Accept),
            Some("unsure") => Some(Acceptance::Unsure),
            Some("reject") => Some(Acceptance::Reject),
            _ => None,
        };
    }
    if matches!(node.kind, BoxKind::Source { .. }) {
        node.source_rating = match get("rating").and_then(Value::as_str) {
            Some("solid") => Some(SourceRating::Solid),
            Some("shaky") => Some(SourceRating::Shaky),
            Some("nil") => Some(SourceRating::Nil),
            _ => None,
        };
    }
    let link = get("link").and_then(text).or_else(|| st.link.clone());
    node.link = link.as_deref().and_then(safe_link);
    node.comments = match get("comments") {
        Some(Value::List(l)) => l.iter().filter_map(comment).collect(),
        _ => Vec::new(),
    };
    match get("notes") {
        Some(Value::List(l)) if node.kind != BoxKind::Note => l.iter().filter_map(text).collect(),
        _ => Vec::new(),
    }
}

/// A comment from data `{author: …, text: …, verdict: …}`.
fn comment(v: &Value) -> Option<Comment> {
    let Value::Map(m) = v else {
        return None;
    };
    let field = |k| data::get(m, k).and_then(Value::as_str).map(str::to_owned);
    let verdict = match field("verdict").as_deref() {
        Some("positive") => Some(Verdict::Positive),
        Some("needs_work") => Some(Verdict::NeedsWork),
        Some("problem") => Some(Verdict::Problem),
        _ => None,
    };
    Some(Comment {
        id: uuid::Uuid::new_v4(),
        author: field("author").unwrap_or_default(),
        text: field("text").unwrap_or_default(),
        verdict,
        created: now_rfc3339(),
    })
}

/// A card's kind: its `kind` data, else its conclusion's shape, else an
/// `R1`/`O1` title, else the relation it was met by (`op`; none for a
/// parked card).
pub(super) fn card_kind(arg: &Arg, op: Option<Op>) -> CardKind {
    match data::get(&arg.data, "kind").and_then(Value::as_str) {
        Some("reason") => return CardKind::Reason,
        Some("objection") => return CardKind::Objection,
        _ => {}
    }
    if let Some(kind) = &arg.shape {
        return kind.clone();
    }
    let mut title = arg.title.as_deref().unwrap_or_default().chars();
    let first = title.next();
    let rest: String = title.collect();
    if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()) {
        match first {
            Some('R') => return CardKind::Reason,
            Some('O') => return CardKind::Objection,
            _ => {}
        }
    }
    match op {
        Some(Op::Support) | None => CardKind::Reason,
        Some(_) => CardKind::Objection,
    }
}

/// A card's `strength` data.
pub(super) fn strength(d: &Data) -> Option<CardStrength> {
    match data::get(d, "strength").and_then(Value::as_str) {
        Some("strong") => Some(CardStrength::Strong),
        Some("weak") => Some(CardStrength::Weak),
        Some("nil") => Some(CardStrength::Nil),
        _ => None,
    }
}

/// Add `more`'s keys that `data` (whose keys are `keys`) lacks.
pub(super) fn merge(data: &mut Data, keys: &mut HashSet<String>, more: &Data) {
    for (k, v) in more {
        if keys.insert(k.clone()) {
            data.push((k.clone(), v.clone()));
        }
    }
}

/// For `The inference in @<T> holds.` (or `does not hold.`): `T` and
/// whether it holds.
pub(super) fn inference_mention(text: &str) -> Option<(String, bool)> {
    let rest = text.trim().strip_prefix("The inference in @<")?;
    let (title, tail) = rest.split_once('>')?;
    let holds = match tail.trim() {
        "holds." => true,
        "does not hold." => false,
        _ => return None,
    };
    Some((super::text::unescape(title), holds))
}
