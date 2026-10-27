//! The templates: maps to start from, with placeholder text to replace.

use super::{Lang, Template, card, start};
use crate::edit::EditError;
use crate::essay::EssaySlot;
use crate::ids::Parent;
use crate::model::CardKind::{Objection, Reason};
use crate::model::{BoxKind, Document};

pub(super) fn build(which: Template, l: Lang) -> Result<Document, EditError> {
    match which {
        Template::BlankReasoning => blank_reasoning(l),
        Template::BlankGrouping => blank_grouping(l),
        Template::EssayPlan => essay_plan(l),
        Template::PeerReview => peer_review(l),
    }
}

fn blank_reasoning(l: Lang) -> Result<Document, EditError> {
    let mut d = start(
        l,
        l.s("Write your claim here", "Schrijf hier je bewering"),
        "",
    )?;
    let claim = Parent::Box(d.root);
    let reason = l.s(
        "Write a reason that supports the claim",
        "Schrijf een reden die de bewering steunt",
    );
    card(&mut d, claim, Reason, &[reason])?;
    let objection = l.s(
        "Write an objection to the claim",
        "Schrijf een bezwaar tegen de bewering",
    );
    card(&mut d, claim, Objection, &[objection])?;
    Ok(d)
}

fn blank_grouping(l: Lang) -> Result<Document, EditError> {
    let mut d = start(
        l,
        l.s("Write your topic here", "Schrijf hier je onderwerp"),
        "",
    )?;
    let topic = Parent::Box(d.root);
    for text in [
        l.s("First category", "Eerste categorie"),
        l.s("Second category", "Tweede categorie"),
        l.s("Third category", "Derde categorie"),
    ] {
        d.add_box(topic, BoxKind::Category, text)?;
    }
    Ok(d)
}

fn essay_plan(l: Lang) -> Result<Document, EditError> {
    let mut d = start(
        l,
        l.s(
            "The claim your essay defends",
            "De bewering die je betoog verdedigt",
        ),
        "",
    )?;
    let claim = Parent::Box(d.root);
    for text in [
        l.s("Your strongest reason", "Je sterkste reden"),
        l.s("A second reason", "Een tweede reden"),
        l.s("A third reason", "Een derde reden"),
    ] {
        card(&mut d, claim, Reason, &[text])?;
    }
    let objection = l.s(
        "The strongest objection a reader could raise",
        "Het sterkste bezwaar dat een lezer kan maken",
    );
    let (_, p) = card(&mut d, claim, Objection, &[objection])?;
    let answer = l.s(
        "Your answer to that objection",
        "Je antwoord op dat bezwaar",
    );
    card(&mut d, Parent::Box(p[0]), Objection, &[answer])?;
    d.set_essay_text(
        EssaySlot::Intro,
        l.s(
            "Introduce the topic, then state the claim you will defend.",
            "Leid het onderwerp in en noem de bewering die je gaat verdedigen.",
        ),
    )?;
    d.set_essay_text(
        EssaySlot::Conclusion,
        l.s(
            "Sum up your reasons, weigh them against the objection, and state your conclusion again.",
            "Vat je redenen samen, weeg ze af tegen het bezwaar en herhaal je conclusie.",
        ),
    )?;
    Ok(d)
}

fn peer_review(l: Lang) -> Result<Document, EditError> {
    let mut d = start(
        l,
        l.s(
            "Put the claim you are reviewing here",
            "Zet hier de bewering die je beoordeelt",
        ),
        "",
    )?;
    d.add_box(
        Parent::Box(d.root),
        BoxKind::Note,
        l.s(
            "To review a map, select a box and click a preset under Comments in the Building panel, \
             or write a comment of your own. Presets come in three kinds: positive, needs work and \
             problem. Each box shows how many comments it has.",
            "Om een kaart te beoordelen kies je een vak en klik je in Bouwstenen onder Opmerkingen \
             op een standaardopmerking, of je schrijft zelf een opmerking. Er zijn drie soorten: \
             positief, kan beter en probleem. Elk vak laat zien hoeveel opmerkingen het heeft.",
        ),
    )?;
    Ok(d)
}
