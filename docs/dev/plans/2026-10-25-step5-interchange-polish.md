# Step 5 — Interchange and polish: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** ReasonCraft gains Argdown import/export, Elbow and Tapered connectors, bottom-up and left-to-right layouts with compact packing, an overview minimap, map numbering, find/replace, spellcheck (English and Dutch), autosave with crash recovery and a Search commands box. The step-5 carry-overs from steps 1–4 are cleared along the way.

**Architecture:**
- Pure logic goes in `reasoncraft_core`: Argdown, find/replace, axis-neutral layout, connector geometry and check options.
- Spellchecking lives in a new crate, `reasoncraft_spell`. It holds the MPL-2.0 `spellbook` dependency and the bundled dictionaries.
- The app adds:
  - a command registry that the ribbon, Search commands and shortcuts all share;
  - a find bar;
  - a spelling pass and underlining layouters;
  - a minimap overlay;
  - a recovery store.
- `actions::apply` stays the only mutator, and the scene stays the single drawing path for both the screen and exports.

**Tech stack:**
- Rust 1.99.1 (pinned) and egui/eframe 0.37, as before.
- New: `spellbook` 0.4 (MPL-2.0, unmodified, in `reasoncraft_spell` only) and `directories` 6.
- Dictionaries: `dictionary-en` 4.0.0 (MIT AND BSD) and `dictionary-nl` 2.0.0 (BSD-3-Clause OR CC-BY-3.0; BSD chosen), both from github.com/wooorm/dictionaries.
- `File::try_lock` (std, stable since 1.89) for recovery session locks.

**Spec:** `docs/dev/specs/2026-10-25-reasoncraft-design.md`. The relevant sections are §4, §5 (connectors, layout direction), §6 (View/Home/File ribbons, Search commands, minimap), §8 (find/replace, spellcheck, autosave), §9 (Argdown, exports) and §10 (settings).

## Global Constraints

- **Platform.** Pure native egui, no webview. Windows, macOS and Linux. Fully offline: dictionaries are bundled and nothing is downloaded at run time.
- **Strings and colours.**
  - All UI text goes through `i18n::t` / `t_args`, with keys in `app/reasoncraft/i18n/en.ftl`. Dutch UI comes in step 6, so there is no `nl.ftl` yet.
  - All colours come from `theme::Tokens`.
- **Mutation.** `actions::apply` is the only mutator. Document changes go through `DocTab::edit`: exactly one history entry, and none when the change is refused or a no-op. Replace all is one entry. Changing the layout direction is one entry.
- **File format.**
  - `SCHEMA_VERSION` stays 1.
  - New document fields are top-level `Document` fields with `#[serde(default, skip_serializing_if = …)]`: `layout_direction` and `ignored_hints`. Older builds keep them through `Document.extra`.
  - New settings fields use `#[serde(default)]`.
  - Files from steps 1–4 open unchanged.
- **Exports are faithful.**
  - The connector style, layout direction and numbering seen on screen are what exports show.
  - Screen-only overlays never reach exports: find highlights, spelling underlines, the minimap, the selection ring, badges, the grid.
- **Licences.**
  - New dependencies must be MIT, Apache-2.0, BSD or ISC, with one exception: `spellbook`, which is MPL-2.0, used unmodified from crates.io, and confined to `reasoncraft_spell`.
  - Dictionary licence files ship in `crates/reasoncraft_spell/dict/<lang>/LICENSE`.
  - `THIRD-PARTY.md` at the repo root lists spellbook (MPL-2.0, with its source URL) and both dictionaries.
- **No synthetic OS input.** Never inject OS-level keyboard or mouse input. Verify with kittests, files and window screenshots only.
- **Tests stay sandboxed.** Tests never touch the real clipboard, file viewers, the user's recovery folder or the eframe settings. Recovery tests use a temp dir passed in explicitly.
- **Quality gate.** `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check` and `cargo test --workspace` pass at every commit. Source and test files stay at roughly 400 lines or fewer; split new code into submodules.

## Review Focus

1. **Argdown written by others.** A file with constructs ReasonCraft can't model must still import:
   - constructs: contradictions, equivalence classes with several members, one statement used in several places, cycles, multi-step premise-conclusion structures, tags, frontmatter, comments, inline formatting;
   - result: a valid document (`validate()` passes) plus a report listing what was dropped or changed, never a panic;
   - huge or deeply nested input is refused with a message.
   - Tests: Task 13 `import_never_panics_on_corpus`, `import_reports_dropped_constructs`, `import_rejects_oversized_input`.
2. **Crash recovery must never lose the newer copy or resurrect the wrong one.** Two running instances must not offer each other's live autosaves as crashed. A clean save, close or discard removes that tab's recovery files. A recovered document, when saved, must still ask before overwriting the original if that file changed since.
   - Tests: Task 11 `live_session_is_not_offered`, `clean_close_removes_recovery`, `restored_doc_save_asks_when_original_changed`.
3. **Layout directions never overlap boxes.** For every direction, on random-ish maps (`sample_iot_like(300)`) and on maps with parked fragments, no two box rects intersect, and every card frame contains its premises and nothing else.
   - Test: Task 4 `no_overlaps_in_any_direction`.
4. **Replace all must be one undo step and leave the document valid.** This includes replacing with an empty string, replacing inside source citations and notes, matches across case-folded characters (for example "İ" and "ß", whose lowercase has a different length), and query text that occurs in the replacement ("a" → "aa").
   - Tests: Task 8 `replace_all_single_history_entry`, `case_fold_with_length_change_is_safe`, `replacement_containing_query_terminates`.
5. **Spellcheck must not freeze the UI.** Dictionary loading (Dutch takes hundreds of ms) runs on a worker. Underlining a 50 000-character essay paragraph must not recheck every frame.
   - Tests: Task 10 `dictionary_loads_off_ui_thread`, `underline_cache_hits_on_unchanged_text`.

---

## File map

| Area | Files |
|---|---|
| Core splits (T1) | `crates/reasoncraft_core/src/edit.rs` → `edit/{mod,add,move_remove,validate,tests}.rs`; `file.rs` → `file/{mod,read,write,tests}.rs` |
| Spec items (T2) | `app/export/request.rs`, `shell/backstage.rs`, `actions/export.rs`, `core/essay/mod.rs` (salvage), `app/export/docx.rs`, `canvas/editor.rs`, `doc_tab/session.rs`, `core/outline_md.rs` |
| Commands (T3) | `app/commands.rs` + `commands/{registry,search}.rs`, `canvas/input.rs` (shortcut table), `shell/titlebar.rs`, `shell/command_search.rs`, ribbon files |
| Layout (T4–T5) | `core/layout/{mod,tree,ports,axis}.rs`, `core/layout/contour.rs`, `core/model.rs` (`LayoutDirection`), `core/connector.rs` |
| Connectors (T6) | `core/connector.rs` → `connector/{mod,curved,elbow,tapered}.rs`, `app/scene/{mod,build,egui_paint}.rs`, `app/export/svg.rs`, `app/settings.rs` |
| View extras (T7) | `app/canvas/minimap.rs`, `app/scene/numbers.rs`, `app/canvas/camera.rs` (`fit_selection`), `shell/ribbon/view.rs` (split out of `ribbon/mod.rs`) |
| Find (T8) | `core/find.rs`, `app/shell/find_bar.rs`, `app/actions/find.rs`, `app/doc_tab/find_state.rs` |
| Spell (T9–T10) | `crates/reasoncraft_spell/{Cargo.toml,src/lib.rs,src/tokens.rs,dict/en/*,dict/nl/*}`, `app/spell.rs`, `app/spell/{service,layouter,pass}.rs`, `app/shell/spelling_panel.rs` |
| Recovery (T11) | `app/recovery.rs` + `recovery/{store,session,prompt}.rs`, `app/app.rs`, `app/file_ops.rs` |
| Argdown (T12–T13) | `core/argdown/{mod,write,lex,parse,import,report}.rs`, `docs/argdown.md`, `app/actions/argdown.rs` |
| Checks (T14) | `core/checks.rs`, `core/checks/tokens.rs`, `core/model.rs` (`IgnoredHint`), `shell/text_panel/check.rs` |
| Export/panels (T15) | `app/export/svg.rs` (bold), `app/export/assets.rs` (non-PNG), `shell/text_panel/{essay,check}.rs` (virtualised) |
| E2E/docs (T16) | `app/reasoncraft/tests/e2e/interchange.rs`, `README.md`, `docs/dev/todo-history.md`, `docs/rulings.md`, `THIRD-PARTY.md` |

---

### Task 1: Split oversized core files; stronger `validate()`

The carry-overs: `edit.rs` is 916 lines and `file.rs` is 603. `validate()` must also check that each map key equals the stored id and the kind-level attach rules. Argdown import (Task 13) relies on this.

