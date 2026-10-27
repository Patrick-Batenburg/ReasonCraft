use std::collections::BTreeSet;

use eframe::egui::{Key, Modifiers};

use super::*;
use crate::canvas::input::{
    SHORTCUTS, format_shortcut, key_name, key_to_action, shortcut_for, shortcut_text,
};
use crate::i18n::t;

#[test]
fn command_ids_unique() {
    let mut seen = BTreeSet::new();
    for cmd in all() {
        assert!(seen.insert(cmd.id), "duplicate command id `{}`", cmd.id);
    }
    // One per source type, theme and export format.
    let count = |prefix: &str| all().iter().filter(|c| c.id.starts_with(prefix)).count();
    assert_eq!(count("home.add_source."), 17);
    assert_eq!(count("view.theme."), 3);
    assert_eq!(count("view.direction."), 3);
    for d in reasoncraft_core::LayoutDirection::ALL {
        let cmd = get(&format!("view.direction.{}", d.key()));
        assert_eq!(cmd.action, Action::SetLayoutDirection(d));
    }
    assert_eq!(count("view.language."), 1 + crate::i18n::UI_LANGUAGES.len());
    assert_eq!(
        get("view.language.system").action,
        Action::SetUiLanguage(None)
    );
    for lang in crate::i18n::UI_LANGUAGES {
        let cmd = get(&format!("view.language.{lang}"));
        assert_eq!(cmd.action, Action::SetUiLanguage(Some(lang.to_owned())));
    }
    assert_eq!(count("view.connector."), 3);
    for s in reasoncraft_core::ConnectorStyle::ALL {
        let cmd = get(&format!("view.connector.{}", s.key()));
        assert_eq!(cmd.action, Action::SetConnectorStyle(s));
    }
    assert_eq!(count("export.outline_") + count("export.essay_"), 6);
}

#[test]
fn every_label_and_group_is_translated() {
    for cmd in all() {
        for key in [cmd.label, cmd.group, cmd.tab()] {
            assert_ne!(t(key), key, "`{}`: no text for `{key}`", cmd.id);
        }
    }
}

#[test]
fn get_finds_by_id() {
    assert_eq!(get("view.zoom_100").action, Action::Zoom100);
}

/// The Home ribbon's Source ▾ menu looks these up when it opens.
#[test]
fn every_source_type_has_an_add_command() {
    for st in reasoncraft_core::SourceType::ALL {
        let cmd = get(&add_source_id(&st));
        assert_eq!(cmd.action, Action::AddSource(st));
    }
}

#[test]
#[should_panic(expected = "unknown command id")]
fn get_panics_on_unknown_id() {
    get("home.no_such_command");
}

#[test]
fn search_ranks_prefix_first() {
    assert_eq!(search("add r", 5)[0].id, "home.add_reason");
    assert_eq!(search("ADD R", 5)[0].id, "home.add_reason");
    assert_eq!(search("zoom 100", 5)[0].id, "view.zoom_100");
    // "r" starts a word of "Reason" but only sits inside "Category".
    let ids: Vec<_> = search("add r", 50).iter().map(|c| c.id).collect();
    let pos = |id| ids.iter().position(|&i| i == id).unwrap();
    assert!(pos("home.add_reason") < pos("home.add_category"));
    // The ribbon tab's name counts too.
    assert_eq!(search("view zoom", 5)[0].id, "view.zoom_fit");
    assert_eq!(search("home reason", 5)[0].id, "home.add_reason");
    assert_eq!(search("export png", 5)[0].id, "export.png");
    // Every word must match.
    assert!(search("add zebra", 5).is_empty());
    assert!(search("   ", 5).is_empty());
    assert_eq!(search("a", 3).len(), 3);
}

#[test]
fn search_ignores_diacritics() {
    // Query and labels fold the same way: "é" matches "E", "ó" matches "o".
    assert_eq!(search("évaluation shading", 5)[0].id, "evaluate.shading");
    assert_eq!(
        search("expert opinión", 5)[0].id,
        "home.add_source.expert_opinion"
    );
}

