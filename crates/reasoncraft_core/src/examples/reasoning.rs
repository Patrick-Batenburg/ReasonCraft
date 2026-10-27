//! The reasoning example: should cities make public transport free?
//!
//! Three reasons and two objections under the claim. The first reason has
//! a co-premise, an objection to one premise and an objection to the
//! whole card; the first objection is rebutted. Every premise rests on a
//! source, and everything is rated.

use super::{Lang, card, rate, source, start};
use crate::edit::EditError;
use crate::ids::Parent;
use crate::model::Acceptance::{Accept, Unsure};
use crate::model::CardKind::{Objection, Reason};
use crate::model::CardStrength::{Strong, Weak};
use crate::model::SourceRating::{Shaky, Solid};
use crate::model::{BoxKind, Document, SourceType};

pub(super) fn build(l: Lang) -> Result<Document, EditError> {
    let mut d = start(
        l,
        l.s(
            "Cities should make public transport free",
            "Steden moeten het openbaar vervoer gratis maken",
        ),
        l.s(
            "Example: free public transport",
            "Voorbeeld: gratis openbaar vervoer",
        ),
    )?;
    let claim = Parent::Box(d.root);
    d.add_box(
        claim,
        BoxKind::Note,
        l.s(
            "Free here means that nobody pays a fare for buses, trams or trains within the city.",
            "Gratis betekent hier dat niemand betaalt voor bussen, trams of treinen binnen de stad.",
        ),
    )?;
    fewer_cars(&mut d, l)?;
    low_incomes(&mut d, l)?;
    already_done(&mut d, l)?;
    lost_income(&mut d, l)?;
    better_service(&mut d, l)?;
    d.set_acceptance(d.root, Some(Accept))?;
    Ok(d)
}

/// The first reason: fewer cars. A co-premise, an objection to its first
/// premise and an objection to the whole card.
fn fewer_cars(d: &mut Document, l: Lang) -> Result<(), EditError> {
    let (card_id, p) = card(
        d,
        Parent::Box(d.root),
        Reason,
        &[
            l.s(
                "Free public transport would tempt many drivers to leave their cars at home",
                "Gratis openbaar vervoer zou veel automobilisten verleiden hun auto te laten staan",
            ),
            l.s(
                "Cities should make cutting car traffic a priority",
                "Steden moeten er een prioriteit van maken om autoverkeer terug te dringen",
            ),
        ],
    )?;
    rate(d, card_id, Strong, &p, &[Unsure, Accept])?;
    source(
        d,
        p[0],
        SourceType::Publication,
        l.s(
            "Studies of fare-free schemes report more passengers once fares were dropped",
            "Onderzoek naar gratis vervoer laat meer reizigers zien zodra je niet meer hoeft te betalen",
        ),
        Solid,
    )?;
    source(
        d,
        p[1],
        SourceType::ExpertOpinion,
        l.s(
            "Health agencies name road traffic as a main source of air pollution in cities",
            "Volgens gezondheidsdiensten is wegverkeer een belangrijke bron van luchtvervuiling in steden",
        ),
        Solid,
    )?;
    let (walkers, q) = card(
        d,
        Parent::Box(p[0]),
        Objection,
        &[l.s(
            "Many of the new passengers would be people who used to walk",
            "Veel nieuwe reizigers zouden mensen zijn die eerst liepen",
        )],
    )?;
    rate(d, walkers, Weak, &q, &[Accept])?;
    source(
        d,
        q[0],
        SourceType::Publication,
        l.s(
            "A study of free public transport in Tallinn found that walking fell as bus use rose",
            "Een studie naar het gratis openbaar vervoer in Tallinn vond dat er minder werd gelopen toen het busgebruik steeg",
        ),
        Solid,
    )?;
    let (zones, z) = card(
        d,
        Parent::Card(card_id),
        Objection,
        &[l.s(
            "Low-emission zones could cut car traffic at a lower cost",
            "Milieuzones kunnen autoverkeer goedkoper terugdringen",
        )],
    )?;
    rate(d, zones, Weak, &z, &[Unsure])?;
    source(
        d,
        z[0],
        SourceType::Example,
        l.s(
            "Many European cities already have a low-emission zone",
            "Veel Europese steden hebben al een milieuzone",
        ),
        Shaky,
    )?;
    Ok(())
}