**Files:**
- Create: `crates/reasoncraft_core/src/edit/{mod.rs,add.rs,move_remove.rs,validate.rs,tests.rs}`, `crates/reasoncraft_core/src/file/{mod.rs,read.rs,write.rs,tests.rs}`
- Delete: `crates/reasoncraft_core/src/edit.rs`, `crates/reasoncraft_core/src/file.rs`
- Split: `app/reasoncraft/tests/ui.rs` (1228 lines) into feature files of ≤ 400 lines: `ui_shell.rs`, `ui_editing.rs`, `ui_navigation.rs`, `ui_ribbon.rs`. `tests/export.rs` (409 lines) gets `export_formats.rs` split out.

**Interfaces:**
- Produces: `Document::validate(&self) -> Result<(), String>`, same signature. It now also rejects:
  - `boxes[k].id != k` or `cards[k].id != k`;
  - an item whose owner may not hold it according to `rules::can_attach` semantics. For example: a Source under a Category, a card under a Category, a Category under a Premise, a Note with children, a Source with children other than notes and comments.
- All public paths stay the same (`reasoncraft_core::Document`, `EditError`, `NewLoose`, `RemoveMode`, `save`, `open`, `save_package`, `open_package`, `Package`). Only files move.

- [ ] **Step 1:** Move the code without changing it. Run `cargo test --workspace`; expect the same count as before (507 passed).
- [ ] **Step 2:** Write failing tests in `edit/tests.rs`:
  - `validate_rejects_key_id_mismatch`: build a doc, then `let n = doc.boxes.shift_remove(&id).unwrap(); doc.boxes.insert(NodeId::new(), n);`. Assert `validate().is_err()` and that the message contains `"key"`.
  - `validate_rejects_source_under_category`: grouping map, push a Source box's id into a Category's `children`. Assert `Err`.
  - `validate_rejects_card_under_category` and `validate_rejects_note_with_children`, written the same way.
  - `validate_accepts_every_fixture`: `sample_iot_like(n)` for n in [1, 10, 300] is `Ok`.
- [ ] **Step 3:** Implement the checks in `edit/validate.rs`, reusing the allowed-children table from `rules.rs`. Don't duplicate it; expose `pub(crate) fn allowed_child(parent_kind, child) -> bool` from `rules.rs` if needed.
- [ ] **Step 4:** Run `cargo test -p reasoncraft_core`; expect PASS. Then the full gate.
- [ ] **Step 5:** Commit: `refactor(core): split edit and file modules; validate checks keys and attach rules`.

### Task 2: Spec-alignment items from the step 4 review

**Files:** `app/export/request.rs`, `app/export/job.rs`, `app/actions/export.rs`, `app/shell/backstage.rs`, `crates/reasoncraft_core/src/essay/mod.rs`, `crates/reasoncraft_core/src/file/read.rs`, `app/export/docx.rs`, `app/canvas/editor.rs`, `app/doc_tab/session.rs`, `crates/reasoncraft_core/src/outline_md.rs`, `i18n/en.ftl`. Tests: `app/reasoncraft/tests/export_formats.rs`, `tests/ui_shell.rs`, core `essay/tests.rs`, `outline_md` tests.

**Interfaces:**
- Produces:
  - `ExportFormat::EssayText` (extension `txt`, name key `export.format.essay_text`), produced with `essay_plain_text(doc, lang)`.
  - `ExportFormat::Argdown` (extension `argdown`), wired up in Task 12.
  - `EssayPlan::salvage_orphans(&mut self, doc_items: impl Fn(&EssaySlot) -> bool)`: paragraphs whose `Item` slot no longer exists move to `unplaced` with an empty label. It is called once on open, from `file/read.rs`.

- [ ] **Step 1: Failing tests.**
  - `essay_text_export_writes_plain_text`: export with `FakeDialogs`; the file has the "Introduction" heading and the paragraph text, and no `#`.
  - `copy_as_image_uses_selection_when_scope_selection`: with a card selected and `ExportOptions.scope = Selection`, the scene passed to the (fake) clipboard has bounds equal to the selected item's subtree bounds, which is smaller than the whole map.
    - Copy as image follows `settings.export.scope`, the same as the PNG/SVG/PDF exports. Selection falls back to the whole map when nothing is selected.
  - `backstage_has_export_print_import`: the File backstage shows the entries "Export…" (opens the Export ribbon tab and closes the backstage), "Print" (runs `Action::Print`) and "Import Argdown…" (disabled until Task 13, with tooltip key `backstage.coming_in_this_step`; Task 13 enables it).
  - `orphan_paragraphs_become_unplaced_on_open`: a hand-built file with `essay.paragraphs` keyed by a missing card id opens with that text in `unplaced`. A second save/open keeps it there.
  - `docx_sets_document_language`: `word/styles.xml` or `word/document.xml` contains `w:lang w:val="nl-NL"` for a Dutch doc and `en-GB` for English. Use the docx-rs default run-properties language API, or post-process the XML if docx-rs lacks it, and note which in the report.
  - `long_paste_is_cut_with_notice`: pasting 60 000 characters into the box editor keeps 50 000 characters and sets the status note key `status.text_cut`.
  - `undo_dropping_inline_edit_clears_edit_focus`: open the inline editor, then undo. Assert `tab.edit_focus` is false or `None`.
  - `markdown_escapes_setext_and_indented_code`: in `outline_md`, a box text `"Title\n==="` and a text starting with four spaces render literally: `===` is escaped as `\===`, and leading spaces become `&#32;`, or another escape you choose and document in the test.
- [ ] **Step 2:** Run them; expect FAIL.
- [ ] **Step 3:** Implement. The Export ribbon's Essay ▾ menu gains "Plain text". The new strings are `export.format.essay_text`, `backstage.export`, `backstage.print`, `backstage.import_argdown` and `status.text_cut` (text: "Text was cut to 50 000 characters.").
- [ ] **Step 4:** Run the full gate; expect PASS.
- [ ] **Step 5:** Commit: `feat: essay plain text, selection copy, backstage export/print, essay salvage on open`.

### Task 3: Command registry and Search commands

Spec §6: "a **Search commands** box that can run any command."

**Files:**
- Create: `app/commands.rs`, `app/commands/registry.rs`, `app/commands/search.rs`, `app/shell/command_search.rs`
- Modify: `canvas/input.rs` (one `SHORTCUTS` table), `shell/titlebar.rs`, `shell/ribbon/{mod,home,evaluate,export}.rs`, `shell/backstage.rs`

**Interfaces:**
- Produces:
  ```rust
  pub struct Command { pub id: &'static str, pub label: &'static str /* i18n key */, pub group: &'static str /* i18n key of ribbon tab */, pub action: Action }
  pub fn all() -> &'static [Command]               // OnceLock<Vec<Command>>
  pub fn get(id: &str) -> &'static Command          // panics on unknown id (programmer error)
  pub fn search(query: &str, limit: usize) -> Vec<&'static Command>
  // canvas/input.rs
  pub struct Shortcut { pub key: egui::Key, pub mods: egui::Modifiers }
  pub fn shortcut_for(action: &Action) -> Option<Shortcut>
  pub fn shortcut_text(s: Shortcut) -> String       // "Ctrl+Shift+R"; "⌘⇧R" on macOS
  ```
- Ids are dotted and stable, e.g. `home.add_reason`, `view.zoom_fit`, `export.png`, `file.save`.
  - Parameterised actions get one command each: the 17 source types `home.add_source.<key>`, the ratings, the themes, the connector styles, the layout directions and the export formats.
  - Later tasks add their commands here: find, replace, spelling, minimap, numbering, connector style, layout direction, import Argdown, export Argdown.
- `key_to_action` becomes a lookup in a single `SHORTCUTS: &[(Shortcut, ActionCtor)]` table. Context-dependent keys stay as explicit code: Tab, Enter, arrows, typing, and keys while editing.
- Ribbon buttons take their label and action from `commands::get(id)`; icons stay in the ribbon code.

- [ ] **Step 1: Failing tests** (`app/reasoncraft/src/commands/tests.rs` and `tests/command_search.rs`):
  - `command_ids_unique`
  - `search_ranks_prefix_first`: `search("add r", 5)[0].id == "home.add_reason"`. Matching is case- and diacritic-insensitive over the translated label and the group label: lowercase both, strip the combining marks from NFD decomposition (`unicode-normalization`, MIT/Apache, is allowed), then match every query word as a substring. Word-prefix hits rank above other substring hits.
  - `shortcut_table_round_trips`: for every `SHORTCUTS` entry, `key_to_action(key, mods, false)` returns its action, and `shortcut_for(&action)` returns that shortcut.
  - `ribbon_renders_all_tabs_with_registry`: a kittest renders each ribbon tab. It must not panic, which proves that every `get` id exists.
  - `alt_q_focuses_search_enter_runs`: Alt+Q focuses the title-bar box; typing "zoom 100" then Enter applies `Zoom100`, so the camera zoom is 1.0.
  - `disabled_command_not_run`: with nothing selected, searching "add co-premise" lists it greyed. Enter does nothing and shows the tooltip key `command.disabled`.
  - `search_esc_restores_focus`: Esc clears the query and closes the list.
