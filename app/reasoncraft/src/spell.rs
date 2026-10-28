//! Spellcheck in the document language: dictionaries loaded in the
//! background ([`SpellService`]), red underlines in the editing fields
//! ([`SpellFields`]) and the F7 spelling pass ([`SpellPass`], shown by
//! [`crate::shell::spelling_panel`]). Underlines are screen-only: the
//! painted map and exports never show them.

mod layouter;
mod line_cache;
mod pass;
mod service;

pub use layouter::{FieldStyle, SpellFields, UNDERLINE_WIDTH, spell_layouter};
pub(crate) use pass::apply;
pub use pass::{CONTEXT_CHARS, MAX_SUGGESTIONS, SpellPass, context};
pub use service::{Ranges, SpellService};

use crate::app::ReasonCraftApp;
use crate::i18n::{FluentArgs, t_args};

/// Once a frame: pick up loaded dictionaries, report a load failure (once,
/// in the status bar), let a spelling pass go on that waits for its
/// dictionary, walks on from the last frame or whose misspelling the
/// document no longer holds, and pick up its suggestions. The pass waits
/// while a box is being edited inline: walking on would move the selection
/// away from the box being edited (it goes on once the edit ends).
pub fn poll(app: &mut ReasonCraftApp) {
    app.spell.poll_all();
    if let Some(error) = app.spell.take_error() {
        let mut args = FluentArgs::new();
        args.set("error", error);
        app.status_note = Some(t_args("spell.load_failed", &args));
    }
    if let Some(tab) = app.tabs.get_mut(app.active)
        && tab.editing().is_none()
        && tab
            .spelling
            .as_ref()
            .is_some_and(|p| p.needs_advance(&tab.doc))
    {
        pass::advance(tab, &mut app.spell);
    }
    if let Some(tab) = app.tabs.get_mut(app.active)
        && let Some(pass) = &mut tab.spelling
    {
        pass::take_suggestions(pass, &mut app.spell, &tab.doc.meta.language);
    }
}

/// Whether the active tab's spelling pass walks on next frame (so the
/// next frame should come at once); not while it waits for an inline edit.
pub fn walking(app: &ReasonCraftApp) -> bool {
    let tab = app.tabs.get(app.active).filter(|t| t.editing().is_none());
    tab.and_then(|t| t.spelling.as_ref())
        .is_some_and(SpellPass::walking)
}
