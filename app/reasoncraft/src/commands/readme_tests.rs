//! README's keyboard tables against the shortcut tables: the same keys,
//! each on the row of the action it runs.

use std::collections::BTreeSet;

use reasoncraft_core::{CardKind, ClearScope, SourceType};

use crate::actions::{Action, Dir, Rate};
use crate::canvas::input::{Os, format_shortcut, shortcuts};

/// README's "Keyboard shortcuts" section.
fn readme_section() -> &'static str {
    let readme = include_str!("../../../../README.md");
    let start = readme
        .find("## Keyboard shortcuts")
        .expect("README has a Keyboard shortcuts section");
    let section = &readme[start..];
    let end = section[3..].find("\n## ").map_or(section.len(), |i| i + 3);
    &section[..end]
}

/// The rows of README's keyboard tables, as their three cells; rows marked
/// "(context)" describe keys handled outside the tables and are left out.
fn readme_rows() -> Vec<[&'static str; 3]> {
    let mut rows = Vec::new();
    for line in readme_section().lines() {
        let line = line.trim();
        if !line.starts_with('|') || line.contains("(context)") {
            continue;
        }
        let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
        assert_eq!(cells.len(), 3, "a row has three columns: {line}");
        if cells[0] == "Windows / Linux" || cells[0].starts_with("---") {
            continue;
        }
        rows.push([cells[0], cells[1], cells[2]]);
    }
    rows
}

/// The keys of a README cell, by position: "F3 / Shift+F3" names two
/// actions; "Ctrl+Y or Ctrl+Shift+Z" names two keys for one.
fn cell_keys(cell: &str) -> Vec<Vec<&str>> {
    cell.split(" / ")
        .map(|p| p.split(" or ").map(str::trim).collect())
        .collect()
}

/// The shortcuts README's keyboard tables list in column `col` (0: Windows
/// and Linux, 1: macOS).
fn readme_column(col: usize) -> BTreeSet<String> {
    readme_rows()
        .iter()
        .flat_map(|row| cell_keys(row[col]).into_iter().flatten())
        .map(str::to_owned)
        .collect()
}

/// README's keyboard tables list exactly the shortcuts of each platform's
/// table, macOS's in its own column.
#[test]
fn readme_matches_both_tables() {
    for (col, os) in [(0, Os::Windows), (0, Os::Linux), (1, Os::Mac)] {
        let listed = readme_column(col);
        let table: BTreeSet<String> = shortcuts(os)
            .iter()
            .map(|(s, _)| format_shortcut(*s, os, false, "en"))
            .collect();
        let missing: Vec<_> = table.difference(&listed).collect();
        let extra: Vec<_> = listed.difference(&table).collect();
        assert!(
            missing.is_empty(),
            "{os:?} shortcuts missing from README: {missing:?}"
        );
        assert!(
            extra.is_empty(),
            "README rows not in {os:?}'s table: {extra:?}"
        );
    }
}

/// What a README row says it does: its action text up to the first
/// explanation, without the "Finish and" of the editing tables.
fn row_head(text: &str) -> String {
    let end = [" (", ":", ","]
        .iter()
        .filter_map(|p| text.find(p))
        .min()
        .unwrap_or(text.len());
    let head = text[..end].trim();
    let head = head.strip_prefix("Finish and ").unwrap_or(head);
    let head = head.strip_prefix("start a ").unwrap_or(head);
    let mut chars = head.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

/// The actions a README row's text names, in the order of its keys.
fn row_actions(head: &str) -> Option<Vec<Action>> {
    use Action as A;
    let reorder = || [Dir::Left, Dir::Right, Dir::Up, Dir::Down].map(A::Reorder);
    Some(match head {
        "Search commands" => vec![A::OpenCommandSearch],
        "Find in box text" => vec![A::OpenFind { replace: false }],
        "Find and replace" => vec![A::OpenFind { replace: true }],
        "Go to the next / previous hit" => vec![A::FindNext, A::FindPrev],
        "Spelling" => vec![A::OpenSpelling],
        "Move the keyboard focus to the next / previous pane" => vec![A::NextPane, A::PrevPane],
        "Help" => vec![A::ContextHelp],
        "Add a co-premise" => vec![A::AddCoPremise],
        "Add a reason for the whole card" => vec![A::TargetWholeCard(CardKind::Reason)],
        "Add an objection to the whole card" => vec![A::TargetWholeCard(CardKind::Objection)],
        "Add a source" => vec![A::AddSource(SourceType::Source)],
        "Edit the selected box" => vec![A::Edit],
        "Delete" => vec![A::Delete],
        "Delete with everything under it" => vec![A::DeleteBranch],
        "Jump between the main map and the parking area" => vec![A::JumpParking],
        "Open the comments of the selected box" => vec![A::CommentSelected],
        "Move to…" => vec![A::StartConnect],
        "Open the link of the selected box in the browser" => vec![A::OpenSelectedLink],
        "Move the selection one place among its siblings" => reorder().to_vec(),
        "Rate the selection high" => vec![A::Rate(Rate::High)],
        "Rate the selection mid" => vec![A::Rate(Rate::Mid)],
        "Rate the selection low" => vec![A::Rate(Rate::Low)],
        "Clear the selection's rating" => vec![A::Rate(Rate::Clear)],
        "Clear every rating in the map" => vec![A::ClearEvaluation(ClearScope::Map)],
        "Undo" => vec![A::Undo],
        "Redo" => vec![A::Redo],
        "Save" => vec![A::Save],
        "Save as" => vec![A::SaveAs],
        "Open" => vec![A::Open],
        "New map" => vec![A::New],
        "Zoom to 100%" => vec![A::Zoom100],
        "Zoom to fit" => vec![A::ZoomFit],
        "Zoom to fit the selection" => vec![A::ZoomFitSelection],
        "Zoom in" => vec![A::ZoomIn],
        "Zoom out" => vec![A::ZoomOut],
        _ => return None,
    })
}

/// Each key in README's tables runs the action its row describes, on
/// Windows and Linux and on macOS.
#[test]
fn readme_rows_name_their_actions() {
    for row in readme_rows() {
        let head = row_head(row[2]);
        let want = row_actions(&head)
            .unwrap_or_else(|| panic!("README row {head:?}: add it to `row_actions`"));
        for (col, os) in [(0, Os::Windows), (0, Os::Linux), (1, Os::Mac)] {
            let keys = cell_keys(row[col]);
            assert_eq!(keys.len(), want.len(), "{os:?} {head:?}: {keys:?}");
            for (alternatives, action) in keys.iter().zip(&want) {
                for key in alternatives {
                    let runs = shortcuts(os)
                        .iter()
                        .find(|(s, _)| format_shortcut(*s, os, false, "en") == *key)
                        .map(|(_, ctor)| ctor());
                    assert_eq!(runs.as_ref(), Some(action), "{os:?} {key} ({head:?})");
                }
            }
        }
    }
}
