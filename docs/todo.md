# ReasonCraft — Status

**Design:** [docs/superpowers/specs/2026-10-25-reasoncraft-design.md](superpowers/specs/2026-10-25-reasoncraft-design.md) (approved 2026-10-25)
**Active plan:** [docs/plans/2026-10-26-step3-text-panel.md](plans/2026-10-26-step3-text-panel.md) — step 3, text panel
**Done:** [docs/plans/2026-10-25-core-editor.md](plans/2026-10-25-core-editor.md) — step 1, core editor (merged)

## Step 1 — Core editor
- [x] Task 1: Workspace, licences and core ID/model types
- [x] Task 2: Structure rules and add operations
- [x] Task 3: Move, connect, parking area and removal
- [x] Task 4: Undo/redo history
- [x] Task 5: `.rcraft` file format
- [x] Task 6: Automatic layout
- [x] Task 7: Connector geometry
- [x] Task 8: App shell (theme, i18n, title bar, tabs, ribbon, status bar)
- [x] Task 9: Canvas (camera, measurement, style-D painting, hit testing)
- [x] Task 10: Editing interactions
- [x] Task 11: File operations in the app
- [x] Task 12: End-to-end verification and docs

## Later steps (plans to be written)
- [ ] Step 2 — Sources, evaluation, notes, comments, building panel ([plan](plans/2026-10-25-step2-sources-evaluation.md))
  - [x] Task 1: Core model — source types, ratings, comments, links, images
  - [x] Task 2: Layout carries roles and strength; image space
  - [x] Task 3: Package format — assets and unknown zip entries
  - [x] Task 4: Settings, evaluation actions, Evaluate ribbon tab, keyboard
  - [x] Task 5: Evaluation visuals on the map
  - [x] Task 6: Sources — adding, icons, changing type
  - [x] Task 7: Comments with presets; author name setting
  - [x] Task 8: Building panel with drag-and-drop
  - [x] Task 9: Sibling reorder, Connect to…, shortcut rebinding
  - [x] Task 10: Change card type and links
  - [x] Task 11: Images in boxes
  - [x] Task 12: End-to-end verification and docs
- [x] Step 3 — Text panel: Outline, Snippets, Check ([plan](plans/2026-10-26-step3-text-panel.md))
  - [x] Task 1: Carry-over fixes, revision counter, text panel tab shell
  - [x] Task 2: Core outline builder
  - [x] Task 3: Outline tab
  - [x] Task 4: Core checks
  - [x] Task 5: Check tab, settings and status bar count
  - [x] Task 6: Core snippets
  - [x] Task 7: Snippets tab
  - [x] Task 8: Drag a snippet selection onto the map
  - [x] Task 9: Document language
  - [x] Task 10: End-to-end verification and docs
- [ ] Step 4 — Output: exports, print, Essay tab
- [ ] Step 5 — Interchange & polish: Argdown, connector styles, layout directions, minimap, numbering, find/replace, spellcheck, autosave, Search commands
- [ ] Step 6 — Release readiness: Dutch, accessibility, help, examples, templates, installers

## Review

### Step 1 — Core editor (2026-10-25)

**Tests:** 117 pass, 1 ignored, across the workspace:
- core: 45 unit tests and 7 layout tests;
- app: 41 unit tests, 23 UI tests (egui_kittest) and 1 end-to-end test (`tests/e2e.rs`).

The ignored test is `layout_500_boxes_fast`. It passes with `cargo test -p reasoncraft_core --release --test layout -- --ignored`. `cargo clippy --workspace --all-targets -- -D warnings` is clean, and `cargo build --release -p reasoncraft` succeeds.

The end-to-end test builds a map through actions only, then saves, closes, reopens and compares it. The map has a claim, a reason with three premises, a second reason, an objection with a rebuttal, a whole-card attack, a note and a parking item. The test compares the documents for structural equality and the layout of every box and card within 0.5 px. A deliberately broken save (whole-card links dropped) makes it fail.

