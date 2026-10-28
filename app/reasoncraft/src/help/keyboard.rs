//! The live shortcut table of the `keyboard` help topic: the keys of the
//! platform the app runs on, read from [`shortcuts`] itself, so the help
//! can never list a key the app does not have.

use reasoncraft_core::{CardKind, ClearScope};

use crate::actions::{Action, Dir, Rate};
use crate::canvas::input::{Os, shortcut_text, shortcuts};
use crate::i18n::t;

/// One row per command of `os`'s table, in table order: what the keys do
/// and the keys (all of a command's keys, joined), in the UI language.
pub fn rows(os: Os) -> Vec<(String, String)> {
    let mut rows: Vec<(Action, Vec<String>)> = Vec::new();
    for (s, ctor) in shortcuts(os) {
        let action = ctor();
        let keys = shortcut_text(*s, os);
        match rows.iter_mut().find(|(a, _)| *a == action) {
            Some((_, all)) => all.push(keys),
            None => rows.push((action, vec![keys])),
        }
    }
    rows.into_iter()
        .map(|(action, keys)| {
            let what = description(&action).map_or_else(|| format!("{action:?}"), t);
            (what, keys.join(", "))
        })
        .collect()
}

/// The i18n key that says what a shortcut's `action` does.
fn description(action: &Action) -> Option<&'static str> {
    use Action as A;
    Some(match action {
        A::AddCoPremise => "help.key.co_premise",
        A::TargetWholeCard(CardKind::Reason) => "help.key.whole_card_reason",
        A::TargetWholeCard(CardKind::Objection) => "help.key.whole_card_objection",
        A::AddSource(_) => "help.key.source",
        A::Edit => "help.key.edit",
        A::Delete => "help.key.delete",
        A::DeleteBranch => "help.key.delete_branch",
        A::JumpParking => "help.key.jump_parking",
        A::CommentSelected => "help.key.comments",
        A::StartConnect => "help.key.connect",
        A::OpenSelectedLink => "help.key.open_link",
        A::Reorder(Dir::Left) => "help.key.move_left",
        A::Reorder(Dir::Right) => "help.key.move_right",
        A::Reorder(Dir::Up) => "help.key.move_up",
        A::Reorder(Dir::Down) => "help.key.move_down",
        A::Rate(Rate::High) => "help.key.rate_high",
        A::Rate(Rate::Mid) => "help.key.rate_mid",
        A::Rate(Rate::Low) => "help.key.rate_low",
        A::Rate(Rate::Clear) => "help.key.rate_clear",
        A::ClearEvaluation(ClearScope::Map) => "help.key.clear_map",
        A::Undo => "help.key.undo",
        A::Redo => "help.key.redo",
        A::Save => "help.key.save",
        A::SaveAs => "help.key.save_as",
        A::Open => "help.key.open",
        A::New => "help.key.new",
        A::Zoom100 => "help.key.zoom_100",
        A::ZoomFit => "help.key.zoom_fit",
        A::ZoomFitSelection => "help.key.zoom_fit_selection",
        A::ZoomIn => "help.key.zoom_in",
        A::ZoomOut => "help.key.zoom_out",
        A::OpenCommandSearch => "help.key.search_commands",
        A::OpenFind { replace: false } => "help.key.find",
        A::OpenFind { replace: true } => "help.key.replace",
        A::FindNext => "help.key.find_next",
        A::FindPrev => "help.key.find_prev",
        A::OpenSpelling => "help.key.spelling",
        A::NextPane => "help.key.next_pane",
        A::PrevPane => "help.key.prev_pane",
        A::ContextHelp => "help.key.help",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{UI_LANGUAGES, t_lang};

    /// Every shortcut of every platform says what it does, in every UI
    /// language.
    #[test]
    fn every_shortcut_is_described() {
        for os in [Os::Windows, Os::Mac, Os::Linux] {
            for (s, ctor) in shortcuts(os) {
                let key = description(&ctor()).unwrap_or_else(|| panic!("{os:?} {s:?}"));
                for lang in UI_LANGUAGES {
                    assert_ne!(t_lang(lang, key), key, "{lang}: `{key}`");
                }
            }
        }
        // Redo has two keys on one row.
        let redo = t("help.key.redo");
        let rows = rows(Os::Windows);
        let (_, keys) = rows.iter().find(|(what, _)| *what == redo).unwrap();
        assert_eq!(keys, "Ctrl+Y, Ctrl+Shift+Z");
    }
}