- [ ] **Step 2:** Run; expect FAIL.
- [ ] **Step 3:** Implement.
  - The results list is a popup under the title-bar box: up to 12 rows, each with label, group and right-aligned shortcut text. Up/Down moves the highlight and Enter or a click runs `apply(app, cmd.action.clone())`.
  - Enabled state comes from `actions::enabled::is_enabled`.
  - The title bar shows the `Alt+Q` hint in `hint_text` ("Search commands (Alt+Q)").
  - README's shortcut table must match `SHORTCUTS`. Add the test `readme_shortcuts_match_table`, which parses the README table rows of the form `| keys | … |` from the "Keyboard" section and compares them with `shortcut_text` of every entry. Skip context rows that are marked `(context)`.
- [ ] **Step 4:** Run the gate; expect PASS.
- [ ] **Step 5:** Commit: `feat(app): command registry and Search commands (Alt+Q)`.

### Task 4: Axis-neutral compact tree layout

The carry-over: use contour packing so maps with 100+ boxes stay readable at Fit, and lay out on main/cross axes so Task 5 can add directions without touching the packing code.

**Files:** create `core/layout/axis.rs` and `core/layout/contour.rs`; modify `core/layout/{mod,tree,ports}.rs`. Tests: `crates/reasoncraft_core/tests/layout.rs`, plus a new `tests/layout_compact.rs`.

**Interfaces:**
- Produces, in `core/layout/axis.rs`:
  ```rust
  pub(crate) struct Ext { pub cross: f32, pub main: f32 }      // size of a box along the axes
  pub(crate) struct Frame { dir: LayoutDirection }             // maps (cross, main) <-> world
  impl Frame { fn ext(&self, size: Vec2) -> Ext; fn to_world(&self, cross: f32, main: f32, ext: Ext) -> Rect }
  ```
  With `LayoutDirection::TopDown` only (Task 5 adds the others), the world output is identical to the old layout except for compaction.
- In `contour.rs`: `pub(crate) struct Contour { segs: Vec<(f32 /*main_lo*/, f32 /*main_hi*/, f32 /*cross_lo*/, f32 /*cross_hi*/)> }`, with `fn separation(left: &Contour, right: &Contour) -> f32` and `fn merge(&mut self, other: &Contour, shift: f32)`.
- `layout()` keeps its signature. `LayoutParams` is unchanged.

Packing rule:
- Siblings are placed in order. Each new subtree shifts along the cross axis by `max(0, separation(acc, next)) + h_gap`.
- `separation` is computed only over intervals that overlap on the main axis, so a shallow subtree can tuck under a wide but shallow neighbour's empty space at deeper levels.
- The parent (box, or card frame with its premises) is centred over the span from its first to its last child root.
- Card frames, card labels and source stacks are part of each subtree's contour, so they never overlap.

- [ ] **Step 1: Failing tests:**
  - `compact_layout_tucks_shallow_sibling`:
    - Root has three reason cards. Card A has one premise whose subtree is three levels deep and wide only at the bottom (four reasons under one reason). Card B is a single premise with no children. Card C has a premise with four reason children (wide at level 2).
    - Assert that `main_bounds.width()` is less than the sum of the three subtrees' widths plus `2*h_gap`, so B and C pack against A's contour.
    - Assert no overlaps.
  - `no_overlaps_in_any_direction`, written now for TopDown only; Task 5 extends it:
    - Build `sample_iot_like(300)` and a doc with three parked fragments.
    - Assert that no two `layout.boxes` rects intersect (allow touching ≤ 0.5).
    - Assert that every card rect contains each of its premise rects.
    - Assert that no box outside a card intersects that card's rect.
  - `layout_500_boxes_fast` (existing, release, ignored) still meets its bound.
  - The existing layout tests keep passing. Positions may move because of compaction: update expectations that pinned exact x values only when the new value follows from the packing rule, and list each one in the report.
- [ ] **Step 2:** Run; expect FAIL on the tuck test.
- [ ] **Step 3:** Implement in `tree.rs` and `contour.rs`. Keep `tree.rs` at 400 lines or fewer.
- [ ] **Step 4:** Run `cargo test -p reasoncraft_core --test layout --test layout_compact`, then the gate.
- [ ] **Step 5:** Commit: `feat(core): compact contour-packed layout on main/cross axes`.

### Task 5: Layout directions

Spec §5: "Layout direction: top-down (default), bottom-up or left-to-right, per document."

**Files:**
- `core/model.rs`: add `LayoutDirection`, plus `Document.layout_direction`.
- `core/layout/{axis,mod,ports}.rs` and `core/connector.rs`.
- App: `actions/view.rs`, `actions/action.rs`, `shell/ribbon/view.rs` (new; move the View tab out of `ribbon/mod.rs`), `scene/build.rs` (card labels and stubs), `canvas/input.rs` (arrow-key navigation by direction).

**Interfaces:**
- Produces:
  ```rust
  #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "snake_case")]
  pub enum LayoutDirection { #[default] TopDown, BottomUp, LeftRight }
  impl LayoutDirection { pub const ALL: [Self; 3]; pub fn key(self) -> &'static str; pub fn toward_parent(self) -> Vec2 /* unit: TopDown (0,-1), BottomUp (0,1), LeftRight (-1,0) */ }
  // Document: #[serde(default, skip_serializing_if = "is_default")] pub layout_direction: LayoutDirection
  pub fn set_layout_direction(&mut self, d: LayoutDirection) -> Result<(), EditError>   // NotAllowed when unchanged (no-op = no history)
  // Connector gains: pub dir: LayoutDirection
  pub fn curved_path(from: Pos2, to: Pos2, stub: f32, dir: LayoutDirection) -> Vec<Segment>
  // App
  Action::SetLayoutDirection(LayoutDirection)   // commands view.direction.{top_down,bottom_up,left_right}
  ```
- **Mapping:**
  - TopDown: cross = x, main = y, children below their parent; the parent port is on the bottom edge and the child attaches at its top centre.
  - BottomUp: main = −y. Children sit above, the parent port is on the top edge and the child attaches at its bottom centre.
  - LeftRight: cross = y, main = x. Children sit to the right, the parent port is on the right edge and the child attaches at its left middle.
  - In LeftRight a card's co-premises stack along the cross axis (vertically), separated by `premise_gap`.
  - The card label band is always at the top of the frame in world space. In LeftRight it adds `card_label_height` to the frame's cross extent at its start.
  - The parking column sits after the main tree along the cross axis: to the right in TopDown and BottomUp, below in LeftRight.
- **Arrow keys** keep their visual meaning:
  - "Up" goes toward the parent in TopDown; in BottomUp the parent is reached with Down.
  - In LeftRight, Left goes to the parent and Up/Down move between siblings.
  - Implement this as one mapping function, `fn nav_dir(screen_dir: Dir, layout: LayoutDirection) -> Dir`.

- [ ] **Step 1: Failing tests:**
  - core `direction_round_trips_and_defaults`: an old file without the field opens as TopDown; saving a TopDown doc writes no `layout_direction` key; BottomUp survives save/open.
  - `no_overlaps_in_any_direction`: extend it to all three directions.
  - `children_follow_direction`: for each direction, every connector `from` lies on the far side of `to` along `-toward_parent()`, so the dot product of `(from - to)` with `toward_parent()` is less than 0.
  - `curved_path_ends_with_stub_along_direction`: the last segment is a `Line` whose direction is parallel to `toward_parent()`, for every direction.
  - app `set_direction_is_one_undo_step`: apply `SetLayoutDirection(LeftRight)`, then undo, then check `layout_direction == TopDown` and that the history grew by exactly one entry. Applying the same direction twice adds no entry.
  - `arrow_keys_follow_direction`: in LeftRight, `Left` from a premise selects the parent claim.
  - `export_uses_direction`: an SVG export of a LeftRight map has its width greater than its height for a chain of four cards.
- [ ] **Step 2:** Run; expect FAIL.
- [ ] **Step 3:** Implement.
  - The View ribbon gets a "Layout direction" large menu button with three items and icons drawn with `ShapePen`.
  - `scene/build.rs` (around line 251) computes the stub along the direction vector; `stub()` already handles markers in any direction.