**Verified by hand** (Windows 11, release build unless noted):
- Built a map by keyboard on a new tab:
  - typed the claim;
  - Tab for a reason, then Ctrl+Enter for a co-premise;
  - Esc and ↑ to get back to the claim, then Tab for a second reason;
  - Shift+Tab for an objection and Shift+Tab again for its rebuttal.
- Added a whole-card objection on the second reason through the "+" menu. The connector meets the card frame at its bottom centre.
- Added a note (Note button) under a premise.
- Clicked empty canvas and pressed Tab to add a parking item. It appears under "Not connected".
- Saved with Ctrl+S through the native dialog. The title lost its unsaved dot.
- Closed the window without being asked anything, relaunched, and opened the file with Ctrl+O. The map came back identical, with 9 boxes.
- Switched the theme light ↔ dark from the status bar, and zoom to fit with Ctrl+1.
- Stress view (debug build): View → Load sample map (debug), then Fit. All 117 boxes are drawn (the zoom floor is now 3%, so Fit shows the whole map).
- Window resizing (see below): all four edges and corners resize, the cursor changes over the zones, and a maximized window shows no zones.

**Window resizing (decision Q):** the borderless window (`with_decorations(false)`) could **not** be resized on Windows. `WM_NCHITTEST` returned `HTCLIENT` at every edge, and edge drags did nothing. Added `app/reasoncraft/src/shell/resize.rs`, which is not used on macOS:
- invisible 5 pt edge zones and 10 pt corner zones on the foreground layer;
- each zone sets the matching resize cursor and sends `ViewportCommand::BeginResize` on a primary press;
- no zones while the window is maximized or full screen.

I checked all eight directions by hand and checked the cursor shapes against the system cursors. Linux was not checked by hand.

**Known limitations:**
- Opening `.rcraft` files from the macOS Finder needs packaging (an app bundle with a document type). Command-line paths work on every OS.
- The "replace existing file?" prompt can only appear with non-Windows file dialogs. The Windows dialog already asks before overwriting the file it returns.
- The Building and Text side panels are placeholders until steps 2 and 3.
- The debug-only "Load sample map" always opens a new tab, even when the current tab is untouched.

### Step 2 — Sources, evaluation, notes, comments, building panel (2026-10-25)

**Tests:** 258 pass, 1 ignored, across the workspace:
- core: 71 unit tests and 10 layout tests;
- app: 114 unit tests, 36 UI tests (egui_kittest), 25 integration tests (`arrange` 8, `box_group` 7, `building` 7, `images` 3) and 2 end-to-end tests (`tests/e2e.rs`).

The ignored test is still `layout_500_boxes_fast` (run it with `--release -- --ignored`). `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --check` are clean, and `cargo build --release -p reasoncraft` succeeds.

The end-to-end test builds the step 1 map through actions only, then adds:
- ratings on the claim, a premise, a card and a source (through `Rate`, as Alt+1/2/3 sends them);
- an expert-opinion source with a link;
- a typed comment (with a trimmed author name) and a preset comment;
- an image (FakeDialogs and a generated PNG);
- a co-premise moved one place left with `Reorder`;
- a parked reason connected under the claim with `StartConnect` and `Move`, plus a loose source that stays parked.

It saves, closes, reopens and checks that the documents are equal, the assets are equal, and the layout matches within 0.5 px (every box and card, plus card roles, card strengths and connector count). A second test opens a `.rcraft` that has an extra `thumbnail.png` zip entry, edits it, saves it under a new name through the app, and checks that the entry is still there (Review Focus 5). Two deliberately broken builds each made these tests fail: one dropped unknown entries on save, the other skipped `source_rating` in serde.

**Verified by hand** (Windows 11, release build, 1500×950 window, driven with Win32 input):
- Built a map by keyboard (claim; a reason with two co-premises; a second reason; an objection), then rated it by keyboard:
  - Alt+1 on the claim;
  - Alt+1 and Alt+3 on two co-premises;
  - Alt+2 on the objection and on its card;
  - Alt+2 on the first reason's card and Alt+1 on the second.

  The markers, "weakly/strongly supports" labels, connector weights and (with shading on) tints all showed.
