# Step 6: Release readiness: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make ReasonCraft ready to hand to others:
- an English and Dutch interface, switchable at run time;
- map labels in the document's language;
- an accessibility pass;
- OS-correct keyboard shortcuts;
- offline help, example maps, templates and recent files;
- an About page with licence notices;
- installers for Windows, macOS and Linux, with `.rcraft` file association;
- the remaining carry-overs from steps 1–5.

**Architecture:**
- The i18n module becomes a set of bundles: English and Dutch both load, the UI language can be swapped with a lock, and `t_lang(lang, key)` serves text in the document's language to the scene and to export workers.
- Help pages, example maps and templates are embedded resources; nothing is downloaded.
- Packaging uses `cargo-packager` configured in the app's `Cargo.toml`:
  - the Windows installer is built and verified here (installed into the owner's Hyper-V test VM, never on the host);
  - Linux is built and tested in a Docker container;
  - macOS is configured and built by a CI workflow file that runs once a remote exists.

**Tech stack:**
- Rust 1.99.1 and egui/eframe 0.37, as before.
- New: `sys-locale` 0.3 (MIT/Apache) for the OS language, `cargo-packager` (a dev tool, not a dependency), and `embed-resource` or `winresource` (MIT) for the Windows exe icon.

**Spec:** `docs/superpowers/specs/2026-10-25-reasoncraft-design.md`, sections §6 (File backstage: New from template, Recent, About; Help), §10 (languages, themes, accessibility, settings), §11 (bundled content), §12 (distribution) and §13 step 6.

## Global Constraints

- **Platform.** Pure native egui, no webview. Windows, macOS and Linux. Fully offline: help, examples, templates and dictionaries are bundled, and nothing is fetched at run time.
- **Strings.**
  - All UI text goes through `i18n::t` / `t_args`.
  - `nl.ftl` must contain exactly the keys of `en.ftl`, with the same placeables. This is enforced by a test from Task 4 on.
  - Text drawn on the map, or written into exports, uses the document's language (`doc.meta.language`). The UI uses the UI language.
- **Colours.** All colours come from `theme::Tokens`. Text meets WCAG AA (4.5:1, or 3:1 for text of 18 px and up) in both themes. This is enforced by the existing token-contrast tests, extended in Task 5.
- **Mutation.** `actions::apply` is the only mutator. Document changes go through `DocTab::edit`, which records one history entry, or none for a refused or no-op change.
- **File format.**
  - `SCHEMA_VERSION` stays 1, and files from steps 1–5 open unchanged.
  - Example maps and templates are ordinary documents, opened as new untitled tabs.
- **Settings.**
  - New fields use `#[serde(default)]`.
  - Unknown enum values never reset the whole settings file; a test enforces this from Task 1 on.
  - "Reset to defaults" restores every field except the recent-files list and the personal dictionary.
- **Licences.**
  - New run-time dependencies must be MIT, Apache-2.0, BSD, ISC or Zlib.
  - `THIRD-PARTY.md` lists every bundled third-party asset, and the About page shows it.
- **No synthetic OS input.** Never inject OS-level keyboard or mouse input on the host.
  - Verify with kittests, files and window captures.
  - Installer checks run only inside the owner's Hyper-V VM "Windows 11 Sandbox":
    - use PowerShell Direct with the saved credential `%USERPROFILE%\.vm-sandbox-cred.xml`;
    - take a checkpoint first;
    - revert it and turn the VM off afterwards.
- **Tests stay sandboxed.** Tests never touch the real clipboard, file viewers, the user's recovery folder, eframe settings or the OS locale. Inject the locale.
- **Quality gate.**
  - `cargo clippy --workspace --all-targets -j 4 -- -D warnings`, `cargo fmt --check` and `cargo test --workspace -j 4` pass at every commit. Always use `-j 4`: parallel MSVC linking exhausts memory.
  - Files stay at roughly 400 lines or fewer.

## Review Focus

1. **Switching the UI language at run time must not leave stale text.** That includes cached galleys, ribbon widths, command search, open modals and the Outline/Essay caches keyed by revision.
   - It must never change a document. Map labels follow the document, not the UI.
   - Test: Task 2 `switching_language_updates_ribbon_and_search`. Task 3 `map_labels_follow_document_language_not_ui`.
2. **Translations must never break formatting.** Every `nl.ftl` message parses, has the same placeables, and fits its widget. The long Dutch compound words are the risk (ribbon labels, buttons).
   - Test: Task 4 `nl_has_same_keys_and_placeables`, `ribbon_fits_in_dutch_at_1280`.
3. **The keyboard must reach everything.** Ribbon, panels, find bar, spelling panel, command search, minimap alternative, modals and virtualised rows. Screen-reader names must be unique and meaningful.
   - Test: Task 5 `every_interactive_widget_has_a_name`, `tab_cycles_through_find_bar`, `virtual_rows_reachable_by_keyboard`.
4. **The installer must associate `.rcraft` and open a double-clicked file.** It must uninstall cleanly and leave the user's documents, settings and recovery folder alone.
   - Check: Task 11 VM install script with assertions on registry keys and files, recorded in the task report.
5. **Example maps and templates must be valid and open as untitled copies.** They must never overwrite anything, and must be in the chosen language.
   - Test: Task 10 `examples_and_templates_validate_in_both_languages`, `opening_example_creates_untitled_dirty_free_tab`.

---

### Task 1: Housekeeping: file splits and tolerant settings

**Files:**
- Split into submodules, or move tests out, so each file is ≤ ~400 lines with room to grow:
  - `app/reasoncraft/src/file_ops.rs`, `app/reasoncraft/src/actions.rs`
  - `crates/reasoncraft_core/src/argdown/build.rs`
  - `app/reasoncraft/src/shell/text_panel/check.rs`
  - `app/reasoncraft/tests/ui_editing.rs`, `app/reasoncraft/tests/e2e/main.rs`
  - `app/reasoncraft/src/canvas/input/tests.rs`
- Modify `app/reasoncraft/src/settings.rs` and the theme/connector/minimap enums.

**Interfaces:**
- Produces: `fn deserialize_or_default<'de, D, T: Deserialize<'de> + Default>(d: D) -> Result<T, D::Error>` in `settings.rs`, used as `#[serde(deserialize_with = "deserialize_or_default")]` on every enum-typed settings field: `theme`, `connector_style`, `minimap`, `text_tab`, `export.theme`, `export.scope`, page setup enums.
  - An unknown variant becomes that field's default; the rest of the file still loads.
  - It must work for RON, which eframe uses, and JSON.

- [ ] **Step 1:** Pure moves first. Run `cargo test --workspace -j 4` and expect the same count (755 passed).
- [ ] **Step 2: Failing test** `unknown_enum_values_keep_other_settings`. In RON, `(author:"Ann",theme:Sepia,connector_style:Zigzag,minimap:Huge)` loads with author "Ann" and the default theme, connector style and minimap. Add the same test in JSON.
- [ ] **Step 3:** Implement. **Step 4:** Run the gate. **Step 5:** Commit `refactor: split large files; settings tolerate unknown enum values`.

### Task 2: Swappable i18n and the UI language

Spec §10: "English and Dutch at launch … Default follows the OS language, switchable in View/Settings."

Carry-overs:
- swappable bundle;
- translated key names in shortcut text;
- stop parsing `●` out of ftl strings (dot count in code);
- store an empty comment author and translate it on display.

**Files:** `app/reasoncraft/src/i18n.rs` → `i18n/{mod.rs,bundles.rs,keys.rs}`; create `app/reasoncraft/i18n/nl.ftl` (with the `app-name` key only; Task 4 fills it). Modify `settings.rs`, `main.rs`, `app.rs`, `shell/ribbon/view.rs`, `shell/backstage.rs` (settings page), `canvas/input/shortcuts.rs`, `canvas/rating.rs` (dots) and `canvas/comments*` (author), plus `Cargo.toml` (`sys-locale = "0.3"`).

**Interfaces:**
- Produces:
  ```rust
  pub const UI_LANGUAGES: [&str; 2] = ["en", "nl"];
  pub fn set_ui_language(lang: &str)            // swaps the active bundle (RwLock); unknown → "en"
  pub fn ui_language() -> &'static str
  pub fn t(key: &str) -> String                 // active UI language, falls back to English per key
  pub fn t_args(key: &str, args: &FluentArgs) -> String
  pub fn t_lang(lang: &str, key: &str) -> String            // explicit language, falls back to English
  pub fn t_lang_args(lang: &str, key: &str, args: &FluentArgs) -> String
  pub fn os_ui_language(locale: Option<&str>) -> &'static str   // "nl" for nl, nl-NL, nl-BE…, else "en"; pure, testable
  // Settings: ui_language: Option<String> (None = follow OS), ui_scale: f32 (Task 5)
  Action::SetUiLanguage(Option<String>)        // settings only, no history; commands view.language.{system,en,nl}
  ```
- Language change takes effect immediately: the next frame redraws everything.
  - Ribbon widths are measured, not cached by string.
  - The command search rebuilds its haystack on language change. Key it by `ui_language()`.
  - The Outline and Check caches must not store translated UI text, or they must key it by language.
- Shortcut text uses translated key names (`key.ctrl`, `key.shift`, `key.alt`, `key.esc`, `key.left`, `key.enter`, `key.space`, …).
- Rating dots are drawn by count in code: two for strong, one for weak. The label text comes from the ftl without `●`.
- A comment author stored as `""` is shown as `t("comment.anonymous")`. New comments with no author setting store `""`.
  - Old files that hold the literal English "Anonymous" keep it. Don't rewrite data.

- [ ] **Step 1: Failing tests:**
  - `os_language_maps_dutch_variants`: `nl`, `nl-NL`, `nl_BE.UTF-8` → `"nl"`; `de-DE`, `None` → `"en"`.
  - `switching_language_updates_ribbon_and_search`: in a kittest, set nl and check that the Home tab label equals `t_lang("nl","ribbon.tab.home")`. Use a key that Task 2 adds to `nl.ftl` with a Dutch value, such as `ribbon.tab.home = Start`. Command search for "Start" finds Home commands.
  - `missing_dutch_key_falls_back_to_english`.
  - `rating_dots_drawn_by_count_not_parsed`: ftl labels contain no `●`.
  - `empty_author_shown_translated`.
  - `shortcut_text_uses_translated_key_names`.
- [ ] **Step 2:** Run them; expect FAIL.
- [ ] **Step 3: Implement.**
  - View → Language menu: System, English, Nederlands. Language names are shown in their own language.
  - The settings page has the same choice.
  - `main.rs` reads the OS locale with `sys_locale::get_locale()` at start-up, only when `ui_language` is `None`.
- [ ] **Step 4:** Run the gate.
- [ ] **Step 5:** Commit `feat(i18n): swappable UI language with OS default`.

### Task 3: Labels in the document's language

Carry-overs:
- map labels and source names by document language, on the canvas, in exports and in the export worker;
- one home for source-type names;
- Compound help by document language.

**Files:**
- `app/reasoncraft/src/scene/{build,boxes,mod}.rs`, `app/reasoncraft/src/export/*` (worker labels), `app/reasoncraft/src/canvas/images/*` (missing-image label)
- `crates/reasoncraft_core/src/words.rs`, `app/reasoncraft/i18n/en.ftl` (source names), `shell/text_panel/check.rs` (Compound help)

**Interfaces:**
- Produces:
  - `SceneOptions.lang: &'static str`, the document language. The scene builds every drawn word with `t_lang(opts.lang, …)`: card labels, the parking title, the missing-image label, source type names and the numbering prefix if any.
  - Source-type names live in the ftl files only (`source.<key>`). `words::source_word` is removed. Core text exports (outline, essay, Argdown headings) take a `&dyn Fn(&str) -> String` label lookup, or the app passes resolved strings. Choose one and apply it to all callers.
  - Core keeps only linking words (`because`/`omdat`, …), which are core logic used in tests without the app. Document this in `words.rs`.

- [ ] **Step 1: Failing tests:**
  - `map_labels_follow_document_language_not_ui`: UI en, doc nl gives the scene text "steunt sterk" (or whatever Task 4 chooses; read it from `t_lang("nl", key)`). UI nl, doc en gives "strongly supports".
  - `export_worker_uses_document_language`: a PNG/SVG export of a Dutch document contains the Dutch label text in its SVG.
  - `source_names_single_source`: grep-style test that `words.rs` has no source names.
  - `compound_help_in_document_language`.
- [ ] **Step 2:** Run; FAIL. **Step 3:** Implement. **Step 4:** Gate. **Step 5:** Commit `feat(i18n): map labels and exports use the document language`.

### Task 4: Dutch translation

**Files:** `app/reasoncraft/i18n/nl.ftl` (every key) and `app/reasoncraft/src/i18n/tests.rs`; modify `app/reasoncraft/src/export/docx.rs` (language tag map).

**Interfaces:**
- Translation rules:
  - Plain, friendly Dutch for students and teachers, using "je" (informal).
  - Use the terms already fixed in core and docs: claim = "bewering", reason = "reden", objection = "bezwaar", rebuttal = "weerlegging", co-premise = "medepremisse", premise = "premisse", source = "bron", grouping = "groepering".
  - Rating labels: "steunt sterk", "steunt zwak", "steunt niet", "weerlegt …", "spreekt tegen …".
  - Linking words: "omdat", "maar", "echter", as in core `words.rs`.
  - Keep each label short enough to fit its control.
- Record any term choice not listed here in the task report's glossary. The owner can adjust these later.
- docx: language tag map `en` → `en-GB`, `nl` → `nl-NL`, with an explicit match so a future third language must be added.

- [ ] **Step 1: Failing tests:**
  - `nl_has_same_keys_and_placeables`: parse both files; the key sets are equal; for each key the placeable names (`{ $count }`, …) are equal.
  - `nl_differs_from_en_for_most_keys`: at least 90% of values differ. This catches untranslated copies, and an allow-list covers values that are legitimately the same, such as "PNG", "SVG", "PDF" and "Argdown".
  - `ribbon_fits_in_dutch_at_1280`: a kittest at 1280×800 with nl. Every ribbon tab's groups fit without clipping: each button's text rect is within its button rect, and the ribbon's content width is no more than the window width.
  - `dutch_plural_forms`: `t_lang_args("nl", "check.not_rated", count 1/2)` returns the singular/plural forms.
- [ ] **Step 2:** Run; FAIL. **Step 3:** Translate. **Step 4:** Gate, plus a window capture of the Dutch UI with a sample map (see the Task 13 capture method) into the SDD folder. Look at it. **Step 5:** Commit `feat(i18n): Dutch interface`.

### Task 5: Accessibility, UI scale and reset to defaults

Spec §10:
- "full keyboard navigation and screen-reader support through egui's AccessKit integration";
- "UI scale / font size setting";
- "reset to defaults".

Carry-overs:
- names for the new widgets: find bar `Aa`/`W`, minimap, spelling panel, command list, Ignore buttons, import report, export dialog;
- duplicate accessible names (Nil, panel vs ribbon);
- blank orphan essay labels;
- keyboard access to off-screen virtual rows;
- the Check tab's `keep`;
- close-button glyph contrast;
- Unicode Cf characters in links.

**Files:** create `app/reasoncraft/src/a11y.rs` (helpers) and `app/reasoncraft/tests/accessibility.rs`; modify the widgets listed, `settings.rs` (`ui_scale`), `app.rs` (apply `ctx.set_zoom_factor`), `shell/backstage.rs` (settings page: scale slider 80–200% in 10% steps; "Reset to defaults" button with confirmation), `theme.rs` (contrast), `core/link.rs` (refuse or strip Cf).

**Interfaces:**
- Produces: `fn labelled(response: Response, name: impl Into<String>) -> Response` in `a11y.rs`, which sets the AccessKit label through `response.widget_info`. Glyph-only buttons must use it.
- **Minimap keyboard alternative:** the commands `view.pan.{left,right,up,down}` (Ctrl+Alt+arrows; on macOS Ctrl+Cmd+arrows per Task 6) and "Fit map" are already accessible. The minimap is marked as a decorative image with the name `t("minimap.name")`.
- `Action::ResetSettings` shows a confirmation modal. On Reset it restores defaults except `recent_files` and `spell_words`.

- [ ] **Step 1: Failing tests:**
  - `every_interactive_widget_has_a_name`: walk the AccessKit tree in a kittest with all panels, the find bar, the spelling panel and command search open. Every node with an action (click, focus) has a non-empty name, and no two siblings share role and name.
  - `tab_cycles_through_find_bar`.
  - `virtual_rows_reachable_by_keyboard`: with 400 boxes in the Essay tab, Tab from the last visible field reaches the next field, and the scroll follows.
  - `ui_scale_applies_and_persists`.
  - `reset_to_defaults_keeps_recent_and_dictionary`.
  - `cf_characters_refused_in_links`: U+202E in a URL is refused.
  - `contrast_of_all_text_tokens_aa`: extend the existing test to the close glyph and any new tokens.
- [ ] **Step 2:** Run; FAIL. **Step 3:** Implement. **Step 4:** Gate. **Step 5:** Commit `feat(a11y): names, keyboard reach, UI scale, reset to defaults`.

### Task 6: OS-correct keyboard shortcuts

Carry-overs:
- macOS: Cmd+H hides the app, so Replace needs another key (ruling F3);
- Alt+digits and Alt+Q type characters on macOS;
- Backspace deletes on macOS;
- Cmd+Q goes through the unsaved-changes prompt;
- ⌘ glyphs are visible.

**Files:** `canvas/input/shortcuts.rs` (per-OS table), `canvas/input.rs`, `app.rs` (close request on Cmd+Q), `README.md` (per-OS column), and `shell/command_search.rs` (shortcut text).

**Interfaces:**
- `SHORTCUTS` gains a platform dimension: `fn shortcuts(os: Os) -> &'static [(Shortcut, ActionCtor)]` with `enum Os { Windows, Mac, Linux }`, plus `Os::current()`.
- The macOS table is derived by these rules:
  - every `Alt+X` becomes `Ctrl+X`, because Control is free on macOS;
  - Replace is `Cmd+Alt+F`;
  - Search commands is `Cmd+Shift+A`, checked against the table so it is unique;
  - Backspace, like Delete, deletes the selected box when not editing.
- Every other OS keeps the current table.
- `shortcut_text(s, Os::Mac)` uses ⌘ ⌥ ⇧ ⌃ only if the bundled UI font has those glyphs; check with the font. Otherwise use "Cmd", "Opt", "Shift" and "Ctrl".
- Cmd+Q / the window close request always goes through `request_close_window`. Test that the unsaved prompt appears.
- README: the shortcut table gets "Windows / Linux" and "macOS" columns, and `readme_shortcuts_match_table` checks both.

- [ ] **Step 1: Failing tests:**
  - `mac_table_has_no_alt_letter_or_digit`
  - `mac_replace_is_cmd_alt_f`
  - `tables_have_unique_shortcuts_per_os`
  - `backspace_deletes_on_mac`
  - `close_request_prompts_when_unsaved`
  - `readme_matches_both_tables`
  - `mac_glyphs_only_if_font_has_them`
- [ ] **Step 2:** Run; FAIL. **Step 3:** Implement. **Step 4:** Gate. **Step 5:** Commit `feat: per-OS shortcut tables (macOS keys)`.

### Task 7: Map and panel polish

Carry-overs:
- spread arrowheads when many children attach to one premise;
- left-to-right port spacing (opposes bars overlap at about 5.7 units);
- a rated premise is 16 units taller than an unrated co-premise (text misaligned);
- long tab titles truncate;
- panel min width;
- drag polish (snippet card follows the pointer, autoscroll);
- scroll shift when unmeasured rows above the view get measured.

**Files:** `crates/reasoncraft_core/src/layout/{ports,indicators,card}.rs`, `shell/tabs*.rs`, `shell/panels*.rs`, `shell/text_panel/virtual_rows.rs` and `canvas/drag*.rs`.

**Interfaces:**
- **Ports:**
  - The minimum spacing between ports on one edge is `port_spacing` (16).
  - When the edge is too short for that, the attach point is the edge centre and the stubs fan out. Each stub ends at the edge but approaches at an angle, so the arrowheads still don't overlap.
  - Test: the arrowhead triangles of 9 connectors into one premise don't intersect, in all directions.
- **Indicator row:** when any premise of a card shows an indicator row, every premise of that card reserves it, so the first text lines align.
- **Tabs:** titles longer than 24 characters are shown truncated with "…"; the full title is in the tooltip.
- **Panels:**
  - Each side panel has a minimum width of 220 px.
  - Below that, the panel collapses to its toggle.
- **Drag:**
  - A snippet drag shows a ghost card following the pointer.
  - The canvas autoscrolls when the pointer is within 32 px of an edge, at 600 px/s.
- **Virtual rows:** when rows above the view are measured, the scroll offset is adjusted by the sum of their height deltas, so visible content doesn't move.

- [ ] **Step 1: Failing tests:**
  - `nine_arrowheads_do_not_overlap_any_direction`
  - `co_premise_texts_align_when_one_is_rated`
  - `long_tab_title_truncates_with_tooltip`
  - `side_panel_min_width`
  - `snippet_drag_ghost_follows_pointer`
  - `drag_autoscrolls_near_edge`
  - `virtual_rows_do_not_jump_when_measured_above`
- [ ] **Step 2:** Run; FAIL. **Step 3:** Implement. **Step 4:** Gate, plus a dense LeftRight Elbow sample PNG in the SDD folder. Look at it. **Step 5:** Commit `fix: arrowhead spreading, aligned co-premises, tab/panel/drag polish`.

### Task 8: Robustness and performance carry-overs

The carry-overs:
- spelling suggestions off the UI thread;
- underline cache per line;
- imported box text cap with a report note;
- PNG cap memory check;
- print PermissionDenied retry;
- recovery `autosaved_revision` as an `Option`;
- `then_discard` kept across a retry;
- shutdown keeps the lock if a delete fails;
- resvg/usvg upgrade together with svg2pdf, if a compatible pair exists.

**Files:** `app/reasoncraft/src/spell/{service,pass,layouter}.rs`, `crates/reasoncraft_core/src/argdown/{build,report}.rs`, `app/reasoncraft/src/export/{raster,pdf,job}.rs`, `app/reasoncraft/src/recovery/{session,store,disk}.rs` and `Cargo.toml`.

**Interfaces:**
- **Spelling suggestions:**
  - They run on the spell worker: `SpellService::suggest_async(lang, word) -> Option<Vec<String>>`, which returns `None` while computing.
  - The panel shows "…" until they arrive.
  - The F7 pass itself never blocks for more than 10 ms per frame: process boxes in slices.
- **Underline cache:** keyed per line, `(lang, hash(line))`, so typing re-checks one line.
- **Import:** box text longer than `MAX_BOX_CHARS` is cut and noted as `ImportNoteKind::TextCut`.
- **PNG export:**
  - Before allocating, estimate `w*h*4`.
  - If that is over 512 MiB, refuse with the existing cap message.
  - The cap stays at 256 MP, but the memory guard is checked first.
- **Print temp file:** on Windows, a `PermissionDenied` while creating the file tries the next name, the same as `AlreadyExists`.
- **Recovery:**
  - `DocTab.autosaved_revision: Option<u64>`.
  - A failed write keeps its `then_discard` for the retry.
  - Shutdown keeps `session.lock` if any delete failed, so the next start can tidy the folder.
- **resvg/svg2pdf:**
  - Check crates.io for an svg2pdf release that depends on the newest usvg.
  - If there is one, upgrade all three and run the export tests.
  - If not, record that in the report and leave them as they are.

- [ ] **Step 1: Failing tests:**
  - `suggestions_never_block_ui`
  - `f7_pass_slices_work` (the max per-frame time is reported through a test hook)
  - `typing_rechecks_one_line`
  - `import_caps_long_text_with_note`
  - `png_export_refuses_over_memory_guard`
  - `print_name_retries_on_permission_denied` (inject the error)
  - `retry_keeps_then_discard`
  - `failed_shutdown_delete_keeps_lock`
- [ ] **Step 2:** Run; FAIL. **Step 3:** Implement. **Step 4:** Gate plus `--release -- --ignored`. **Step 5:** Commit `fix: spelling off the UI thread, safer exports, recovery retries`.

### Task 9: Offline help and About

Spec §11: "Offline help pages covering concepts (claim, premise, co-premise, objection, rebuttal, source types, evaluation, the key-term check) and how-tos."

Spec §6: File backstage "About" and a Help ribbon tab.

**Files:**
- Create `app/reasoncraft/help/{en,nl}/*.md` (embedded with `include_str!` through a generated index), `app/reasoncraft/src/help/{mod,render,index}.rs` and `app/reasoncraft/src/shell/about.rs`.
- Modify the Help ribbon tab (replace "coming soon"), `shell/backstage.rs` (About page) and the Check tab rows (a "?" link to the topic).

**Interfaces:**
- **Topics**, with file stems as ids:
  - `getting-started`, `claims-and-premises`, `reasons-and-objections`, `co-premises`, `rebuttals`;
  - `whole-card-vs-premise`, `sources`, `evaluation`, `checks` (the key-term check explained), `grouping-maps`;
  - `outline-and-essay`, `snippets`, `exports-and-print`, `argdown`, `keyboard`, `files-and-recovery`, `languages`.
- Each topic exists in English and Dutch. The Dutch versions are written in Dutch, not machine-literal.
- **Renderer:**
  - It supports a Markdown subset: `#`/`##` headings, paragraphs, `-` lists, `**bold**`, `` `code` ``, and links `[text](topic-id)` between topics.
  - Anything else renders as plain text.
  - There is no HTML and no external links.
- **Help panel:**
  - It opens in the right text panel as a "Help" tab, in the UI language.
  - F1 opens `getting-started`, or the topic for the focused area: Check tab → `checks`, Essay → `outline-and-essay`.
  - The Help ribbon tab offers: Help topics, Keyboard shortcuts (the `keyboard` topic, which lists the live table from `shortcuts(Os::current())` rather than static text), Example maps (Task 10) and About.
- **About page** (backstage):
  - app name, version (`env!("CARGO_PKG_VERSION")`) and licence "MIT OR Apache-2.0";
  - the contents of `THIRD-PARTY.md` (embedded);
  - credits to the dictionary projects.
  - No links to external websites are required. If a link is shown, it is plain text.

- [ ] **Step 1: Failing tests:**
  - `every_topic_exists_in_both_languages`
  - `topic_links_resolve`
  - `renderer_ignores_unknown_markup`
  - `f1_opens_contextual_topic`
  - `keyboard_topic_lists_live_shortcuts`
  - `about_shows_third_party_notices`
- [ ] **Step 2:** Run; FAIL. **Step 3:** Write the content and implement. **Step 4:** Gate. **Step 5:** Commit `feat: offline help in English and Dutch; About page`.

### Task 10: Example maps, templates and recent files

Spec:
- §11: "Example maps written for ReasonCraft … a reasoning map and a grouping map on a neutral topic, used by Help → Example maps and the templates."
- §6: File → "New (blank or from template)", "Recent".
- Step 2 carry-over: the building panel's Templates section.

**Files:** create `crates/reasoncraft_core/src/examples.rs` and `app/reasoncraft/src/actions/examples.rs`; modify `shell/backstage.rs` (New page with templates; Recent list), `shell/building.rs` (Templates section), `settings.rs` (`recent_files`) and `file_ops*` (push on open and save).

**Interfaces:**
- Produces, in core:
  ```rust
  pub enum Example { Reasoning, Grouping }
  pub enum Template { BlankReasoning, BlankGrouping, EssayPlan, PeerReview }
  pub fn example(which: Example, lang: &str) -> Document      // built through public edit ops
  pub fn template(which: Template, lang: &str) -> Document
  ```
- **Example topics** (neutral):
  - Reasoning: "Cities should make public transport free" / "Steden moeten het openbaar vervoer gratis maken". It has 3 reasons and 2 objections, with co-premises, a rebuttal, a whole-card objection, sources of several types, ratings and a note.
  - Grouping: "Ways to reduce food waste" / "Manieren om voedselverspilling te verminderen", three levels deep.
- **Templates:**
  - BlankReasoning: a claim placeholder plus one reason and one objection, with placeholder text in the language.
  - BlankGrouping: a topic plus three categories.
  - EssayPlan: a claim, 3 reasons, 1 objection with a rebuttal, and the essay intro/conclusion prompts.
  - PeerReview: a claim with comment presets explained in a note.
- **Opening:** examples and templates open as a new untitled tab marked clean (not dirty), with `doc.meta.language = lang`. Their language follows the UI language at the moment they are opened.
- **Recent files:**
  - `Settings.recent_files: Vec<PathBuf>`, newest first, max 10, pushed on a successful open or save.
  - The backstage Recent list shows the file name and folder.
  - Clicking a missing file shows the existing open error and removes the file from the list.

- [ ] **Step 1: Failing tests:**
  - `examples_and_templates_validate_in_both_languages` (and `run_checks` gives no NotConnected issues)
  - `opening_example_creates_untitled_clean_tab`
  - `template_language_follows_ui`
  - `recent_files_capped_and_deduplicated`
  - `missing_recent_file_removed_with_error`
  - `building_panel_templates_insert_as_new_tab`
- [ ] **Step 2:** Run; FAIL. **Step 3:** Implement. **Step 4:** Gate, plus PNG exports of both examples in both languages to the SDD folder. Look at them. **Step 5:** Commit `feat: example maps, templates and recent files`.

### Task 11: App identity and the Windows installer

Spec §12:
- "Installable builds for Windows (installer), macOS (.dmg) and Linux (AppImage and/or .deb) … Files with `.rcraft` open in the app."

**Files:**
- `app/reasoncraft/assets/icon.svg` (master; the existing `icon.png` is regenerated from it), `app/reasoncraft/assets/icon.ico` and `icon.icns` (generated, committed), `app/reasoncraft/build.rs` (Windows exe icon and version info through `winresource`).
- `app/reasoncraft/Cargo.toml` (`[package.metadata.packager]`), `app/reasoncraft/packaging/{linux/reasoncraft.desktop,linux/reasoncraft-mime.xml,windows/*.nsh if needed}`.
- `docs/release.md`, and `main.rs` (open a path passed by the OS association; it already accepts paths, so verify).

**Interfaces:**
- **Packager config:**
  - product name "ReasonCraft", identifier `nl.reasoncraft.app`, publisher "ReasonCraft contributors";
  - licence file, icons, and the file association `.rcraft`: name "ReasonCraft map", MIME `application/x-reasoncraft`, role Editor;
  - formats `nsis` (Windows, per-user install, no admin), `dmg`/`app` (macOS), `deb` and `appimage` (Linux).
- **Icon:** an original design, two linked rounded boxes with an arrow, drawn as SVG. It must not resemble any other product's mark. Render it to 16–1024 px PNGs, `.ico` and `.icns` with a small `cargo run -p xtask -- icons` (a new `xtask` crate in the workspace, never shipped).
- **Windows build:** `cargo packager --release -p reasoncraft --formats nsis` produces `target/release/packager/ReasonCraft_<ver>_x64-setup.exe`.
- **VM verification**, scripted in `packaging/windows/verify-in-vm.ps1`, run by the implementer, output recorded:
  1. Start the "Windows 11 Sandbox" VM and take a checkpoint "before-reasoncraft".
  2. Copy the installer in with `Copy-VMFile` (Guest Service Interface), or PowerShell Direct `Copy-Item -ToSession`.
  3. Silent install with `/S`.
  4. Assert: the exe exists, the `HKCU\Software\Classes\.rcraft` ProgID points to ReasonCraft, and a sample `.rcraft` copied in opens. Start the app with the file as argument and take a window capture of the VM (`vmconnect` screenshot through WMI as before).
  5. Silent uninstall. Assert the files and association are gone, and that a document in the user's Documents folder survives.
  6. Revert the checkpoint, delete it, and turn the VM off.
  - If the VM cannot be started or reached, report it and stop at the built installer. That is a deferred risk, not a blocker.

- [ ] **Step 1: Failing tests:**
  - `icon_assets_present_and_sized`: every PNG size exists, the `.ico` has 16/32/48/256, and `icon.png` matches the SVG render.
  - `packager_config_has_association`: parse `Cargo.toml` metadata.
  - `cli_path_opens_file` (exists or extended).
- [ ] **Step 2:** Run; FAIL. **Step 3:** Implement and build the installer. **Step 4:** Gate; run the VM verification and record the transcript and screenshot in the SDD folder. **Step 5:** Commit `feat(packaging): app icon, Windows installer with .rcraft association`.

### Task 12: Linux and macOS builds and CI

**Files:** `.github/workflows/ci.yml` and `.github/workflows/release.yml`, `packaging/linux/Dockerfile` (build image), `docs/release.md` (extend).

**Interfaces:**
- **`ci.yml`:**
  - Runs on a matrix of `windows-latest`, `macos-latest` and `ubuntu-latest`.
  - Toolchain from `rust-toolchain.toml`. Linux installs egui's system deps: `libxkbcommon-dev libgtk-3-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libssl-dev`.
  - Steps: `cargo fmt --check`, `cargo clippy --workspace --all-targets -j 4 -- -D warnings`, `cargo test --workspace -j 4`.
- **`release.yml`:** on a tag `v*`, builds the packager formats per OS and uploads them as workflow artifacts (no publishing).
- **Linux verification here:** build `packaging/linux/Dockerfile` (based on `rust:1.99-bookworm` with the deps) and run `cargo test --workspace -j 4` and `cargo packager --release --formats deb` inside it.
  - Docker Desktop may need starting: allowed, and report it.
  - Record the results.
  - Linux-only failures found are fixed in this task. Clipboard on Linux is checked only through the existing fake-clipboard tests, so note real-clipboard testing as manual.
- **macOS:** there is no macOS machine. The config and CI are the deliverable, and a macOS run is a release checklist item in `docs/release.md`.

- [ ] **Step 1:** Write the Dockerfile and workflows.
- [ ] **Step 2:** Run the Linux container test and the `.deb` build. Fix Linux-only failures with tests.
- [ ] **Step 3:** Validate the workflow YAML (`actionlint` through Docker, if available; otherwise a YAML parse test).
- [ ] **Step 4:** Gate.
- [ ] **Step 5:** Commit `ci: three-OS test and package workflows; Linux build verified in Docker`.

### Task 13: End-to-end check and release docs

**Files:** create `app/reasoncraft/tests/e2e/release.rs`, `CHANGELOG.md` and `docs/user-guide.md` (short, links to the help topics); modify `README.md`, `docs/todo.md`, `docs/rulings.md` and `THIRD-PARTY.md`.

- [ ] **Step 1: E2E `release_flow`** (actions only):
  1. Set the UI to nl.
  2. Open the Dutch reasoning example and check its labels are Dutch.
  3. Switch the document language to en and check the map labels turn English while the UI stays Dutch.
  4. Open a template, edit it and save it with FakeDialogs; check it lands in recent files.
  5. Reset settings, keeping recent files.
  6. F1 opens help in Dutch.
  7. Export SVG; it contains the English labels.
  8. Every new widget has an AccessKit name.
- [ ] **Step 2: Hand checks**, without OS input:
  - Window captures in English and Dutch, in light and dark theme, at UI scale 100% and 150%.
  - Use a scenario flag such as `--scenario release-check` that is compiled only with `debug_assertions`, loads the example and opens the panels, then quits after the capture; remove it if it was temporary.
  - Look at every capture.
- [ ] **Step 3: Docs.**
  - README: install instructions per OS, features, languages, help, and the shortcut tables for both OS columns.
  - CHANGELOG: v0.1.0 with features grouped by area.
  - `todo.md`: tick step 6, write the Review section and the remaining known limitations.
  - `rulings.md`: the Step 6 rulings.
- [ ] **Step 4:** Full gate, `--release -- --ignored`, and `cargo packager` for Windows once more.
- [ ] **Step 5:** Commit `docs: release documentation; step 6 end-to-end`.

---

## Rulings made while planning (copy into docs/rulings.md, Step 6)

- **H3:** the UI language defaults to the OS language (Dutch for any `nl*` locale, otherwise English). It is switchable in View and Settings and takes effect immediately. Cost: none.
- **I3:** map labels, source names and the words in exports follow the document's language, not the UI language. Cost: a mixed-language screen when a Dutch user opens an English map.
- **J3:** the Dutch uses informal "je". The fixed argument terms are bewering, reden, bezwaar, weerlegging, medepremisse, premisse and bron. Cost: the owner may prefer other terms; they are all in `nl.ftl`.
- **K3:** on macOS, every Alt shortcut becomes Ctrl, Replace becomes Cmd+Alt+F and Search commands becomes Cmd+Shift+A. Cost: these differ from the Windows keys.
- **L3:** installers come from `cargo-packager`: NSIS per user (no admin) on Windows, DMG on macOS, and AppImage plus .deb on Linux. They are not code-signed (spec §12). Cost: SmartScreen and Gatekeeper warnings.
- **M3:** the Windows installer is verified only inside the owner's "Windows 11 Sandbox" VM, which is reverted afterwards, never on the host. Linux is verified in Docker. macOS is configured only and runs in CI once a remote exists. Cost: the macOS build is unverified until then.
- **N3:** recent files and the personal dictionary survive "Reset to defaults". Cost: none.
- **O3:** examples and templates open as clean untitled tabs, so closing them never prompts. Cost: an edited example still prompts, as usual.
- **P3:** the new app icon is an original design: two linked rounded boxes with an arrow. Cost: the owner may want a different design later.