- [ ] **Step 4:** Run the gate.
- [ ] **Step 5:** Commit: `feat: per-document layout direction (top-down, bottom-up, left-to-right)`.

### Task 6: Connector styles — Curved, Elbow, Tapered

Spec §5:
- Connectors are user-selectable in View and saved as a personal preference, which exports also use.
- **Elbow**: vertical and horizontal segments with rounded corners.
- **Tapered**: a filled connector that widens toward the parent.
- The owner asked that tapered connectors never become paper-thin.

Carry-overs:
- the link glyph overlapping the rightmost arrowheads on busy boxes;
- badge/marker overlap between adjacent co-premises;
- a mesh fill, because the painter only fills convex shapes.

**Files:** split `core/connector.rs` into `connector/{mod.rs,curved.rs,elbow.rs,tapered.rs}`; `app/scene/{mod,build,egui_paint}.rs`, `app/export/svg.rs`, `app/settings.rs`, `shell/ribbon/view.rs`, `canvas/links.rs`, `canvas/comments.rs`, `scene/boxes.rs` (marker).

**Interfaces:**
- Produces:
  ```rust
  #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "snake_case")]
  pub enum ConnectorStyle { #[default] Curved, Elbow, Tapered }
  pub fn elbow_path(from: Pos2, to: Pos2, stub: f32, radius: f32, dir: LayoutDirection) -> Vec<Segment>
  pub fn tapered_band(from: Pos2, to: Pos2, stub: f32, w_child: f32, w_parent: f32, dir: LayoutDirection) -> (Vec<Pos2>, Vec<Pos2>) // left, right edges, ≥ 16 samples each, same length
  pub const ELBOW_RADIUS: f32 = 8.0;          // world units, clamped to half the shorter adjacent segment
  pub const MIN_VISIBLE_PX: f32 = 1.5;        // screen px floor at every zoom (all styles)
  // Settings: pub connector_style: ConnectorStyle (serde default)
  // SceneOptions: pub connector_style: ConnectorStyle, pub direction: LayoutDirection
  // Scene: Shape::Strip { left: Vec<Pos2>, right: Vec<Pos2>, fill: Color32 }
  Action::SetConnectorStyle(ConnectorStyle)   // commands view.connector.{curved,elbow,tapered}; settings-only, no history
  ```
- **Elbow geometry:**
  - Go from `from` along the main axis to the midpoint (the main coordinate halfway between `from` and the stub start).
  - Turn along the cross axis to the port's cross coordinate.
  - Go along the main axis to the stub start, then draw the straight stub.
  - Corners are quarter-circle cubics (k = 0.5523·r).
  - When the cross offset is under 0.5, the path is a single straight line.
- **Tapered widths:**
  - `w_child = max(stroke_px(kind, strength, zoom) , MIN_VISIBLE_PX / zoom)`.
  - `w_parent = max(3.0 * w_child, 6.0 / zoom)`, reached at the stub start.
  - The stub itself is a normal stroke of `w_child`, so the arrowhead or bar lines up.
  - The centre line follows `curved_path`.
- **Painting:**
  - egui paints a `Strip` as a triangle-strip `epaint::Mesh` (pairs `left[i]`, `right[i]`).
  - SVG writes `M left… L reversed(right)… Z` with `fill`.
  - PDF needs no work, because it goes through SVG.
- **Overlap fixes:**
  - The link glyph moves to the box's top-left inner corner, so it no longer collides with the rightmost arrowheads at the bottom-right ports.
  - The acceptance marker and the comment badge never overlap a neighbouring premise: both are placed inside the box rect, with the marker top-right and the badge top-left below the link glyph, offset 4 px.
  - Write an overlap test for both.

- [ ] **Step 1: Failing tests:**
  - core `elbow_segments_axis_aligned_except_corners`: all `Line` segments are parallel to an axis, for every direction.
  - `elbow_straight_when_aligned`
  - `tapered_band_never_thinner_than_floor`: at zoom 0.05 and 4.0, the minimum distance between `left[i]` and `right[i]` is at least `MIN_VISIBLE_PX / zoom - 1e-3`.
  - `tapered_widens_toward_parent`: the width is non-decreasing from the child end to the parent end.
  - app `scene_uses_selected_style`: Tapered produces `Shape::Strip` connectors, and Elbow produces no `CubicTo` other than corner cubics with the 0.5523 ratio. Simplest check: there are as many `CubicTo` as corners, which is at most 2 per connector.
  - `strip_paints_as_mesh`: `paint_shapes` on a concave strip produces an `epaint::Shape::Mesh`.
  - `svg_strip_is_filled_path`
  - `connector_style_persists_in_settings`
  - `link_glyph_clear_of_arrowheads`: on a box with a link and 5 child cards, the glyph rect does not intersect any arrowhead triangle's bounding rect.
  - `marker_and_badge_inside_box`
- [ ] **Step 2:** Run; expect FAIL.
- [ ] **Step 3:** Implement. The View ribbon gets a "Connectors" menu with three items, each previewed with a small icon.
- [ ] **Step 4:** Run the gate. Also export a sample map in all three styles (TopDown and LeftRight) to SVG and PNG in the task's SDD folder, and look at the PNGs.
- [ ] **Step 5:** Commit: `feat: Elbow and Tapered connectors as a personal setting`.

### Task 7: Minimap, numbering on the map, fit selection

Spec §6 lists these under View:
- Zoom (fit map, **fit selection**, 100%);
- **Overview minimap (on/off/auto)**;
- **Numbering (1, 1.1, 1.2…)**.

**Files:** create `app/canvas/minimap.rs` and `app/scene/numbers.rs`; modify `canvas/camera.rs`, `canvas/mod.rs`, `settings.rs`, `scene/build.rs`, `scene/mod.rs` (`SceneOptions.numbers`), `shell/ribbon/view.rs`, `actions/view.rs`.

**Interfaces:**
- Produces:
  ```rust
  #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "snake_case")]
  pub enum MinimapMode { #[default] Auto, On, Off }
  // Settings: minimap: MinimapMode, show_numbers: bool (default false)
  impl Camera { pub fn fit_selection(&mut self, item: Rect, viewport: Rect) }   // like fit but may zoom up to 2.0
  Action::{ZoomFitSelection, SetMinimap(MinimapMode), ToggleNumbers}
  // scene/numbers.rs
  pub fn number_labels(doc: &Document) -> HashMap<Item, String>  // from build_outline(numbering: true): boxes get their outline number; cards get none (premises carry "1a")
  ```
- **Minimap:**
  - A 200 × 140 px overlay in the canvas's bottom-right corner, 12 px inset, with a `tokens.panel` background at 92% alpha and a 1 px border.
  - It draws `layout.boxes` as filled rects in the role colour, card frames as outlines, and the viewport as a rectangle in `tokens.accent`.
  - It is cached per `(revision, theme, direction)` as a list of rects; it is not a scene.
  - Clicking or dragging inside it centres the camera on that world point.
  - Auto shows it only when the content bounds at the current zoom exceed the viewport in either dimension.
  - It is never part of exports.
- **Numbering:**
  - When `show_numbers` is on, each numbered box gets a small label (`tokens.text_muted`, 0.8 × body size) inside its top-left padding.
  - The box's measured height includes a number row when numbering is on, through `LayoutParams` or a measure flag, so text never overlaps the number.
  - Exports show numbers when they are on, as with markers.
  - Numbers match the Outline's numbers exactly.
- **Fit selection:** Ctrl+Shift+1, and the command `view.zoom_fit_selection`. It fits the selected item, or the subtree for a card, with a 24 px margin.

- [ ] **Step 1: Failing tests:**
  - `minimap_auto_hidden_when_map_fits`
  - `minimap_shows_when_map_larger`
  - `minimap_click_centres_camera`: click the minimap's centre, then assert that the camera's view centre is within 1 world unit of the content bounds' centre.
  - `minimap_not_in_exports`
  - `numbers_match_outline`: for `sample_iot_like(40)`, every box's label equals its `OutlineLine.number`.
  - `numbers_never_overlap_text`: on a numbered box, the number rect does not intersect the first text row rect.
  - `fit_selection_zooms_to_item`
  - `toggle_numbers_persists`
- [ ] **Step 2:** Run; expect FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run the gate.
- [ ] **Step 5:** Commit: `feat: overview minimap, numbering on the map, fit selection`.

### Task 8: Find and replace

Spec §8: "Find (and replace) across box text, sources and notes." The Home ribbon has an Editing group with Find and Spelling.

**Files:** create `core/find.rs` (exported from `lib.rs`), `app/shell/find_bar.rs`, `app/actions/find.rs` and `app/doc_tab/find_state.rs`; modify `canvas/paint.rs` (hit outline), `shell/ribbon/home.rs` and `commands/registry.rs`.