- Dragged "Expert opinion" from the building panel onto a premise. The target outlined green, the source appeared under it with its icon, and the edit started. Typed the citation, then Alt+1 rated it solid.
- Comments:
  - Ctrl+Shift+M opened the popover, and Presets → "Needs more evidence." added an orange "1" badge;
  - dragging the "Well supported by evidence." preset from the panel onto the source added a green badge.
- Alt+Left moved the third co-premise one place left.
- Clicked empty canvas, then Tab added a parking item. Ctrl+Shift+K showed the hint in the status bar, and Enter connected the item under the claim.
- Home → Image picked a PNG in the native dialog. The image showed above the premise text, with nothing clipped.
- Evaluate → Show markers and Evaluation shading off and on again: the markers and tints went away and came back.
- Saved with Ctrl+S, killed the app, and started it with the file. Everything came back.
- Screenshots: `task-12-dark.png` and `task-12-light.png` in `.superpowers/sdd/2026-10-25-step2-sources-evaluation/`.

**Known limitations:**
- With NumLock on, Alt+numpad digits still rate: egui 0.37 reports numpad digits exactly like top-row digits. With NumLock off, Alt-codes are filtered.
- Dropping an item on a box that already has children inserts it at the pointer's x position among them. There is no "append" drop zone.
- Items in the parking area can be reordered by keyboard only (Alt+Left/Right), not by drag.
- Image insert runs synchronously: about 0.2 s for a typical photo, and about 1 s for a 9000×9000 image in release (more in debug).
- Images are re-encoded as PNG, so a photo can take more space than the JPEG it came from.
- A placeholder flashes briefly while a stored image decodes.
- A comment badge (top-right of a box) can overlap the rating marker of the next co-premise in the same card (top-left), because premises sit only 8 px apart.
- The building panel's Templates section is deferred to step 6, with the bundled templates.
- Colour and font size in the Home "Box" group are deferred to step 5.

### Step 3 — Text panel: Outline, Check, Snippets (2026-10-26)

**Tests:** 375 pass, 4 ignored, across the workspace:
- core: 87 unit tests, plus `checks` 9, `checks_text` 7, `layout` 10 and `outline` 8 integration tests;
- app: 158 unit tests, 89 UI and integration tests (egui_kittest; `arrange` 8, `box_group` 7, `building` 7, `check` 6, `document` 3, `images` 3, `outline` 6, `snippet_drag` 3, `snippets` 9, `text_panel` 1, `ui` 36) and 7 end-to-end tests (`tests/e2e/`).

