//! The Help tab's commands: the help pages, About and the example maps.

use super::cmd;
use crate::actions::Action;
use crate::commands::Command;
use crate::help::HelpPage;
use reasoncraft_core::examples::Example;

pub(super) fn commands() -> Vec<Command> {
    let help = "group.help";
    vec![
        cmd(
            "help.topics",
            "help.cmd.topics",
            help,
            Action::OpenHelp(HelpPage::Contents),
        ),
        cmd(
            "help.keyboard",
            "help.cmd.keyboard",
            help,
            Action::OpenHelp(HelpPage::Topic("keyboard")),
        ),
        cmd("help.about", "help.cmd.about", help, Action::ShowAbout),
        cmd(
            "help.example_reasoning",
            "example.reasoning",
            "group.examples",
            Action::OpenExample(Example::Reasoning),
        ),
        cmd(
            "help.example_grouping",
            "example.grouping",
            "group.examples",
            Action::OpenExample(Example::Grouping),
        ),
    ]
}