**Interfaces:**
- Produces:
  ```rust
  #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)] pub struct FindOptions { pub match_case: bool, pub whole_word: bool }
  #[derive(Clone, Debug, PartialEq, Eq)] pub struct FindHit { pub node: NodeId, pub range: std::ops::Range<usize> } // byte range in node text
  pub fn find_all(doc: &Document, query: &str, opts: FindOptions) -> Vec<FindHit>
  impl Document {
      pub fn replace_range(&mut self, node: NodeId, range: Range<usize>, with: &str) -> Result<(), EditError> // NotFound if range invalid / not on char boundary
      pub fn replace_all(&mut self, query: &str, with: &str, opts: FindOptions) -> usize
  }
  // App
  Action::{OpenFind { replace: bool }, CloseFind, FindNext, FindPrev, ReplaceCurrent, ReplaceAll, SetFindQuery(String), SetReplaceText(String), SetFindOptions(FindOptions)}
  ```
- **Search order:** the main tree in outline order, then the parking area. Every box kind is searched (claim, premise, category, source citation text, note). Within a box, hits are in text order.
- **Case-insensitive matching:**
  - Compare char by char using `char::to_lowercase()` sequences, and report byte ranges in the original text. Never index a lowercased copy.
  - An empty query gives no hits.
  - Whole word: the characters before and after the hit are not alphanumeric and not `_`.
- **`replace_all`:** process the hits right-to-left within each box, so earlier ranges stay valid. It never rescans text it has just inserted.
- **Keyboard:** Ctrl+F opens Find, Ctrl+H opens Replace, F3 / Shift+F3 or Enter / Shift+Enter in the field go to the next or previous hit, and Esc closes.
- **Find bar:**
  - It sits above the canvas and holds the query, "Aa" (match case), "W" (whole word), the counter "3 of 12" (`find.count`), Prev and Next.
  - In replace mode it adds the replacement field, Replace and Replace all.
  - The current hit selects its box and reveals it.
  - Boxes with hits get a 2 px `tokens.find_hit` outline. That is a new token in both themes, AA contrast against the canvas, and screen-only.
  - Replace all shows the status note `find.replaced` ("Replaced { $count }").
  - Hits are cached in `FindState` per `(revision, query, opts)`.

- [ ] **Step 1: Failing tests** (core `find` tests and app `tests/find.rs`):
  - `finds_in_all_box_kinds_in_outline_order`
  - `match_case_and_whole_word`
  - `case_fold_with_length_change_is_safe`: the text "İstanbul STRASSE straße" with the query "straße" finds "straße", plus "STRASSE" only if the char-by-char fold matches (it does not, so assert exactly 1 hit). The query "i̇stanbul" must not panic, and every returned range lies on char boundaries.
  - `replacement_containing_query_terminates`: `replace_all("a","aa")` on "banana" gives "baanaanaa" and returns 3.
  - `replace_all_with_empty_string`
  - `replace_all_single_history_entry`: in the app, after Replace all across 3 boxes, one undo restores all three.
  - `replace_current_then_next`
  - `find_bar_keyboard`: Ctrl+F focuses the field, typing selects the first hit's box, F3 advances, and Esc closes and restores the canvas focus.
  - `find_highlight_not_in_exports`
- [ ] **Step 2:** Run; expect FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run the gate.
- [ ] **Step 5:** Commit: `feat: find and replace across boxes, sources and notes`.

### Task 9: `reasoncraft_spell` crate

**Files:** create `crates/reasoncraft_spell/Cargo.toml` (licence `MIT OR Apache-2.0` for our code; depends on `spellbook = "0.4"`), `src/lib.rs`, `src/tokens.rs`, `src/tests.rs`, `dict/en/{index.aff,index.dic,LICENSE}`, `dict/nl/{index.aff,index.dic,LICENSE}` and `THIRD-PARTY.md`. Add the crate to the workspace members.

**Interfaces:**
- Produces:
  ```rust
  pub const LANGUAGES: [&str; 2] = ["en", "nl"];
  #[derive(Debug, thiserror::Error)] pub enum SpellError { #[error("unsupported language {0}")] Unsupported(String), #[error("dictionary failed to load: {0}")] Parse(String) }
  pub struct Speller { /* spellbook::Dictionary + user words */ }
  impl Speller {
      pub fn load(lang: &str) -> Result<Speller, SpellError>      // dictionaries via include_str!
      pub fn check_word(&self, word: &str) -> bool
      pub fn misspelled(&self, text: &str) -> Vec<std::ops::Range<usize>>   // byte ranges
      pub fn suggest(&self, word: &str, max: usize) -> Vec<String>
      pub fn add_word(&mut self, word: &str)                       // personal dictionary (spellbook add)
  }
  pub fn words(text: &str) -> Vec<std::ops::Range<usize>>          // tokenizer
  ```
- Download the files from `https://raw.githubusercontent.com/wooorm/dictionaries/main/dictionaries/{en,nl}/{index.aff,index.dic,license}` and record the commit hash in `THIRD-PARTY.md`.
- **Tokenizer rules:**
  - A word is a run of alphabetic characters that may contain internal `'`, `’` or `-` between letters.
  - Whitespace-delimited chunks containing `://`, `@` or starting with `www.` are skipped, and so are tokens containing digits and single letters.
  - A hyphenated token is accepted if the whole token checks, or if every part checks.
  - Words in ALL CAPS of 5 letters or fewer are skipped, as acronyms.

- [ ] **Step 1: Failing tests:**
  - `english_basics`: "The quick brown fox jumps" gives no misspellings. "I recieve teh mail" flags exactly "recieve" and "teh", and `suggest("recieve",5)` contains "receive".
  - `dutch_basics`: "Dit is een fietsenstalling bij het station" gives none. "tijdsgebonden", "ijsje", "e-mail" and "zo’n" are accepted. "fietz" is flagged.
  - `tokenizer_skips_urls_numbers_acronyms`: "See https://x.nl, NATO and 2024 or a@b.nl" gives no tokens other than "See", "and" and "or".
  - `add_word_accepts`
  - `ranges_on_char_boundaries`: text with emoji and "é".
  - `load_speed` (release, ignored): each language loads in under 1.5 s and checks 10 000 words in under 200 ms.
- [ ] **Step 2:** Run `cargo test -p reasoncraft_spell`; expect FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run the crate tests, `cargo test -p reasoncraft_spell --release -- --ignored`, then the gate.
- [ ] **Step 5:** Commit: `feat(spell): bundled English and Dutch spellchecker crate`.

### Task 10: Spellcheck in the app

Spec §8: "Spellcheck in the document language." Spec §6: Home → Editing → Spelling.

**Files:** create `app/spell.rs` and `app/spell/{service,layouter,pass}.rs`, plus `app/shell/spelling_panel.rs`; modify `settings.rs`, `canvas/editor.rs`, `shell/text_panel/{essay_slot,snippet_card}.rs`, `canvas/comments/popover.rs`, `shell/ribbon/home.rs`, `commands/registry.rs` and `jobs.rs`.

**Interfaces:**
- Consumes: `reasoncraft_spell::{Speller, words}`, `core::find::FindHit` / `Document::replace_range` (Task 8), and `jobs`.
- Produces:
  ```rust
  // Settings: spellcheck: bool (default true), spell_words: Vec<String> (personal dictionary, default empty)
  pub struct SpellService { /* per lang: Loading | Ready(Arc<Speller>) | Failed(String) */ }
  impl SpellService { pub fn get(&mut self, lang: &str, jobs: &mut Jobs) -> Option<Arc<Speller>> /* starts load on first call */ }
  pub fn spell_layouter(...) // builds a LayoutJob with misspelled ranges underlined (Stroke 1.5, tokens.spell_error)
  Action::{OpenSpelling, SpellChange { hit: FindHit, with: String }, SpellChangeAll { word: String, with: String }, SpellIgnore, SpellIgnoreAll(String), SpellAddWord(String)}
  ```
- **Underlines:**
  - They appear only in editing fields (the inline box editor, essay paragraphs, snippet text, the comment field), never on the painted map and never in exports.
  - Misspelled ranges are cached per `(lang, hash(text))` in a small LRU of 256 entries, so a 50 000-character paragraph is checked once per change.
  - `tokens.spell_error` is a new token in both themes.
- **Spelling pass (F7):**
  - It opens a non-modal panel docked above the text panel.
  - It walks the misspellings in find order (Task 8) across box text, sources and notes, starting at the selection.
  - For each one it selects and reveals the box, then shows the word in context (±30 chars), up to 6 suggestions and the buttons Change, Change all, Ignore, Ignore all (this session), Add to dictionary and Close.
  - Change is one history entry. Change all is one entry, and runs `replace_all` with `match_case` and `whole_word` both on.
  - At the end it shows "Spelling check complete" (`spell.done`).
