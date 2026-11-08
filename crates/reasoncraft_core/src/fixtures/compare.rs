//! Whether two maps hold the same as far as Argdown carries it.

use crate::ids::{CardId, Item, NodeId};
use crate::model::{BoxKind, Document};

/// Whether `a` and `b` hold the same map as far as Argdown carries it
/// (`docs/argdown.md`): title, author, language, the tree and the parking
/// area with every box's kind, text, acceptance, rating, link and
/// comments (author, text, verdict), and every card's kind, strength,
/// premises and children, in order.
///
/// Ignored, because Argdown does not keep them: ids, images, widths,
/// comment dates, the links and comments of notes under a box, where notes
/// sit among their other siblings (the notes' own order counts), and the
/// spaces around a box's text (line breaks in it count as spaces).
pub fn structurally_equal(a: &Document, b: &Document) -> Result<(), String> {
    for (what, x, y) in [
        ("title", &a.meta.title, &b.meta.title),
        ("author", &a.meta.author, &b.meta.author),
        ("language", &a.meta.language, &b.meta.language),
    ] {
        if x != y {
            return Err(format!("{what}: {x:?} != {y:?}"));
        }
    }
    let mut cmp = Compare {
        a,
        b,
        todo: vec![(
            Item::Box(a.root),
            Item::Box(b.root),
            "root".to_owned(),
            true,
        )],
    };
    if a.parking.len() != b.parking.len() {
        return Err(format!(
            "parking: {} items != {}",
            a.parking.len(),
            b.parking.len()
        ));
    }
    for (k, (&x, &y)) in a.parking.iter().zip(&b.parking).enumerate() {
        cmp.todo.push((x, y, format!("parking[{k}]"), true));
    }
    // A work list, not recursion: maps can be thousands of levels deep.
    while let Some((x, y, at, whole)) = cmp.todo.pop() {
        cmp.item(x, y, &at, whole)?;
    }
    Ok(())
}

struct Compare<'a> {
    a: &'a Document,
    b: &'a Document,
    /// Pairs still to compare: items, where they are, and `whole`.
    todo: Vec<(Item, Item, String, bool)>,
}

impl Compare<'_> {
    /// Compare two items and all under them. `whole`: a note compares its
    /// link and comments too (a parked note keeps them).
    fn item(&mut self, x: Item, y: Item, at: &str, whole: bool) -> Result<(), String> {
        match (x, y) {
            (Item::Box(x), Item::Box(y)) => self.node(x, y, at, whole),
            (Item::Card(x), Item::Card(y)) => self.card(x, y, at),
            _ => Err(format!("{at}: a box and a card")),
        }
    }

    fn node(&mut self, x: NodeId, y: NodeId, at: &str, whole: bool) -> Result<(), String> {
        let (Some(p), Some(q)) = (self.a.boxes.get(&x), self.b.boxes.get(&y)) else {
            return Err(format!("{at}: missing box"));
        };
        let at = format!("{at} {:?}", flat(&p.text));
        if p.kind != q.kind {
            return Err(format!("{at}: kind {:?} != {:?}", p.kind, q.kind));
        }
        if flat(&p.text) != flat(&q.text) {
            return Err(format!("{at}: text != {:?}", flat(&q.text)));
        }
        let comments = |n: &crate::model::BoxNode| {
            let c = n.comments.iter();
            c.map(|c| (c.author.clone(), c.text.clone(), c.verdict))
                .collect::<Vec<_>>()
        };
        let note = p.kind == BoxKind::Note;
        let mut fields = vec![
            (
                "acceptance",
                format!("{:?}", p.acceptance),
                format!("{:?}", q.acceptance),
            ),
            (
                "rating",
                format!("{:?}", p.source_rating),
                format!("{:?}", q.source_rating),
            ),
        ];
        if !note || whole {
            fields.push(("link", format!("{:?}", p.link), format!("{:?}", q.link)));
            fields.push((
                "comments",
                format!("{:?}", comments(p)),
                format!("{:?}", comments(q)),
            ));
        }
        for (what, u, v) in fields {
            if u != v {
                return Err(format!("{at}: {what} {u} != {v}"));
            }
        }
        let split = |d: &Document, items: &[Item]| {
            let is_note = |i: &Item| matches!(i, Item::Box(b) if d.boxes.get(b).is_some_and(|n| n.kind == BoxKind::Note));
            let (notes, rest): (Vec<Item>, Vec<Item>) = items.iter().partition(|i| is_note(i));
            let texts: Vec<String> = notes
                .iter()
                .filter_map(|i| match i {
                    Item::Box(b) => d.boxes.get(b).map(|n| flat(&n.text)),
                    Item::Card(_) => None,
                })
                .collect();
            (texts, rest)
        };
        let (notes_p, rest_p) = split(self.a, &p.children);
        let (notes_q, rest_q) = split(self.b, &q.children);
        if notes_p != notes_q {
            return Err(format!("{at}: notes {notes_p:?} != {notes_q:?}"));
        }
        self.list(&rest_p, &rest_q, &at)
    }

    fn card(&mut self, x: CardId, y: CardId, at: &str) -> Result<(), String> {
        let (Some(c), Some(d)) = (self.a.cards.get(&x), self.b.cards.get(&y)) else {
            return Err(format!("{at}: missing card"));
        };
        let at = format!("{at} card");
        if c.kind != d.kind || c.strength != d.strength {
            let (u, v) = ((&c.kind, c.strength), (&d.kind, d.strength));
            return Err(format!("{at}: kind and strength {u:?} != {v:?}"));
        }
        let premises = |c: &crate::model::Card| c.premises.iter().map(|&p| Item::Box(p)).collect();
        let (pc, pd): (Vec<Item>, Vec<Item>) = (premises(c), premises(d));
        self.list(&pc, &pd, &format!("{at} premise"))?;
        let children = |c: &crate::model::Card| c.children.iter().map(|&k| Item::Card(k)).collect();
        let (kc, kd): (Vec<Item>, Vec<Item>) = (children(c), children(d));
        self.list(&kc, &kd, &format!("{at} whole-card child"))
    }

    fn list(&mut self, xs: &[Item], ys: &[Item], at: &str) -> Result<(), String> {
        if xs.len() != ys.len() {
            return Err(format!("{at}: {} children != {}", xs.len(), ys.len()));
        }
        for (k, (&x, &y)) in xs.iter().zip(ys).enumerate().rev() {
            self.todo.push((x, y, format!("{at}/{k}"), false));
        }
        Ok(())
    }
}

/// Text as Argdown keeps it: on one line, without surrounding spaces.
fn flat(text: &str) -> String {
    let lines: Vec<&str> = text
        .split(['\n', '\r'])
        .filter(|l| !l.trim().is_empty())
        .collect();
    lines.join(" ").trim().to_owned()
}