The ignored tests are the release-only performance tests `layout_500_boxes_fast`, `outline_500_boxes_fast`, `checks_500_boxes_fast` and the kittest `outline_scrolls_large_map`. All four pass with `--release -- --ignored` (run per test target: the core lib's unit tests do not build in release because they use a debug-only helper). `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --check` are clean, and `cargo build --release -p reasoncraft` succeeds.

The end-to-end tests now live in `app/reasoncraft/tests/e2e/` (`main.rs` keeps the step 1–2 tests):
- `text_panel.rs` builds a map through actions only: two snippets (one tagged with a statistic source and a link), a reorder of the snippets, a selection dropped onto a premise with `Insert { Text }` (one history step, trimmed text, the source with its link under the new premise; undo and redo take both together), the language set to Dutch (the "of" compound check fires only then) and three outline switches. It saves, closes, reopens and checks that the document (snippets included), the outline's plain text and the issue list are equal, and that the tab's caches serve the reopened document. A build that drops `snippets` from the file makes it fail.
- `meta_sessions.rs` covers the title/author session (Task 9 gap): a typed, uncommitted title is committed as one trimmed history step by switching tabs, by closing its tab (the unsaved prompt appears; Cancel keeps it) and by closing the window (the prompt appears). A session only ever runs in the active tab, because switching commits it; a session forced into a background tab is still committed when that tab is closed from another tab. Removing the commit in `request_close_tab` makes that test fail.

**Checked by hand** (Windows 11, release build, 1280×800 window). The owner uses this machine, so no keyboard or mouse input was sent to the desktop (ruling O2). The interactive parts are covered by kittests instead, and the app was looked at through window captures:
- *Map built bottom-up from snippets:* a throwaway test built a Dutch map through actions. Four snippets (two with sources) were dropped into the parking area or onto the claim, connected with `Move`, one card turned into an objection with **Type**, a note added, and the claim and a card rated. The resulting `.rcraft` was opened from the command line. The kittests `drag_chip_appears_for_selection` and `alt_drag_on_handle_carries_whole_snippet` drag with the pointer.
- *Outline numbering and copy:* the dark capture shows `1`, `2`, `3` with because/but labels, sources unnumbered under their premise, `P1`/`P2` for the parked reason and note, role colours, the rating marker on the claim and the selected claim highlighted. Copying is covered by `copy_all_puts_plain_text_on_clipboard`.
- *Resolving check issues by clicking:* the light capture shows the Check tab on the same map: missing key words with the terms listed, "en" as two statements in one, a question, unsupported premises, and "18 issues" in the status bar. The new kittest `resolve_issue_by_clicking_then_fixing` clicks a "Nothing supports it" row, which selects the premise, then adds a source and sees the tab go all clear and the chip go to 0.
- *Switching language:* the kittest `switching_language_changes_checks` clicks Dutch on File → Document and sees the compound check fire, and Undo brings English back. The captures show a Dutch document.
- Screenshots: `task-10-dark.png` (Outline), `task-10-light.png` (Check) and `task-10-snippets.png` (Snippets, dark) in `.superpowers/sdd/2026-10-26-step3-text-panel/`. The app's settings file was backed up before the captures and restored afterwards.

**Known limitations:**
- Key terms: plural and stem changes (Dutch "baan"/"banen") are not matched, and words shorter than four characters ("job"/"jobs") are ignored.
- "Two statements in one" also fires on ordinary phrasing ("duur in aanschaf en onderhoud") and on quoted titles.
- The Check tab's rows are not virtualised; a large, partly rated map lists many "Not rated yet" rows.
- The outline does not scroll to follow a selection made on the canvas.
- A snippet card being dragged to a new place in the list does not follow the pointer (an insertion line shows where it goes).
- The first Ctrl+V after launch is occasionally ignored (seen once, not confirmed).
- Pasting 120,000 characters as a snippet takes one frame of about 340 ms in a debug build. The text is cut to 50,000 characters, with a notice.
- The UI is English only, so the outline's linking words stay English in a Dutch document (Dutch UI is step 6).

## Carry-over from step 1 reviews (put into the step 2+ plans)
- [x] Step 2, first item: drag siblings to reorder them (spec §5), with a keyboard equivalent; add a "Connect to…" command for parking items
- [x] Step 2: rebind the whole-card objection shortcut — Ctrl+Shift+Tab is the app switcher on macOS (Cmd) and usually "previous tab"
- [x] Step 2: store roles/strength in `Layout` so evaluation visuals add no per-frame `owner_of` scans; design an image asset store kept out of undo snapshots; save must keep unknown zip entries
- [ ] Step 5: compact layout (contour packing) so 100+ box maps stay readable at Fit; lay out on main/cross axes for layout directions
- [ ] Step 5: `validate()` should also check map key == stored id and kind-level attach rules (needed for Argdown import)
- [ ] Step 6: make the i18n bundle swappable for Dutch; Backspace deletes on macOS; verify Cmd+Q goes through the unsaved-changes prompt; close-button glyph contrast; long tab titles truncate

## Carry-over from step 2 reviews
- [x] Step 3, first item: mailto attach check — drop the '#' truncation and refuse control chars / '%' in parameter names
- [x] Step 3: keep theme/panel/marker toggles on the Connect-mode keep-list (optional polish)
- [ ] Step 4, first task: backend-neutral painter (shape list / surface trait, images by asset key, PaintOptions export flag without selection/badges/link glyphs/grid) before any exporter
- [ ] Step 4: inserting large photos off the UI thread (worker + busy state)
- [ ] Step 5: link glyph vs rightmost arrowheads on busy boxes; badge/marker overlap between adjacent co-premises
- [ ] Step 6: store empty comment author and translate on display; stop parsing '●' out of ftl strings (dot count in code); Unicode Cf characters in links; duplicate accessible names (Nil, panel vs ribbon)