- While the dictionary loads, the panel shows "Loading dictionary…" and the fields show no underlines.
- `spellcheck=false` hides the underlines and disables F7.
- A dictionary that fails to load shows an error once and leaves editing unaffected.

- [ ] **Step 1: Failing tests:**
  - `dictionary_loads_off_ui_thread`: the first `get` returns `None` and starts a job; after the job is polled, it returns `Some`.
  - `underline_cache_hits_on_unchanged_text`: rendering the same 50 000-character paragraph twice calls the checker once (count through a test hook).
  - `essay_field_underlines_misspelling`: the field galley has an underline section covering "recieve".
  - `spelling_pass_change_and_undo`
  - `change_all_one_history_entry`
  - `add_to_dictionary_persists_in_settings`
  - `spellcheck_off_no_underlines`
  - `dutch_doc_uses_dutch_dictionary`
- [ ] **Step 2:** Run; expect FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run the gate.
- [ ] **Step 5:** Commit: `feat(app): spellcheck underlines and spelling pass in the document language`.

### Task 11: Autosave and crash recovery

Spec §8:
- "autosave to a recovery location every 30–60 s";
- "After a crash, offer to restore on next start."

Spec §10 lists the autosave interval as a setting.

**Files:** create `app/recovery.rs` and `app/recovery/{store,session,prompt}.rs`; modify `app/app.rs`, `main.rs`, `file_ops.rs`, `settings.rs`, `shell/backstage.rs` (settings page interval), `doc_tab.rs` (`recovered_from`, `autosaved_revision`) and the root `Cargo.toml` (`directories = "6"`).

**Interfaces:**
- Produces:
  ```rust
  pub struct RecoveryStore { dir: PathBuf, session: Uuid, _lock: std::fs::File }
  impl RecoveryStore {
      pub fn open(dir: PathBuf) -> std::io::Result<RecoveryStore>          // creates dir, writes+locks session-<uuid>.lock
      pub fn default_dir() -> Option<PathBuf>   // directories::ProjectDirs::from("", "", "ReasonCraft")?.data_local_dir()/recovery
      pub fn orphaned(&self) -> Vec<RecoveredDoc>                           // entries of sessions whose lock can be taken (crashed)
      pub fn write(&self, tab_key: Uuid, pkg: PackageSnapshot, meta: RecoveryMeta) // off-thread via jobs
      pub fn remove(&self, tab_key: Uuid)
      pub fn discard(&self, doc: &RecoveredDoc)
      pub fn shutdown(self)                                                 // removes own session files
  }
  pub struct RecoveryMeta { pub original: Option<PathBuf>, pub original_mtime: Option<SystemTime>, pub title: String, pub saved_at: DateTime<Utc> }
  pub struct RecoveredDoc { pub session: Uuid, pub tab_key: Uuid, pub meta: RecoveryMeta, pub package: PathBuf }
  // Settings: autosave_secs: u32 (default 45, clamped 30..=600; 0 = off)
  // DocTab: recovered_from: Option<RecoveryMeta>, recovery_key: Uuid (= the tab's existing per-tab key), autosaved_revision: u64
  Modal::Recover(Vec<RecoveredDoc>)  // Restore all / Discard all / Decide later
  pub fn tick(app: &mut ReasonCraftApp, now: f64)  // called each frame with ctx time; tests call it directly
  ```
- **File layout:**
  - Each session has `<dir>/<session>/`, containing `session.lock` (held with `File::try_lock` for the whole run) and one `<tab_key>.rcraft` plus `<tab_key>.json` (`RecoveryMeta`) per dirty tab.
  - Writes go to a temp file, which is then renamed, as in `save_package`.
- **Autosave rule:**
  - Every `autosave_secs`, each tab with `revision != autosaved_revision` and unsaved work is snapshotted on the UI thread. The live edit-session text is already in `tab.doc`.
  - The snapshot is written on a worker.
  - A tab that becomes clean (saved, or undone to the saved state) has its files removed.
  - Closing a tab removes its files, whether it was saved or discarded.
  - A normal exit calls `shutdown`.
- **Startup:**
  - If `orphaned()` is not empty, show `Modal::Recover` listing title, original path and time.
  - Restore opens each entry as a new tab: dirty, `path = original`, title suffix " (recovered)", `recovered_from = Some(meta)`. It then deletes the orphan's files and re-autosaves under the current session.
  - Discard all deletes the files. Decide later keeps them.
  - Orphan session folders that end up empty are removed.
- **Saving a recovered doc to its original path:** if the file's mtime differs from `original_mtime`, show the existing replace confirmation (`Modal::ConfirmReplace`) with the message key `recovery.original_changed`.
- **Tests:** `ReasonCraftApp::new_for_test()` keeps recovery off (`None`). `new_for_test_with_recovery(dir: PathBuf)` turns it on.

- [ ] **Step 1: Failing tests** (`tests/recovery.rs` and `recovery/tests.rs`):
  - `autosave_writes_after_interval`: edit, `tick(t0)`, `tick(t0+46)`, wait for the jobs; the `<tab>.rcraft` file exists and opens with the edited text.
  - `no_autosave_when_clean`
  - `clean_close_removes_recovery`
  - `save_removes_recovery`
  - `live_session_is_not_offered`: two `RecoveryStore`s on the same dir. B's `orphaned()` does not include A's files while A is alive. After A is dropped without `shutdown` (a simulated crash), C sees them.
  - `restore_opens_dirty_tab_with_original_path`
  - `restored_doc_save_asks_when_original_changed`: change the original file's mtime, then Ctrl+S shows ConfirmReplace.
  - `discard_all_removes_files`
  - `autosave_interval_clamped`
  - `autosave_never_blocks_ui`: writing is a job; `tick` returns while the job is pending.
- [ ] **Step 2:** Run; expect FAIL.
- [ ] **Step 3:** Implement. `main.rs` passes `RecoveryStore::default_dir()`. If opening the store fails, show a status note `recovery.unavailable` and run without autosave.
- [ ] **Step 4:** Run the gate.
- [ ] **Step 5:** Commit: `feat(app): autosave to a recovery folder and restore after a crash`.

### Task 12: Argdown export

Spec §9: "Export the map as Argdown text … Map statements, support/attack relations, co-premise arguments … and attacks on inferences/whole cards … document [the mapping]."

**Files:** create `core/argdown/mod.rs`, `core/argdown/write.rs`, `core/argdown/titles.rs`, `core/argdown/tests_write.rs` and `docs/argdown.md`; modify `app/export/request.rs` and `job.rs` (`ExportFormat::Argdown`), `shell/ribbon/export.rs` and `commands/registry.rs`.

**Interfaces:**
- Produces: `pub fn to_argdown(doc: &Document) -> String`. It always includes parking (a lossless interchange file; see the rulings) and uses `\n` line endings.
- **Mapping (write exactly this; `docs/argdown.md` documents it with an example):**
  1. **Frontmatter** between `===` lines: `title`, `author`, `language` and `generator: ReasonCraft`. Values are YAML-quoted strings.
  2. **Titles.**
     - Every box statement gets a title made from its first five words. Strip `[]<>{}:#@`, collapse whitespace and cut at 40 characters.
     - Duplicates get " 2", " 3" appended. An empty title becomes `Statement N`.
     - Cards become arguments titled `R<n>` (reasons) or `O<n>` (objections), numbered in outline order.
  3. **Main tree block.** The root first: `[Title]: text`, then one indented line (4 spaces) per card targeting the root: `<+ <Rn>` for a reason, `<- <On>` for an objection.
  4. **Argument blocks** follow, in outline order, one per card, separated by blank lines.
     - Header: `<Rn>` plus data `{strength: strong|weak|nil}` when the card is rated.
     - Then a blank line and the premise-conclusion structure:
       - `(k) [Title]: premise text` per premise. Optional data: `{acceptance: accept|unsure|reject}`, `{link: "url"}`, `notes: ["…"]`, `comments: [{author, text, verdict}]`.
       - Under each premise, indented 4 spaces: one line per card targeting that premise (`<+ <Rm>` or `<- <Om>`), and one line per source: `<+ [Title]: citation {source: <SourceType key>, rating: solid|shaky|nil, link: "…"}`, with the source's notes and comments as data.
       - Then the inference line `----`. Under it, indented: one line per whole-card attack (`<_ <Om>`) and per whole-card reason (`<+ <Rm>`). The second form is a ReasonCraft convention, documented as such.
       - Then the conclusion `(k+1) [Target title]` for a reason card. For an objection card it is `(k+1) Not: <target title>`, followed by an indented `-> [Target title]`. When the target is a whole card, the conclusion is the text `The inference in <Rn> holds.` or `The inference in <Rn> does not hold.`, with no relation line.
  5. **Root and other statements:**
     - Sources, notes and comments on the root are written as indented lines and data on the root statement, the same way as for premises.
     - Root acceptance goes in the root's data.
  6. **Grouping maps:** the root statement, then category items as a nested tree of `<+ [Title]: text {category: true}` lines. Their notes and comments go in data.
  7. **Parking:**
     - A heading `# <not_connected_heading(lang)>`.
     - Then each parked fragment as its own block. A parked card is an argument block whose conclusion line is omitted (an argument without a target). A parked box is a statement block with its children.
  8. **Escaping:** text that contains a newline is joined with spaces (`docs/argdown.md` notes this). The characters `[`, `]`, `<` and `>` at the start of text are escaped with a backslash. Images are not exported, and the export notice says "Images are not included in Argdown." (`export.argdown_no_images`) when the map has any.

