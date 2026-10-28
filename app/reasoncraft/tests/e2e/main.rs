//! End to end: build and evaluate a map through actions only (ratings,
//! sources, links, comments, an image, a reorder, a "Connect to…"), save
//! it, open it again and check that nothing was lost. The step-3 text
//! panel flow is in [`text_panel`], the title/author session on the
//! close paths in [`meta_sessions`], the step-4 essay and exports in
//! [`output`], the step-5 interchange (find, spelling, Argdown, SVG,
//! recovery) in [`interchange`].

mod build_reopen;
mod interchange;
mod meta_sessions;
mod output;
mod text_panel;
mod zip_entries;

use reasoncraft::actions::{Action, Dir, Insertable, Rate, apply, wait_for_images};
use reasoncraft::app::ReasonCraftApp;
use reasoncraft::canvas::comments::PRESETS;
use reasoncraft::file_ops::FakeDialogs;
use reasoncraft::i18n::t;
use reasoncraft_core::fixtures::FixedMeasure;
use reasoncraft_core::{
    Acceptance, BoxKind, CardId, CardKind, CardStrength, Item, LayoutParams, NewLoose, NodeId,
    Parent, Role, SourceRating, SourceType, Verdict, file, layout,
};

/// Apply each action in turn, each save written before the next.
fn run(app: &mut ReasonCraftApp, actions: impl IntoIterator<Item = Action>) {
    for a in actions {
        apply(app, a);
        // Saves are written on a worker: the next step sees them done.
        reasoncraft::file_ops::wait_for_saves(app);
    }
}

/// The box the active tab is editing (every add starts an edit).
fn editing(app: &ReasonCraftApp) -> NodeId {
    app.active_tab().editing().expect("an add starts an edit")
}

/// The card that owns premise `b`.
fn card_of(app: &ReasonCraftApp, b: NodeId) -> CardId {
    match app.active_tab().doc.owner_of(Item::Box(b)) {
        Some(Parent::Card(c)) => c,
        other => panic!("{b:?} is owned by {other:?}, not a card"),
    }
}

fn typed(text: &str) -> Action {
    Action::TypeText(text.to_owned())
}