/// The second reason: people on low incomes. Two premises.
fn low_incomes(d: &mut Document, l: Lang) -> Result<(), EditError> {
    let (card_id, p) = card(
        d,
        Parent::Box(d.root),
        Reason,
        &[
            l.s(
                "Free public transport would help people on low incomes get to work",
                "Gratis openbaar vervoer zou mensen met een laag inkomen helpen om op hun werk te komen",
            ),
            l.s(
                "Cities should make sure that everyone can afford to get around",
                "Steden moeten het voor iedereen betaalbaar maken om zich te verplaatsen",
            ),
        ],
    )?;
    rate(d, card_id, Strong, &p, &[Accept, Accept])?;
    source(
        d,
        p[0],
        SourceType::ByDefinition,
        l.s(
            "A fare takes a larger share of a small budget than of a large one",
            "Een kaartje kost een groter deel van een klein budget dan van een groot budget",
        ),
        Solid,
    )?;
    source(
        d,
        p[1],
        SourceType::MyBasis,
        l.s(
            "People who cannot travel miss out on work, school and care",
            "Wie niet kan reizen, mist kansen op werk, school en zorg",
        ),
        Solid,
    )?;
    Ok(())
}

/// The third reason: it is done already. One premise, two sources.
fn already_done(d: &mut Document, l: Lang) -> Result<(), EditError> {
    let (card_id, p) = card(
        d,
        Parent::Box(d.root),
        Reason,
        &[l.s(
            "Some cities already make public transport free",
            "Sommige steden maken het openbaar vervoer al gratis",
        )],
    )?;
    rate(d, card_id, Weak, &p, &[Accept])?;
    source(
        d,
        p[0],
        SourceType::Example,
        l.s(
            "Tallinn has offered free public transport to its residents since 2013",
            "Tallinn biedt zijn inwoners sinds 2013 gratis openbaar vervoer",
        ),
        Solid,
    )?;
    source(
        d,
        p[0],
        SourceType::Event,
        l.s(
            "Luxembourg made all public transport in the country free in 2020",
            "Luxemburg maakte in 2020 al het openbaar vervoer in het land gratis",
        ),
        Solid,
    )?;
    Ok(())
}

/// The first objection: lost fare income, rebutted.
fn lost_income(d: &mut Document, l: Lang) -> Result<(), EditError> {
    let (card_id, p) = card(
        d,
        Parent::Box(d.root),
        Objection,
        &[l.s(
            "Cities would lose a large source of income for public transport",
            "Steden zouden een grote bron van inkomsten voor het openbaar vervoer verliezen",
        )],
    )?;
    rate(d, card_id, Weak, &p, &[Unsure])?;
    source(
        d,
        p[0],
        SourceType::Data,
        l.s(
            "Transport budgets list ticket sales as a source of income",
            "Begrotingen van vervoersbedrijven noemen kaartverkoop als bron van inkomsten",
        ),
        Solid,
    )?;
    let (rebuttal, r) = card(
        d,
        Parent::Box(p[0]),
        Objection,
        &[l.s(
            "In some cities, fares pay for only a small part of the costs",
            "In sommige steden betalen de kaartjes maar een klein deel van de kosten",
        )],
    )?;
    rate(d, rebuttal, Strong, &r, &[Accept])?;
    source(
        d,
        r[0],
        SourceType::Media,
        l.s(
            "News reports on Luxembourg's switch said fares had covered less than a tenth of the costs",
            "Volgens nieuwsberichten over de overstap in Luxemburg dekten de kaartjes minder dan een tiende van de kosten",
        ),
        Solid,
    )?;
    Ok(())
}

/// The second objection: the money could buy better service.
fn better_service(d: &mut Document, l: Lang) -> Result<(), EditError> {
    let (card_id, p) = card(
        d,
        Parent::Box(d.root),
        Objection,
        &[l.s(
            "The money for free fares would do more good if spent on more frequent services",
            "Het geld voor gratis reizen doet meer goed als het naar vaker rijdend vervoer gaat",
        )],
    )?;
    rate(d, card_id, Weak, &p, &[Unsure])?;
    source(
        d,
        p[0],
        SourceType::ExpertOpinion,
        l.s(
            "Many transport planners say that frequent, reliable service wins more riders than low fares",
            "Veel vervoersplanners zeggen dat vaak en betrouwbaar rijden meer reizigers trekt dan lage prijzen",
        ),
        Solid,
    )?;
    Ok(())
}
