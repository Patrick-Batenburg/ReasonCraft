//! The map for screen readers. The map is painted, so AccessKit
//! sees one canvas node; this gives it:
//! - a value that describes the selection ("Reason, premise 1 of 2,
//!   accepted: Cats purr"), or "Nothing selected";
//! - a description of "Move to…" while it runs;
//! - a child node per box and card on screen (and the selection, wherever
//!   it is), in the map's reading order, named the same way, with the
//!   selection marked selected and set as the canvas's active descendant,
//!   so a screen reader reads each item the arrow keys move to.
//!
//! Only built while AccessKit is on (a screen reader is listening).

use std::collections::{HashMap, HashSet};

use eframe::egui::{self, Rect, Role, UiBuilder, accesskit};
use reasoncraft_core::{
    Acceptance, BoxKind, CardId, CardStrength, Document, Item, Layout, NodeId, Parent,
    Role as CardRole, SourceRating,
};

use crate::doc_tab::DocTab;
use crate::i18n::{FluentArgs, t, t_args};

/// The most characters of a box's text a child node is named with; the
/// selection's own description has all of it.
const NAME_CHARS: usize = 300;

/// The id of the child node of `item`.
fn item_node_id(item: Item) -> egui::Id {
    super::canvas_id().with(("map_item", item))
}

/// Describe the map to AccessKit for this frame (see the module docs).
/// `view` is the map's screen rect.
pub(super) fn expose(ui: &mut egui::Ui, tab: &DocTab, layout: &Layout, view: Rect) {
    let ctx = ui.ctx().clone();
    let canvas = super::canvas_id();
    // Nothing to do while no screen reader listens.
    if ctx.accesskit_node_builder(canvas, |_| ()).is_none() {
        return;
    }
    let tree = Tree::new(&tab.doc);
    let value = tab
        .selection
        .filter(|&i| tree.has(i))
        .map_or_else(|| t("a11y.map.nothing"), |i| tree.describe(i, usize::MAX));
    let moving = tab.connect_mode.filter(|&i| tree.has(i)).map(|item| {
        let mut args = FluentArgs::new();
        args.set("item", tree.describe(item, NAME_CHARS));
        t_args("a11y.map.moving", &args)
    });
    ctx.accesskit_node_builder(canvas, |node| {
        node.set_value(value);
        if let Some(moving) = moving {
            node.set_description(moving);
        }
    });

    let cam = tab.camera;
    for &item in &tree.order {
        let world = match item {
            Item::Box(b) => layout.boxes.get(&b),
            Item::Card(c) => layout.cards.get(&c),
        };
        let Some(&world) = world else { continue };
        let screen = cam.world_rect_to_screen(world);
        let selected = tab.selection == Some(item);
        if !selected && !view.intersects(screen) {
            continue;
        }
        let id = item_node_id(item);
        let builder = UiBuilder::new()
            .scope_id(id)
            .max_rect(screen)
            .accessibility_parent(canvas)
            .accessibility_role(Role::TreeItem)
            .accessibility_label(tree.describe(item, NAME_CHARS));
        drop(ui.new_child(builder));
        ctx.accesskit_node_builder(id, |node| {
            node.set_bounds(accesskit::Rect {
                x0: screen.min.x.into(),
                y0: screen.min.y.into(),
                x1: screen.max.x.into(),
                y1: screen.max.y.into(),
            });
            node.set_selected(selected);
        });
        if selected {
            ctx.accesskit_node_builder(canvas, |node| {
                node.set_active_descendant(id.accesskit_id());
            });
        }
    }
}

/// The map's structure, worked out once a frame: every item in reading
/// order (the main tree from the root, each card before its premises and
/// what hangs below, then the parking area), what holds each, which are
/// parked, and each card's role.
pub struct Tree<'a> {
    doc: &'a Document,
    order: Vec<Item>,
    owner: HashMap<Item, Parent>,
    parked: HashSet<Item>,
    roles: HashMap<CardId, CardRole>,
}