- [ ] **Step 1: Failing tests** in `tests_write.rs`:
  - `writes_frontmatter_and_root`
  - `reason_with_two_premises_as_pcs`: the expected text is a literal multi-line string for a claim and one reason with two premises.
  - `objection_targets_premise`: the relation line sits under the premise in the target's PCS, and the objection's conclusion is `Not:` followed by `-> [..]`.
  - `whole_card_attack_under_inference`
  - `sources_and_ratings_in_data`
  - `grouping_map_uses_category_data`
  - `parking_after_heading`
  - `titles_unique_and_sanitised`
  - `export_argdown_via_ribbon`: an app test writes a `.argdown` file with `FakeDialogs`.
- [ ] **Step 2:** Run; expect FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run the gate.
- [ ] **Step 5:** Commit: `feat: Argdown export with documented mapping`.

### Task 13: Argdown import

Spec §9: "Import Argdown files, laying them out automatically … import as faithfully as possible and report anything dropped." Spec §4: extra top-level claims go to the parking area.

**Files:** create `core/argdown/lex.rs`, `core/argdown/parse.rs` (syntax tree), `core/argdown/import.rs` (syntax tree to `Document`), `core/argdown/report.rs`, `core/argdown/tests_import.rs` and `core/argdown/corpus/*.argdown` (about 10 hand-written samples of general Argdown, including the two examples in `docs/argdown.md`); create `app/actions/argdown.rs`; modify `shell/backstage.rs` (enable "Import Argdown…"), `file_ops.rs` (dialog filter), `dialogs.rs` (`pick_import`) and `commands/registry.rs`.

**Interfaces:**
- Produces:
  ```rust
  pub const MAX_ARGDOWN_BYTES: usize = 2 * 1024 * 1024;
  pub const MAX_IMPORT_BOXES: usize = 5000;
  pub const MAX_NESTING: usize = 64;
  #[derive(Debug, thiserror::Error)] pub enum ArgdownError { TooLarge, TooManyBoxes, TooDeep { line: usize }, Empty }
  #[derive(Clone, Debug, PartialEq, Eq)] pub enum ImportNoteKind { Contradiction, ExtraEquivalentWording, SharedStatementCopied, CycleBroken, UnknownReference(String), IntermediateConclusion, InferenceRuleDropped, TagsDropped, UndercutOnStatement, EmptyArgument }
  #[derive(Clone, Debug, PartialEq, Eq)] pub struct ImportNote { pub line: usize, pub kind: ImportNoteKind }
  #[derive(Clone, Debug, Default)] pub struct ImportReport { pub notes: Vec<ImportNote> }
  pub fn from_argdown(src: &str) -> Result<(Document, ImportReport), ArgdownError>
  // App
  Action::ImportArgdown   // pick file → new tab (untitled, dirty, title from frontmatter or file stem) → Modal::ImportReport if notes non-empty
  ```
- **Parsing.**
  - The syntax is the one summarised in `docs/argdown.md`: frontmatter, headings, statements `[T]: text`, references `[T]`, arguments `<T>: text`, relations `+ - _ ><` with an optional `<`/`>` direction, PCS `(n)`, inference lines `----` or `--`…`--`, comments `//`, `/* */` and `<!-- -->`, and data `{…}`.
  - Data is parsed as a small YAML-flow subset: strings, numbers, booleans, lists, and maps of those. Anything else is kept as a string.
  - Indentation: one tab equals 4 spaces. A child relation line is more indented than its parent line.
- **Building:**
  1. **Root.** Collect the top-level trees. The main root is the statement with the most relations pointing at it, directly or through argument conclusions; ties go to the first one in the file. All other top-level items become parking fragments, in file order.
  2. **Statement → statement.**
     - `+ [X]` or `<+ [X]` under statement S creates a Reason card under S with one premise holding X's text.
     - `-` creates an Objection card.
     - `{category: true}` on the child creates a Category item instead, but only in a grouping map.
     - `{source: key}` creates a Source box of that type, falling back to `SourceType::Other`.
  3. **Argument children.**
     - `<+ <A>` or `<- <A>` under S creates a card whose premises are A's PCS premises: statements before the first inference line, plus later premises that are not conclusions.
     - Intermediate conclusions are dropped and noted as `IntermediateConclusion`.
     - An argument without a PCS becomes a single premise holding its description, or its title if it has none. If it has neither, it is noted as `EmptyArgument` and skipped.
     - An argument whose conclusion references `[S]` supports S without needing an explicit relation.
     - A conclusion line followed by `-> [S]` or `- [S]` attacks S.
  4. **Inference relations.** `_` (undercut) under an argument or its inference line creates a whole-card Objection. `+` under an inference line creates a whole-card Reason. An undercut placed under a plain statement is treated as an attack and noted as `UndercutOnStatement`.
  5. **Relations under a PCS premise line** target that premise.
  6. **Classes and repetition.**
     - An equivalence class with several members uses the first member's text; each extra member is noted as `ExtraEquivalentWording`.
     - A statement that would get a second parent is copied (it keeps the same text) and noted as `SharedStatementCopied`.
     - A relation that would create a cycle is dropped and noted as `CycleBroken`.
     - `><` is noted as `Contradiction`.
     - A reference to an undefined title is created as a statement with the title as its text, and noted as `UnknownReference`.
  7. **Text clean-up.**
     - Inline formatting is stripped (`_x_`, `__x__`, `*x*`, `**x**`).
     - In `[text](url)` the text is kept, and the first URL becomes the box link (passed through `safe_link`) when none is set.
     - Mentions `@[T]` and `@<T>` become `T`.
     - Tags are dropped, noted once as `TagsDropped`.
  8. **Data.** `acceptance`, `strength`, `rating`, `link`, `notes`, `comments`, `source` and `category` are read. Other data keys are ignored without a note.
  9. **Frontmatter.** `title`, `author` and `language` are read; a language other than `en` or `nl` falls back to `en`. `layout_direction` stays TopDown.
  10. The result must pass `validate()`. Run it in tests and also in release builds; on failure, return `Empty`. This is a bug guard; the corpus test proves it never fires.
- **App.**
  - The report modal lists the notes, translated as `argdown.note.<kind>` with line numbers.
  - The imported tab gets its file name only through Save As. The import never writes to the source path.

- [ ] **Step 1: Failing tests** in `tests_import.rs`:
  - `round_trip_preserves_structure`: for `sample_iot_like(60)` (rated, with sources and notes), a doc with parking, and a grouping map, `from_argdown(&to_argdown(&d))` is structurally equal.
    - Structural equality compares texts, kinds, card kinds and targets (premise or whole card), sources with type and rating, acceptance, strength, notes, comments, parking fragments, title, author and language. Ids are ignored, and images are expected to be dropped.
    - Write `fn structurally_equal(a: &Document, b: &Document) -> Result<(), String>` in the test module.
  - `imports_docs_examples`: both examples from `docs/argdown.md`. The undercut example gives a whole-card objection on `<Sample>`'s card.
  - `import_reports_dropped_constructs`: a sample containing `><`, two members of one class, a reused statement, a cycle, tags and a multi-step PCS has exactly the expected note kinds.
  - `import_never_panics_on_corpus`: every corpus file, plus 2 000 deterministic mutations of each (byte deletions, duplicated lines, random indentation changes; xorshift seeded per file, no `rand` dependency), returns `Ok` with a document that passes `validate()`, or `Err`.
  - `import_rejects_oversized_input`: input of 2 MiB + 1 gives `TooLarge`; 65 nested relation levels give `TooDeep`.
  - `extra_top_level_claims_go_to_parking`
  - app `import_opens_new_dirty_tab_with_report`