#[test]
fn shortcut_table_round_trips() {
    for (s, ctor) in SHORTCUTS {
        let action = ctor();
        assert_eq!(
            key_to_action(s.key, s.mods, false),
            Some(action.clone()),
            "{s:?}"
        );
        let shown = shortcut_for(&action).expect("an entry exists");
        assert_eq!(key_to_action(shown.key, shown.mods, false), Some(action));
    }
    // While editing, Alt+Q is the one Alt shortcut.
    let alt = Modifiers::ALT;
    assert_eq!(
        key_to_action(Key::Q, alt, true),
        Some(Action::OpenCommandSearch)
    );
    assert_eq!(key_to_action(Key::Num1, alt, true), None);
    // The first entry for an action is the one shown.
    assert_eq!(
        shortcut_for(&Action::Redo).map(|s| format_shortcut(s, false, "en")),
        Some("Ctrl+Y".to_owned())
    );
}

#[test]
fn shortcut_text_per_platform() {
    let s = shortcut_for(&Action::TargetWholeCard(reasoncraft_core::CardKind::Reason)).unwrap();
    assert_eq!(format_shortcut(s, false, "en"), "Ctrl+Shift+R");
    assert_eq!(format_shortcut(s, true, "en"), "⌘⇧R");
    let q = shortcut_for(&Action::OpenCommandSearch).unwrap();
    assert_eq!(format_shortcut(q, false, "en"), "Alt+Q");
}

#[test]
fn shortcut_text_uses_translated_key_names() {
    use crate::actions::Dir;
    use crate::i18n::{UI_LANGUAGES, t_lang};
    let left = shortcut_for(&Action::Reorder(Dir::Left)).unwrap();
    let co_premise = shortcut_for(&Action::AddCoPremise).unwrap();
    let branch = shortcut_for(&Action::DeleteBranch).unwrap();
    for lang in UI_LANGUAGES {
        let k = |key: &str| t_lang(lang, key);
        let joined = |a: &str, b: &str| format!("{}+{}", k(a), k(b));
        assert_eq!(
            format_shortcut(left, false, lang),
            joined("key.alt", "key.left")
        );
        assert_eq!(
            format_shortcut(co_premise, false, lang),
            joined("key.ctrl", "key.enter")
        );
        assert_eq!(
            format_shortcut(branch, false, lang),
            joined("key.shift", "key.delete")
        );
        for (key, name) in [(Key::Escape, "key.esc"), (Key::Space, "key.space")] {
            assert_eq!(key_name(key, lang), k(name));
            assert_ne!(k(name), name, "`{name}` has a text");
        }
    }
    assert_eq!(key_name(Key::Space, "nl"), "Spatie");
    assert_eq!(key_name(Key::R, "nl"), "R");
    // macOS keeps its modifier symbols.
    assert_eq!(format_shortcut(left, true, "en"), "⌥Left");
    // What menus show follows the UI language (English in this binary).
    let mac = cfg!(target_os = "macos");
    assert_eq!(shortcut_text(left), format_shortcut(left, mac, "en"));
}

/// README's keyboard tables list exactly the shortcuts of [`SHORTCUTS`];
/// rows marked "(context)" describe keys handled outside the table.
#[test]
fn readme_shortcuts_match_table() {
    let readme = include_str!("../../../../README.md");
    let start = readme
        .find("## Keyboard shortcuts")
        .expect("README has a Keyboard shortcuts section");
    let section = &readme[start..];
    let end = section[3..].find("\n## ").map_or(section.len(), |i| i + 3);
    let mut listed = BTreeSet::new();
    for line in section[..end].lines() {
        let line = line.trim();
        if !line.starts_with('|') || line.contains("(context)") {
            continue;
        }
        let keys = line.trim_matches('|').split('|').next().unwrap().trim();
        if keys == "Keys" || keys.starts_with("---") {
            continue;
        }
        for part in keys.split(" or ").flat_map(|p| p.split(" / ")) {
            listed.insert(part.trim().to_owned());
        }
    }
    let table: BTreeSet<String> = SHORTCUTS
        .iter()
        .map(|(s, _)| format_shortcut(*s, false, "en"))
        .collect();
    let missing: Vec<_> = table.difference(&listed).collect();
    let extra: Vec<_> = listed.difference(&table).collect();
    assert!(
        missing.is_empty(),
        "shortcuts missing from README: {missing:?}"
    );
    assert!(extra.is_empty(), "README rows not in SHORTCUTS: {extra:?}");
}
