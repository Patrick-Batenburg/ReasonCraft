//! The Help tab's commands: the help pages, About and the example maps.

use super::cmd;
use crate::actions::Action;
use crate::commands::Command;
use crate::help::HelpPage;
use crate::shell::icons::Icon;
use reasoncraft_core::examples::Example;

pub(super) fn commands() -> Vec<Command> {
    let help = "group.help";
    vec![
        cmd(
            "help.topics",
            "help.cmd.topics",
            help,
            Action::OpenHelp(HelpPage::Contents),
        )
        .with_icon(Icon::Help),
        cmd(
            "help.keyboard",
            "help.cmd.keyboard",
            help,
            Action::OpenHelp(HelpPage::Topic("keyboard")),
        )
        .with_icon(Icon::Keyboard),
        cmd("help.about", "help.cmd.about", help, Action::ShowAbout).with_icon(Icon::About),
        cmd(
            "help.example_reasoning",
            "example.reasoning",
            "group.examples",
            Action::OpenExample(Example::Reasoning),
        )
        .with_icon(Icon::Reason),
        cmd(
            "help.example_grouping",
            "example.grouping",
            "group.examples",
            Action::OpenExample(Example::Grouping),
        )
        .with_icon(Icon::Category),
    ]
}
