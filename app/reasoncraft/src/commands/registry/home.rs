//! The Home tab's commands: adding, editing and the box group.

use reasoncraft_core::{CardKind, SourceType};

use super::cmd;
use crate::actions::{Action, Dir};
use crate::commands::Command;

pub(super) fn commands() -> Vec<Command> {
    use Action as A;
    let mut out = Vec::new();
    let add = "group.add";
    out.extend([
        cmd("home.add_reason", "action.add_reason", add, A::AddReason),
        cmd(
            "home.add_objection",
            "action.add_objection",
            add,
            A::AddObjection,
        ),
        cmd(
            "home.add_co_premise",
            "action.add_co_premise",
            add,
            A::AddCoPremise,
        ),
        cmd(
            "home.add_category",
            "action.add_category",
            add,
            A::AddCategory,
        ),
        cmd("home.add_note", "action.add_note", add, A::AddNote),
    ]);
    // Built once for the program's lifetime (`all` keeps them in a static).
    let leak = |s: String| -> &'static str { Box::leak(s.into_boxed_str()) };
    for st in SourceType::ALL {
        let key = st.key();
        out.push(cmd(
            leak(crate::commands::add_source_id(&st)),
            leak(format!("source.{key}")),
            "group.add_source",
            A::AddSource(st),
        ));
    }
    out.extend([
        cmd(
            "home.whole_card_reason",
            "command.whole_card_reason",
            add,
            A::TargetWholeCard(CardKind::Reason),
        ),
        cmd(
            "home.whole_card_objection",
            "command.whole_card_objection",
            add,
            A::TargetWholeCard(CardKind::Objection),
        ),
        cmd("home.comment", "action.comment", add, A::CommentSelected),
    ]);

    let editing = "group.editing";
    out.extend([
        cmd("home.edit", "command.edit", editing, A::Edit),
        cmd("home.delete", "action.delete", editing, A::Delete),
        cmd(
            "home.delete_branch",
            "action.delete_branch",
            editing,
            A::DeleteBranch,
        ),
        cmd("home.connect", "action.connect", editing, A::StartConnect),
        cmd(
            "home.jump_parking",
            "command.jump_parking",
            editing,
            A::JumpParking,
        ),
        cmd(
            "home.move_left",
            "command.move_left",
            editing,
            A::Reorder(Dir::Left),
        ),
        cmd(
            "home.move_right",
            "command.move_right",
            editing,
            A::Reorder(Dir::Right),
        ),
        cmd("home.undo", "action.undo", editing, A::Undo),
        cmd("home.redo", "action.redo", editing, A::Redo),
        cmd(
            "home.find",
            "action.find",
            editing,
            A::OpenFind { replace: false },
        ),
        cmd(
            "home.replace",
            "action.replace",
            editing,
            A::OpenFind { replace: true },
        ),
        cmd("home.find_next", "command.find_next", editing, A::FindNext),
        cmd("home.find_prev", "command.find_prev", editing, A::FindPrev),
        cmd("home.spelling", "action.spelling", editing, A::OpenSpelling),
    ]);

    let bx = "group.box";
    out.extend([
        cmd("home.link", "action.link", bx, A::EditLink),
        cmd("home.image", "action.image", bx, A::InsertImage),
        cmd(
            "home.remove_image",
            "action.remove_image",
            bx,
            A::RemoveImage,
        ),
    ]);

    out
}
