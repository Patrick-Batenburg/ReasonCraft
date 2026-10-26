//! Spellcheck in the document language: dictionaries loaded in the
//! background ([`SpellService`]), red underlines in the editing fields
//! ([`SpellFields`]) and the F7 spelling pass ([`SpellPass`], shown by
//! [`crate::shell::spelling_panel`]). Underlines are screen-only: the
//! painted map and exports never show them.

mod layouter;
mod pass;
mod service;

pub use layouter::{FieldStyle, SpellFields, UNDERLINE_WIDTH, spell_layouter};
pub(crate) use pass::apply;
pub use pass::{CONTEXT_CHARS, MAX_SUGGESTIONS, SpellPass, context};
pub use service::{Ranges, SpellService};

use crate::app::ReasonCraftApp;
use crate::i18n::{FluentArgs, t_args};

/// Once a frame: pick up loaded dictionaries, report a load failure (once,
/// in the status bar) and let a waiting spelling pass go on.
pub fn poll(app: &mut ReasonCraftApp) {
    app.spell.poll_all();
    if let Some(error) = app.spell.take_error() {
        let mut args = FluentArgs::new();
        args.set("error", error);
        app.status_note = Some(t_args("spell.load_failed", &args));
    }
    if let Some(tab) = app.tabs.get_mut(app.active)
        && tab.spelling.as_ref().is_some_and(SpellPass::waiting)
    {
        pass::advance(tab, &mut app.spell);
    }
}
