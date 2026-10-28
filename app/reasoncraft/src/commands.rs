//! Every command the ribbon, the backstage and the Search commands box
//! offer, in one registry: a stable id, a label and the action it runs.
//!
//! The ribbon and backstage take their labels and actions from here (their
//! icons stay with them); the title bar's Search commands box (Alt+Q)
//! searches it. Shortcuts live in one table, [`crate::canvas::input::SHORTCUTS`].

use std::sync::OnceLock;

use crate::actions::Action;

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
    pub action: Action,
}

impl Command {
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
