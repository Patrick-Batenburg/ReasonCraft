//! The app's UI tests, one module per area, in one test binary (one
//! binary links and debug-builds much faster than one per file).
//!
//! Tests that switch the UI language live in `tests/ui_language/`, which
//! runs them under a lock (ruling S6-T2-1); everything here runs in
//! English. The other binaries are `tests/e2e/` (end-to-end flows) and
//! `tests/recovery/` (autosave and restore).

#[path = "../common/mod.rs"]
mod common;

mod accessibility;
mod arrange;
mod background_work;
mod backstage_keys;
mod backstage_small;
mod box_group;
mod building;
mod canvas_scene;
mod check;
mod clipboard;
mod close_request;
mod close_waits;
mod command_search;
mod connector_style;
mod deep_maps;
mod dialog_queue;
mod doc_tabs;
mod document;
mod essay;
mod essay_grouping;
mod examples;
mod export;
mod export_fixture;
mod export_formats;
mod export_replace;
mod field_shortcuts;
mod find;
mod find_bar_narrow;
mod help;
mod help_wrap;
mod images;
mod import_argdown;
mod launch;
mod layout_direction;
mod minimap;
mod outline;
mod panel_rows;
mod ribbon_collapsed;
mod ribbon_overflow;
mod ribbon_tips;
mod settings_enums;
mod shell_polish;
mod single_instance;
mod snippet_drag;
mod snippets;
mod spelling;
mod spelling_edit;
mod spelling_esc;
mod spelling_slices;
mod tab_fields;
mod text_panel;
mod ui_editing;
mod ui_files;
mod ui_navigation;
mod ui_ribbon;
mod ui_shell;
mod ui_small_window;
mod zoom_view;
