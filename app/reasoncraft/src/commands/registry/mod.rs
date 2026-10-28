//! The list of commands, in ribbon order, one file per ribbon tab.

use super::Command;
use crate::actions::Action;

mod evaluate;
mod export;
mod help;
mod home;
mod view;

pub(super) fn build() -> Vec<Command> {
    let file = "ribbon.tab.file";
    let mut out = vec![
        cmd("file.new", "action.new", file, Action::New),
        cmd("file.open", "action.open", file, Action::Open),
        cmd(
            "file.import_argdown",
            "action.import_argdown",
            file,
            Action::ImportArgdown,
        ),
        cmd("file.save", "action.save", file, Action::Save),
        cmd("file.save_as", "action.save_as", file, Action::SaveAs),
    ];
    out.extend(home::commands());
    out.extend(evaluate::commands());
    out.extend(view::commands());
    out.extend(export::commands());
    out.extend(help::commands());
    out
}

/// A command with this id, label key, group key and action.
fn cmd(id: &'static str, label: &'static str, group: &'static str, action: Action) -> Command {
    Command {
        id,
        label,
        group,
        icon: None,
        action,
    }
}