- [ ] **Step 2:** Run; expect FAIL.
- [ ] **Step 3:** Implement. Keep each file at 400 lines or fewer: `lex`, `parse`, `import` and `report`.
- [ ] **Step 4:** Run the gate.
- [ ] **Step 5:** Commit: `feat: Argdown import with a report of dropped constructs`.

### Task 14: Check carry-overs

The carry-overs:
- a per-issue "ignore this hint" stored in the document;
- `CheckSet` tolerating unknown kinds;
- the Compound check skipping quoted text.

**Files:** `core/model.rs`, `core/checks.rs`, `core/checks/tokens.rs`, `shell/text_panel/check.rs` and `actions/action.rs`.

**Interfaces:**
- Produces:
  ```rust
  #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
  pub struct IgnoredHint { pub kind: String /* CheckKind key, kept as string for forward compat */, pub item: Item, #[serde(default, skip_serializing_if = "Vec::is_empty")] pub terms: Vec<String> }
  // Document: #[serde(default, skip_serializing_if = "Vec::is_empty")] pub ignored_hints: Vec<IgnoredHint>
  pub fn run_checks(doc: &Document, set: &CheckSet) -> Vec<Issue>   // now omits issues matching an IgnoredHint (kind + item + same terms set)
  Action::{IgnoreHint(Issue), RestoreHints}   // one history entry each
  ```
- `CheckSet { disabled: Vec<CheckKind> }` deserializes with a custom `deserialize_with` that skips unknown strings instead of failing the whole settings load.
- Hints for items that no longer exist are removed by `remove_inner` together with the item. `validate` ignores dangling hints rather than failing. `run_checks` never panics on them.
- Compound skips text inside straight quotes `"…"`, typographic quotes `“…”` and `„…”`, and single typographic quotes `‘…’`.
- In the Check tab, each issue row gets an "Ignore" button (`check.ignore`). The footer gets "Show ignored hints (n)" (`check.restore`, which runs `RestoreHints`).

- [ ] **Step 1: Failing tests:**
  - `ignored_hint_hides_issue_and_survives_save`
  - `ignore_is_one_undo_step`
  - `unknown_check_kind_in_settings_is_skipped`: `{"checks":{"disabled":["key_term","future_kind"]}}` loads with `[KeyTerm]`.
  - `compound_skips_quotes`: `Hij zei "brood en kaas" vandaag` gives no Compound issue, while `Brood en kaas zijn duur` still does.
  - `dangling_hint_is_harmless`
- [ ] **Step 2:** Run; expect FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run the gate.
- [ ] **Step 5:** Commit: `feat(checks): ignore single hints, tolerant check settings, quotes skip Compound`.

### Task 15: Export and panel carry-overs

The carry-overs:
- PDF bold drawn once, so copied text is no longer doubled;
- non-PNG assets in exports;
- virtualising the Essay and Check tabs.

**Files:** `app/export/svg.rs`, a new `app/export/assets.rs`, `shell/text_panel/{essay,check}.rs`, and a height cache in `doc_tab` (`essay_heights`, `check_heights`).

**Interfaces:**
- Bold text in SVG and PDF: draw the text once with `font-weight="bold"` when the embedded font has a bold face. Otherwise draw it once and add a `stroke` of 0.35 px in the text colour, with `paint-order="stroke"`, on that same element. Never draw it twice. Translucent text then darkens no more than a single stroke.
- `export/assets.rs`: `pub fn export_png_bytes(asset: &[u8]) -> Option<Cow<[u8]>>`.
  - It returns PNG bytes unchanged.
  - JPEG and WebP are decoded with the `image` crate under the same pixel limits as the canvas cache and re-encoded as PNG.
  - Anything else returns `None`, which gives the placeholder.
  - It runs on the export worker, never on the UI thread.
- Virtualisation: the Essay and Check tabs use `ScrollArea::show_viewport`. Row heights are cached per `(revision, panel width, row key)`, and only rows that intersect the viewport, plus one screen of margin, are laid out.

- [ ] **Step 1: Failing tests:**
  - `pdf_bold_text_extracted_once`: extract the text of the PDF page by decompressing the content stream with `miniz_oxide`, as the existing tests do. The string "Statistic" appears once per label.
  - `jpeg_asset_exports_as_png`
  - `essay_tab_lays_out_only_visible_rows`: with 400 boxes, at most 60 rows are laid out per frame (count through a test hook).
  - `check_tab_virtualised`: the same check, for the Check tab.
  - `outline_scrolls_large_map` keeps passing.
- [ ] **Step 2:** Run; expect FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run the gate, plus `cargo test --workspace --release -- --ignored`.
- [ ] **Step 5:** Commit: `fix(export,panels): single-pass bold, JPEG/WebP assets, virtualised Essay and Check tabs`.

### Task 16: End-to-end verification and docs

**Files:** create `app/reasoncraft/tests/e2e/interchange.rs` (registered in `tests/e2e/main.rs`); modify `README.md`, `docs/dev/todo-history.md`, `docs/rulings.md`, `docs/argdown.md` and `THIRD-PARTY.md`.

- [ ] **Step 1: E2E test `interchange_round_trip`.** Using actions and `FakeDialogs` only:
  1. Build a Dutch map with a claim, a two-premise reason with a source, an objection on one premise, a whole-card rebuttal and a parked note.
  2. Set LeftRight, Tapered, numbering on.
  3. Run find "fiets", then Replace all with "bike", and check that one undo restores it.
  4. Run the spelling pass on a seeded misspelling and change it.
  5. Export Argdown, import it into a new tab and check `structurally_equal` against the source tab.
  6. Export SVG in both tabs and check that the files are identical apart from ids.
  7. Autosave with the recovery dir set to a temp dir, simulate a crash by dropping the store without `shutdown`, start a new app on the same dir, choose Restore, and check that the text matches.
- [ ] **Step 2: Check by hand.** Do not send any OS input.
  - Save `task-16-sample.rcraft` and export PNG in TopDown/Curved, BottomUp/Elbow and LeftRight/Tapered into the SDD folder, then look at them.
  - Capture the window (PrintWindow, with the file passed on the command line) showing the minimap, the find bar and the numbering.
- [ ] **Step 3: Docs.**
  - README: a features section (Argdown, directions, connector styles, minimap, numbering, find/replace, spellcheck, autosave and recovery with the recovery folder location per OS, Search commands), and the shortcut table matching `SHORTCUTS` (the Task 3 test enforces this).
  - `docs/dev/todo-history.md`: tick step 5, and add a Review section with test counts and known limitations.
  - `docs/rulings.md`: add the step 5 rulings.
- [ ] **Step 4:** Run the full gate, plus `cargo build --release -p reasoncraft` and `cargo test --workspace --release -- --ignored`.
- [ ] **Step 5:** Commit: `test: step 5 end-to-end interchange; docs`.

---

## Rulings made while planning (copy into docs/rulings.md, Step 5)

- **Y2:** spellcheck uses `spellbook` (MPL-2.0, used unmodified, isolated in `reasoncraft_spell`). It is the only pure-Rust Hunspell-compatible checker that handles Dutch compounds. Cost: one file-level-copyleft dependency, which needs a notice in `THIRD-PARTY.md`.
- **Z2:** Argdown export always includes the parking area, since it is a lossless interchange format, unlike the image exports. Cost: a reader may see unfinished fragments under "Not connected".
- **A3:** Argdown mapping:
  - every card is an argument with a premise-conclusion structure;
  - objections conclude "Not: …" plus `->`;
  - whole-card reasons use `+` under the inference line (a ReasonCraft convention);
  - sources and categories are marked with `{source: …}` and `{category: true}`;
  - images are not exported.

  Cost: other Argdown tools show whole-card reasons as plain support of the argument.
- **B3:** Find covers box text only (claims, premises, categories, source citations, notes), not comments, links or snippets. Cost: comments are not searchable.
- **C3:** spelling underlines appear only in editing fields, never on the painted map or in exports. Cost: misspellings are visible only while editing a field or during the F7 pass.
- **D3:** autosave defaults to 45 s (range 30–600, 0 = off). The recovery folder is the OS local data dir `ReasonCraft/recovery`, and sessions are locked with `File::try_lock`. Cost: none.
- **E3:** in LeftRight, co-premises stack vertically inside the card frame; the label band stays on top. Cost: tall cards in LeftRight.
- **F3:** Search commands uses Alt+Q, and Replace uses Ctrl+H. On macOS, Cmd+H hides the app; macOS key review is a step-6 carry-over. Cost: Replace on macOS works only from the ribbon or Search commands until then.
- **G3:** numbering on the map uses the Outline's numbers (1, 1a/1b for co-premises, 1.w1 for whole-card attacks, P1 for parking), so the map and the Outline always agree. Cost: the scheme is not the plain "1, 1.1" of the spec example.
