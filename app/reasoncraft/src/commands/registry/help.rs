//! The Help tab's commands: the help pages and About.

use super::cmd;
use crate::actions::Action;
use crate::commands::Command;
use crate::help::HelpPage;

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
    ]
}