impl<'a> Tree<'a> {
    pub fn new(doc: &'a Document) -> Self {
        let mut order = Vec::new();
        let mut owner = HashMap::new();
        let mut parked = HashSet::new();
        // Each entry: an item, what holds it, and whether it is parked.
        let mut stack: Vec<(Item, Option<Parent>, bool)> = doc
            .parking
            .iter()
            .rev()
            .map(|&i| (i, Some(Parent::Parking), true))
            .collect();
        stack.push((Item::Box(doc.root), None, false));
        let mut seen = HashSet::new();
        while let Some((item, holder, in_parking)) = stack.pop() {
            if !seen.insert(item) {
                continue;
            }
            order.push(item);
            if let Some(holder) = holder {
                owner.insert(item, holder);
            }
            if in_parking {
                parked.insert(item);
            }
            let (as_parent, children) = match item {
                Item::Box(b) => (
                    Parent::Box(b),
                    doc.boxes
                        .get(&b)
                        .map(|n| n.children.clone())
                        .unwrap_or_default(),
                ),
                Item::Card(c) => (
                    Parent::Card(c),
                    doc.cards.get(&c).map_or_else(Vec::new, |card| {
                        let premises = card.premises.iter().map(|&p| Item::Box(p));
                        premises
                            .chain(card.children.iter().map(|&k| Item::Card(k)))
                            .collect()
                    }),
                ),
            };
            for child in children.into_iter().rev() {
                stack.push((child, Some(as_parent), in_parking));
            }
        }
        Self {
            doc,
            order,
            owner,
            parked,
            roles: reasoncraft_core::card_roles(doc).into_iter().collect(),
        }
    }

    /// Whether `item` is in the document.
    fn has(&self, item: Item) -> bool {
        match item {
            Item::Box(b) => self.doc.boxes.contains_key(&b),
            Item::Card(c) => self.doc.cards.contains_key(&c),
        }
    }

    fn role(&self, c: CardId) -> CardRole {
        self.roles.get(&c).copied().unwrap_or(CardRole::Supports)
    }

    /// What a screen reader says for `item`: its kind (a premise by its
    /// card's role), its rating, its place among its siblings, whether it
    /// is parked, and its text (cut at `max_chars`); for a card, the text
    /// of its premises.
    pub fn describe(&self, item: Item, max_chars: usize) -> String {
        let doc = self.doc;
        let mut parts = Vec::new();
        let text = match item {
            Item::Box(b) => {
                let Some(node) = doc.boxes.get(&b) else {
                    return String::new();
                };
                parts.push(self.box_kind(item, &node.kind));
                if let Some(rating) = box_rating(node.acceptance, node.source_rating) {
                    parts.push(rating);
                }
                cut(&node.text, max_chars)
            }
            Item::Card(c) => {
                let Some(card) = doc.cards.get(&c) else {
                    return String::new();
                };
                let role = self.role(c);
                let mut args = FluentArgs::new();
                args.set("role", role_name(role));
                parts.push(t_args("a11y.map.card", &args));
                if let Some(strength) = card.strength {
                    parts.push(card_rating(strength, role));
                }
                let texts: Vec<&str> = card
                    .premises
                    .iter()
                    .filter_map(|p| doc.boxes.get(p).map(|n| n.text.as_str()))
                    .collect();
                cut(&texts.join(" + "), max_chars)
            }
        };
        if let Some(position) = self.position(item) {
            parts.push(position);
        }
        if self.parked.contains(&item) {
            parts.push(t("a11y.map.parked"));
        }
        let text = if text.trim().is_empty() {
            t("a11y.map.empty")
        } else {
            text
        };
        let mut args = FluentArgs::new();
        args.set("what", parts.join(", "));
        args.set("text", text);
        t_args("a11y.map.item", &args)
    }

    /// What the inline editor of box `b` is called: "Text of Reason, …".
    pub fn editor_name(&self, b: NodeId) -> String {
        let Some(node) = self.doc.boxes.get(&b) else {
            return String::new();
        };
        let mut args = FluentArgs::new();
        args.set("what", self.box_kind(Item::Box(b), &node.kind));
        t_args("a11y.map.editing", &args)
    }

