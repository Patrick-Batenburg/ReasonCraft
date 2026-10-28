//! Every command the ribbon, the backstage and the Search commands box
//! offer, in one registry: a stable id, a label, the ribbon icon and the
//! action it runs.
//!
//! The ribbon and backstage take their labels, icons and actions from
//! here; the title bar's Search commands box (Alt+Q) searches it. The
//! keyboard shortcuts live in one table per platform, see
//! [`crate::canvas::input::shortcuts`](fn@crate::canvas::input::shortcuts).

use std::sync::OnceLock;

use crate::actions::Action;
use crate::i18n::UI_LANGUAGES;
use crate::shell::icons::Icon;

mod registry;
mod search;
pub use search::search;

#[cfg(test)]
mod readme_tests;
#[cfg(test)]
mod tests;

/// One command.
#[derive(Clone, Debug)]
pub struct Command {
    /// Dotted and stable, e.g. `home.add_reason`, `view.zoom_fit`.
    pub id: &'static str,
    /// The i18n key of its label.
    pub label: &'static str,
    /// The i18n key of the ribbon group (or File page) it belongs to,
    /// e.g. `group.add`; shown beside the label in search results.
    pub group: &'static str,
    /// Its icon on the ribbon (`None` for commands the ribbon shows
    /// otherwise: File page entries, menu items, source types).
    pub icon: Option<Icon>,
    pub action: Action,
}

impl Command {
    /// This command with ribbon icon `icon`.
    pub fn with_icon(self, icon: Icon) -> Self {
        Self {
            icon: Some(icon),
            ..self
        }
    }

    /// Its ribbon icon.
    ///
    /// # Panics
    ///
    /// When it has none: a programmer error that the ribbon tests catch.
    pub fn ribbon_icon(&self) -> Icon {
        self.icon
            .unwrap_or_else(|| panic!("command `{}` has no ribbon icon", self.id))
    }

    /// The i18n key of the ribbon tab the command sits on, from the first
    /// part of its id (`view.zoom_100` is on View).
    pub fn tab(&self) -> &'static str {
        match self.id.split('.').next() {
            Some("file") => "ribbon.tab.file",
            Some("home") => "ribbon.tab.home",
            Some("evaluate") => "ribbon.tab.evaluate",
            Some("view") => "ribbon.tab.view",
            Some("export") => "ribbon.tab.export",
            Some("help") => "ribbon.tab.help",
            _ => panic!("command `{}` names no ribbon tab", self.id),
        }
    }
}

/// The id of the command that adds a source of type `st`
/// (`home.add_source.<key>`).
pub fn add_source_id(st: &reasoncraft_core::SourceType) -> String {
    format!("home.add_source.{}", st.key())
}

/// The interface-language choices, in menu order, with the command that
/// picks each: following the OS (`None`), then every UI language. View →
/// Language and the File page's settings both list these.
pub fn ui_language_choices() -> &'static [(Option<&'static str>, &'static Command)] {
    static CHOICES: OnceLock<Vec<(Option<&'static str>, &'static Command)>> = OnceLock::new();
    CHOICES.get_or_init(|| {
        std::iter::once(None)
            .chain(UI_LANGUAGES.map(Some))
            .map(|lang| {
                let key = lang.unwrap_or("system");
                (lang, get(&format!("view.language.{key}")))
            })
            .collect()
    })
}

/// Every command, in ribbon order.
pub fn all() -> &'static [Command] {
    static ALL: OnceLock<Vec<Command>> = OnceLock::new();
    ALL.get_or_init(registry::build)
}

/// The command with this id.
///
/// # Panics
///
/// On an unknown id: a programmer error that the ribbon test catches.
pub fn get(id: &str) -> &'static Command {
    all()
        .iter()
        .find(|c| c.id == id)
        .unwrap_or_else(|| panic!("unknown command id `{id}`"))
}
