//! The grouping example: ways to reduce food waste, three levels deep.

use super::{Lang, start};
use crate::edit::EditError;
use crate::ids::{NodeId, Parent};
use crate::model::{BoxKind, Document};

/// English and Dutch text.
type Text = (&'static str, &'static str);

/// A category, a note on it and the categories under it.
struct Group {
    text: Text,
    note: Option<Text>,
    below: &'static [Group],
}

const fn leaf(en: &'static str, nl: &'static str) -> Group {
    group(en, nl, &[])
}

const fn group(en: &'static str, nl: &'static str, below: &'static [Group]) -> Group {
    Group {
        text: (en, nl),
        note: None,
        below,
    }
}

const GROUPS: [Group; 3] = [
    group(
        "At home",
        "Thuis",
        &[
            group(
                "Plan before you shop",
                "Plan voordat je boodschappen doet",
                &[
                    leaf("Write a shopping list", "Maak een boodschappenlijstje"),
                    leaf("Check the fridge first", "Kijk eerst in de koelkast"),
                ],
            ),
            group(
                "Store food well",
                "Bewaar eten goed",
                &[
                    leaf(
                        "Freeze bread you will not eat soon",
                        "Vries brood in dat je niet snel opeet",
                    ),
                    Group {
                        text: (
                            "Learn what the dates on packs mean",
                            "Leer wat de datums op verpakkingen betekenen",
                        ),
                        note: Some((
                            "\"Best before\" is about quality; \"use by\" is about safety.",
                            "\"Ten minste houdbaar tot\" gaat over kwaliteit, \"te gebruiken tot\" over veiligheid.",
                        )),
                        below: &[],
                    },
                ],
            ),
            leaf("Cook with leftovers", "Kook met restjes"),
        ],
    ),
    group(
        "In shops",
        "In winkels",
        &[
            leaf(
                "Sell oddly shaped fruit and vegetables",
                "Verkoop groente en fruit met een vreemde vorm",
            ),
            leaf(
                "Lower the price of food near its date",
                "Geef korting op eten dat bijna over de datum is",
            ),
            leaf(
                "Give unsold food to food banks",
                "Geef onverkocht eten aan de voedselbank",
            ),
        ],
    ),
    group(
        "In canteens and restaurants",
        "In kantines en restaurants",
        &[
            leaf("Offer smaller portions", "Bied kleinere porties aan"),
            leaf(
                "Let guests take leftovers home",
                "Laat gasten restjes meenemen",
            ),
        ],
    ),
];

pub(super) fn build(l: Lang) -> Result<Document, EditError> {
    let mut d = start(
        l,
        l.s(
            "Ways to reduce food waste",
            "Manieren om voedselverspilling te verminderen",
        ),
        l.s("Example: food waste", "Voorbeeld: voedselverspilling"),
    )?;
    let root = d.root;
    for g in &GROUPS {
        add(&mut d, l, root, g)?;
    }
    Ok(d)
}

/// Add `g`, its note and everything under it beneath `parent`.
fn add(d: &mut Document, l: Lang, parent: NodeId, g: &Group) -> Result<(), EditError> {
    let (en, nl) = g.text;
    let id = d.add_box(Parent::Box(parent), BoxKind::Category, l.s(en, nl))?;
    if let Some((en, nl)) = g.note {
        d.add_box(Parent::Box(id), BoxKind::Note, l.s(en, nl))?;
    }
    for below in g.below {
        add(d, l, id, below)?;
    }
    Ok(())
}