    /// A box's kind; a premise is named by its card's role, and when its
    /// card has more than one, by its place among them.
    fn box_kind(&self, item: Item, kind: &BoxKind) -> String {
        match kind {
            BoxKind::Claim => t("a11y.map.claim"),
            BoxKind::Premise => {
                let Some(&Parent::Card(c)) = self.owner.get(&item) else {
                    return t("a11y.map.supports");
                };
                let mut name = role_name(self.role(c));
                let premises = &self.doc.cards[&c].premises;
                if premises.len() > 1
                    && let Some(i) = premises.iter().position(|p| Item::Box(*p) == item)
                {
                    let mut args = FluentArgs::new();
                    args.set("n", i + 1);
                    args.set("total", premises.len());
                    name = format!("{name}, {}", t_args("a11y.map.premise", &args));
                }
                name
            }
            BoxKind::Category => t("a11y.map.category"),
            BoxKind::Note => t("a11y.map.note"),
            BoxKind::Source { source_type } => {
                let mut args = FluentArgs::new();
                args.set("type", t(&format!("source.{}", source_type.key())));
                t_args("a11y.map.source", &args)
            }
        }
    }

    /// "2 of 3": `item`'s place among its siblings (a premise's among its
    /// card's premises is part of its kind instead); `None` when it has
    /// none.
    fn position(&self, item: Item) -> Option<String> {
        let doc = self.doc;
        let siblings: Vec<Item> = match (*self.owner.get(&item)?, item) {
            (Parent::Card(_), Item::Box(_)) => return None,
            (Parent::Box(b), _) => doc.boxes.get(&b)?.children.clone(),
            (Parent::Card(c), Item::Card(_)) => doc
                .cards
                .get(&c)?
                .children
                .iter()
                .map(|&k| Item::Card(k))
                .collect(),
            (Parent::Parking, _) => doc.parking.clone(),
        };
        if siblings.len() < 2 {
            return None;
        }
        let n = siblings.iter().position(|&s| s == item)? + 1;
        let mut args = FluentArgs::new();
        args.set("n", n);
        args.set("total", siblings.len());
        Some(t_args("a11y.map.position", &args))
    }
}

fn role_name(role: CardRole) -> String {
    t(match role {
        CardRole::Supports => "a11y.map.supports",
        CardRole::Opposes => "a11y.map.opposes",
        CardRole::Rebuts => "a11y.map.rebuts",
    })
}

/// "accepted", "rated Solid" and the like.
fn box_rating(acceptance: Option<Acceptance>, source: Option<SourceRating>) -> Option<String> {
    if let Some(a) = acceptance {
        return Some(t(match a {
            Acceptance::Accept => "a11y.map.accept",
            Acceptance::Unsure => "a11y.map.unsure",
            Acceptance::Reject => "a11y.map.reject",
        }));
    }
    let rating = t(match source? {
        SourceRating::Solid => "eval.solid",
        SourceRating::Shaky => "eval.shaky",
        SourceRating::Nil => "eval.nil",
    });
    let mut args = FluentArgs::new();
    args.set("rating", rating);
    Some(t_args("a11y.map.rated", &args))
}

/// "strongly supports" and the like, as the card's label on the map says.
fn card_rating(strength: CardStrength, role: CardRole) -> String {
    let strength = match strength {
        CardStrength::Strong => "strong",
        CardStrength::Weak => "weak",
        CardStrength::Nil => "nil",
    };
    let role = match role {
        CardRole::Supports => "supports",
        CardRole::Opposes => "opposes",
        CardRole::Rebuts => "rebuts",
    };
    t(&format!("rating.{strength}.{role}"))
}

/// `text` cut to `max` characters, with "…" when cut.
fn cut(text: &str, max: usize) -> String {
    match text.char_indices().nth(max) {
        Some((end, _)) => format!("{}…", &text[..end]),
        None => text.to_owned(),
    }
}
