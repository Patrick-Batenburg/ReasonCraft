//! The Home tab's commands: adding, editing and the box group.

use reasoncraft_core::{CardKind, SourceType};

use super::cmd;
use crate::actions::{Action, Dir};
use crate::commands::Command;
use crate::shell::icons::Icon;

pub(super) fn commands() -> Vec<Command> {
    use Action as A;
    let mut out = Vec::new();
    let add = "group.add";
    let clip = "group.clipboard";
    out.extend([
        cmd("home.paste", "action.paste", clip, A::Paste(None)).with_icon(Icon::Paste),
        cmd("home.cut", "action.cut", clip, A::Cut).with_icon(Icon::Cut),
        cmd("home.copy", "action.copy", clip, A::Copy).with_icon(Icon::Copy),
    ]);
    out.extend([
        cmd("home.add_reason", "action.add_reason", add, A::AddReason).with_icon(Icon::Reason),
        cmd(
            "home.add_objection",
            "action.add_objection",
            add,
            A::AddObjection,
        )
        .with_icon(Icon::Objection),
        cmd(
            "home.add_co_premise",
            "action.add_co_premise",
            add,
            A::AddCoPremise,
        )
        .with_icon(Icon::CoPremise),
        cmd(
            "home.add_category",
            "action.add_category",
            add,
            A::AddCategory,
        )
        .with_icon(Icon::Category),
        cmd("home.add_note", "action.add_note", add, A::AddNote).with_icon(Icon::Note),
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
        )
        .with_icon(Icon::Reason),
        cmd(
            "home.whole_card_objection",
            "command.whole_card_objection",
            add,
            A::TargetWholeCard(CardKind::Objection),
        )
        .with_icon(Icon::Objection),
        cmd("home.comment", "action.comment", add, A::CommentSelected).with_icon(Icon::Comment),
    ]);

    let editing = "group.editing";
    out.extend([
        cmd("home.edit", "command.edit", editing, A::Edit),
        cmd("home.delete", "action.delete", editing, A::Delete).with_icon(Icon::Delete),
        cmd(
            "home.delete_branch",
            "action.delete_branch",
            editing,
            A::DeleteBranch,
        )
        .with_icon(Icon::DeleteBranch),
        cmd("home.connect", "action.connect", editing, A::StartConnect).with_icon(Icon::Connect),
        cmd("home.park", "action.park", editing, A::MoveToParking).with_icon(Icon::Park),
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
        cmd("home.undo", "action.undo", editing, A::Undo).with_icon(Icon::Undo),
        cmd("home.redo", "action.redo", editing, A::Redo).with_icon(Icon::Redo),
        cmd(
            "home.find",
            "action.find",
            editing,
            A::OpenFind { replace: false },
        )
        .with_icon(Icon::Find),
        cmd(
            "home.replace",
            "action.replace",
            editing,
            A::OpenFind { replace: true },
        )
        .with_icon(Icon::Replace),
        cmd("home.find_next", "command.find_next", editing, A::FindNext),
        cmd("home.find_prev", "command.find_prev", editing, A::FindPrev),
        cmd("home.spelling", "action.spelling", editing, A::OpenSpelling).with_icon(Icon::Spelling),
    ]);

    let bx = "group.box";
    out.extend([
        cmd("home.link", "action.link", bx, A::EditLink).with_icon(Icon::Link),
        cmd(
            "home.open_link",
            "action.open_link",
            bx,
            A::OpenSelectedLink,
        )
        .with_icon(Icon::Link),
        cmd("home.image", "action.image", bx, A::InsertImage).with_icon(Icon::Image),
        cmd(
            "home.remove_image",
            "action.remove_image",
            bx,
            A::RemoveImage,
        )
        .with_icon(Icon::RemoveImage),
    ]);

    out
}
