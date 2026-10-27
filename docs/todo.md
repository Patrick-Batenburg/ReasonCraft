# ReasonCraft — Status

**Design:** [docs/superpowers/specs/2026-10-25-reasoncraft-design.md](superpowers/specs/2026-10-25-reasoncraft-design.md) (approved 2026-10-25)
**Active plan:** [docs/plans/2026-10-25-step5-interchange-polish.md](plans/2026-10-25-step5-interchange-polish.md) — step 5, interchange and polish
**Merged:** steps 1–4
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
- [x] Step 4 — Output: exports, print, Essay tab ([plan](plans/2026-10-26-step4-output-essay.md))
  - [x] Task 1: Unified edit session and file splits
  - [x] Task 2: Scene builder and screen painter on the scene
  - [x] Task 3: SVG writer
  - [x] Task 4: PNG export and copy as image
  - [x] Task 5: PDF export, page setup and print
  - [x] Task 6: Core outline Markdown and linking words
  - [x] Task 7: Export ribbon tab, dialog and background jobs
  - [x] Task 8: Core essay plan
  - [x] Task 9: Essay tab
  - [x] Task 10: Word export
  - [x] Task 11: Image insert off the UI thread
  - [x] Task 12: End-to-end verification and docs
- [x] Step 5 — Interchange & polish ([plan](plans/2026-10-25-step5-interchange-polish.md))
  - [x] Task 1: Split core files; stronger validate()
  - [x] Task 2: Spec-alignment items from the step 4 review
  - [x] Task 3: Command registry and Search commands
  - [x] Task 4: Axis-neutral compact tree layout
  - [x] Task 5: Layout directions
  - [x] Task 6: Connector styles (Curved, Elbow, Tapered)
  - [x] Task 7: Minimap, numbering on the map, fit selection
  - [x] Task 8: Find and replace
  - [x] Task 9: reasoncraft_spell crate
  - [x] Task 10: Spellcheck in the app
  - [x] Task 11: Autosave and crash recovery
  - [x] Task 12: Argdown export
  - [x] Task 13: Argdown import
  - [x] Task 14: Check carry-overs
  - [x] Task 15: Export and panel carry-overs
  - [x] Task 16: End-to-end verification and docs
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

### Step 4 — Output: exports, print, Essay tab (2026-10-26)

**Tests:** 497 pass, 5 ignored, across the workspace (debug):
- core: 104 unit tests, plus `checks` 11, `checks_text` 7, `layout` 10, `outline` 8, `outline_md` 4 and `snippet_file` 2 integration tests;
- app: 215 unit tests, 128 UI and integration tests (egui_kittest; `arrange` 8, `background_work` 8, `box_group` 7, `building` 7, `canvas_scene` 1, `check` 7, `document` 3, `essay` 7, `export` 11, `export_replace` 4, `field_shortcuts` 3, `images` 7, `outline` 6, `snippet_drag` 3, `snippets` 9, `text_panel` 1, `ui` 36) and 8 end-to-end tests (`tests/e2e/`).

`cargo test --workspace --release` gives 496 passed (one debug-only test is left out of release builds). The 5 ignored release-only performance tests (`layout_500_boxes_fast`, `outline_500_boxes_fast`, `checks_500_boxes_fast`, `scene_500_boxes_fast` and the kittest `outline_scrolls_large_map`) pass with `cargo test --workspace --release -- --ignored`. `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --check` are clean, and `cargo build --release -p reasoncraft` succeeds.

The new end-to-end test `tests/e2e/output.rs` builds a map through actions only: a claim, a strong reason, a weak reason and an objection. It writes the essay's introduction and two sections (three history steps). It deletes the weak branch and checks that its paragraph becomes an unplaced note labelled with the branch's text, then undoes and checks that the paragraph is back in its slot. It then exports in all eight formats through `RunExport` and `FakeDialogs`, waiting for the export jobs, and checks the files:
- PNG: the signature, and an IHDR size of twice the SVG's size;
- SVG: it parses with usvg;
- PDF: `%PDF-` and exactly one `/Type /Page`;
- outline Markdown and plain text: the claim and the objection;
- outline and essay Word files: `[Content_Types].xml` and `word/document.xml` are present and well-formed, and the text is there;
- essay Markdown: every paragraph, with the strong section before the weak one.

Finally it saves, closes, reopens and compares the essay.

Writing this test showed a real bug. An image export applied without a running frame (scripted runs and tests) panicked in debug builds, because the context that measures text dropped its font-atlas delta unhandled. The delta is now cleared.

**Carried fixes (from the Task 11 review)**, each with a test in `tests/background_work.rs`:
- An image still being prepared counts as unsaved work. Closing its tab or the window shows the unsaved prompt, even when the document has no other change. **Save** keeps the prompt open with a spinner ("Preparing image…", Cancel and Don't save still available) while frames keep running, and saves and closes by itself once the image has landed. If the image fails, the save still goes ahead and the error is shown. Nothing waits on the UI thread. A finished image held back for an edit session is applied before the prompt, so the prompt counts it.
- The busy chip counts the images the tabs wait for, including one held for an edit session, so it shows exactly while a box shows "Preparing image…". The tooltip's line break is now an escaped newline instead of a literal one.
- The status note and the export notice clear after 6 s (egui time), or with the next edit, file, export or image command. Looking around (selecting, navigating, zooming, panels, tabs, theme, closing a message, "Open folder") keeps them.

**Checked by hand** (Windows 11, release build). No keyboard or mouse input was sent to the desktop (ruling O2).
- A throwaway example built a Dutch, rated sample through actions:
  - a claim;
  - a strong two-premise reason with a rated statistic source ("NO₂");
  - a weak reason with an image, prepared on the worker;
  - a nil objection with a rebuttal ("méér");
  - essay text for the introduction, three elements and the conclusion, plus a title and an author.

  It saved `task-12-sample.rcraft` and exported every format into `.superpowers/sdd/2026-10-26-step4-output-essay/`. The example was deleted afterwards.
- `task-12-map.png` (1528×842, 2×) and `task-12-svg-render.png` (the outlined SVG rendered with resvg) look the same as the map: wrapped lines, rating dots and labels, the image, the source icon and the subscript ₂. There is no selection ring, "+" button or grid.
- `task-12-map.pdf` is one A4 portrait page with the map centred and scaled to the width. Ubuntu-Light is embedded (Type0, Identity-H) and the text can be extracted. It was rendered with PyMuPDF to `task-12-pdf-render.png`.
- The outline Markdown and text use "omdat", "maar", "echter" and "Statistiek". The essay Markdown has "Inleiding", "Deel 1" to "Deel 3" strongest first, and "Conclusie".
- Both Word files pass a zip test and parse. They use the Title, Heading1 and Normal styles, and name the author in `docProps/core.xml`.
- Window capture `task-12-window.png` (PrintWindow, file passed on the command line, no input). It shows the Export ribbon tab (PNG, SVG, PDF, Copy as image, Outline ▾, Essay ▾, Page setup…, Print) and the Essay tab with the help note, the Introduction and Section 1.
  - A temporary local build put the ribbon on Export; it was reverted before the commit.
  - The settings file was backed up and was unchanged afterwards.

**Final review fixes** (507 tests pass): text-field ids are salted per tab so egui's undo never crosses documents (essay, title, author, box editor, snippets, comments); closing the window waits for running exports (waiting prompt with Cancel and Quit anyway; Quit anyway writes nothing; an export failing behind another prompt is still shown); print and export temp files are created new, never overwritten, private (0600) on Unix; Copy as image uses the export theme; exports show the placeholder for images the canvas refuses as too large.

**Known limitations:**
- No word processor is installed here. The Word files were checked as OOXML (zip, well-formed XML, styles) but never opened in Word or LibreOffice.
- The Linux clipboard (Copy as image) was not tested on a real Linux desktop.
- In the PDF, bold labels are drawn twice (faux bold), so copying their text gives it twice ("Statistic Statistic").
- The card labels and source names drawn on the map ("strongly supports", "Statistic") follow the UI language, which is English only. A Dutch map therefore exports them in English; the outline and essay exports use Dutch.
- In outlined SVG, emoji are left out (the export dialog says so).
- The Essay tab is not virtualised: every statement and field is laid out each frame, which may be slow on maps with hundreds of boxes.
- resvg/usvg stay at 0.45 until svg2pdf moves on (ruling S2).
- In exports, an image that is not stored as PNG shows as missing, and faux bold darkens translucent text.
- The "Preparing image…" band covers the first line of the box's text while the image is prepared.
- The 6 s for an export notice also run while the busy chip hides it.
- A Word file with no author set keeps docx-rs's "unknown" creator.

### Step 5 — Interchange and polish (2026-10-27)

**Tests:** 752 pass, 12 ignored, across the workspace (debug, `cargo test --workspace -j 4`):
- core: 185 unit tests, plus 56 integration tests (`checks` 11, `checks_hints` 3, `checks_text` 7, `essay_file` 1, `layout` 10, `layout_compact` 2, `layout_direction` 5, `layout_sources` 2, `outline` 8, `outline_md` 5, `snippet_file` 2);
- spell: 8 unit tests;
- app: 285 unit tests, 209 UI and integration tests in 33 files (egui_kittest; new in step 5: `close_waits`, `command_search`, `connector_style`, `find`, `import_argdown`, `layout_direction`, `minimap`, `panel_rows`, `recovery` (22), `spelling`, `tab_fields`, and `ui.rs` split into `ui_editing`, `ui_files`, `ui_navigation`, `ui_ribbon` and `ui_shell`) and 9 end-to-end tests (`tests/e2e/`).

`cargo test --workspace --release -j 4` gives 751 passed (one debug-only core test is left out of release builds). The 12 ignored release-only timing tests pass with `cargo test --workspace --release -- --ignored`: `layout_500_boxes_fast`, `outline_500_boxes_fast`, `checks_500_boxes_fast`, `scene_500_boxes_fast`, the kittest `outline_scrolls_large_map`, six linear-time Argdown import tests and the dictionary `load_speed`. `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --check` are clean, and `cargo build --release -p reasoncraft` succeeds.

The new end-to-end test `tests/e2e/interchange.rs` (`interchange_round_trip`) runs in an app that autosaves to a temp recovery folder, and uses actions and `FakeDialogs` only:
1. It builds a Dutch map with a title and an author: a claim, a two-premise reason with a statistic source, an objection on one premise, a whole-card rebuttal of that objection and a parked note.
2. It sets left to right, tapered connectors and numbers on.
3. Find "fiets" has 3 hits. Replace all with "bike" is one history step, one Undo restores every box, and Redo brings the replacement back.
4. The spelling pass, started on the premise with the seeded "gezondt", waits for the Dutch dictionary, offers "gezond" and changes it.
5. It exports Argdown, imports the file into a new tab (no import report) and checks `structurally_equal` against the source tab.
6. It exports SVG from both tabs and checks that the files are byte-identical. The imported tab is set to left to right first, because Argdown has no layout direction (ruling T16-2), and the SVG writer emits no ids (ruling T16-1). Setting the imported tab to top-down instead makes the test fail.
7. It autosaves, drops the app without a normal exit (a crash), starts a new app on the same folder, sees the recovery prompt with both maps, chooses Restore all, and finds both documents back, equal and unsaved.

**Checked by hand** (Windows 11, release build). No keyboard or mouse input was sent to the desktop (ruling O2). The artefacts are in `.superpowers/sdd/2026-10-25-step5-interchange-polish/` as `task-16-*`. A throwaway test made them and was deleted.
- `task-16-sample.rcraft` is the Dutch map above, with ratings: the claim accepted, the reason strong, the objection weak, the rebuttal strong, a second reason nil and the source solid. The PNG exports `task-16-top_down-curved.png`, `task-16-bottom_up-elbow.png` and `task-16-left_right-tapered.png` all show:
  - the numbers (1a, 1b, 1b.1, 1b.1.w1, 2);
  - the rating labels clear of the connectors;
  - the source under its premise;
  - no screen-only overlays.

  The parking area is left out, as in every image export. In bottom-up elbow, the objection's connector makes a small sideways jog (about 9 px at 2×), because it attaches to the premise, not the card centre.
- The default outlined SVG was rendered with resvg at 4× (`task-16-outlined-resvg-4x.png`, close-up `task-16-outlined-bold-zoom.png`). The bold "Statistic" label is one path with `paint-order="stroke"`. The stroke sits under the fill, the counters stay open and the edges are clean.
- Elbow connectors and rating labels were checked on a dense left-to-right map (`sample_iot_like(60)` with every card rated, `task-16-dense-left_right-elbow.png`). No connector crosses a rating label, and the labels stay in the card's top band. Where many children attach to one premise (Reason 0, with nine), their arrowheads pile up at nearly one point on the premise's edge.
- The first F7 was timed on `sample_iot_like(500)` in release:
  - The English dictionary loads in about 7 ms, and the first pass walks all 500 boxes in 1.8 ms (the sample has no English misspellings).
  - With the language set to Dutch, the English text is full of "misspellings". The first stop takes 46 ms, and a whole pass of 2,296 Ignore steps takes 118 s, about 51 ms per step. Most of that is suggestions: about 11 ms per word in English and 29 ms in Dutch.
  - Each click stays responsive, but suggestions run on the UI thread.
- Window capture `task-16-window.png` (PrintWindow, the sample passed on the command line, no input) shows:
  - the map left to right with tapered connectors and the numbers on the boxes;
  - the find bar with "fiets" and "bike", at "1 of 4" (whole word is off, so "Fietsers" matches), with the hits outlined;
  - the minimap in the bottom-right corner;
  - the Outline with the same numbers.

  A temporary local build turned these on: an environment-variable switch in `main.rs` set the numbers, the minimap, tapered connectors and the find bar, and pointed eframe's settings and the recovery folder at temp folders. It was reverted before the commit. The settings file `app.ron` had the same SHA-256 before and after, and no recovery folder was created under `%LOCALAPPDATA%`.

**Known limitations:**
- The map's card labels and source names ("strongly supports", "Statistic") and all new UI strings are English only. Dutch UI is step 6.
- Spelling suggestions run on the UI thread, at about 30–50 ms per step of a Dutch F7 pass.
- When many children attach to one premise, their arrowheads pile up on its edge. In left-to-right maps, the opposes bars of ports close together on one card edge can overlap.
- Sources form a row under their premise. In crowded left-to-right cards, a row can sit beside its premise (ruling T5-3).
- In exports, a box with only a link or comments keeps a blank indicator band (ruling T6-4).
- Argdown:
  - The official Argdown map view may show a redundant relation for whole-card conclusions. This could not be checked offline.
  - Comment dates, the links and comments of notes, where notes sit among their siblings, images and the layout direction are not carried.
  - Identical repeated arguments are reported more than once, and a few refused relations go unreported.
- Find does not search comments (ruling B3).
- Autosave:
  - Two instances started at the same instant can both offer the same crashed session (no data is lost).
  - A retry after a failed restore write offers the orphan again.
- Ignoring the "not rated yet" hint only holds while the number of unrated items stays the same.
- The Essay and Check tabs keep the last height of rows out of view, so the scroll position can shift slightly after edits above the view.
- In the PDF, the bold outline is not selectable text; the plain text under it is (ruling T15-1).
- The Search commands list rebuilds its labels (about 160 strings) every frame while it is open.

## Carry-over from step 4 reviews
- [x] Step 5 (Tasks 1, 15): draw PDF bold once (no duplicate text on copy); non-PNG assets in exports; split `crates/reasoncraft_core/src/edit.rs` (916 lines) and `file.rs` (603 lines)
- [x] Step 5 (Task 15): virtualise the Essay tab (cached heights) and the Check tab
- [ ] Step 6: map labels and source names in exports by document language (with the Dutch UI); open the Word files in Word and LibreOffice; test the clipboard on Linux; upgrade resvg/usvg together with svg2pdf
- [x] Step 5 (Tasks 1, 2, 6; each connector carries its direction and style, ruling T6-3) (final review): Copy as image of the selection (spec §9); essay plain-text export (`essay_plain_text` exists); File backstage Export…/Print entries (spec §6); essay paragraphs for vanished boxes become unplaced notes on load; Word files carry the document language; split `app/reasoncraft/tests/ui.rs` (1228 lines) and `tests/export.rs`; tell the user when a paste is cut at 50 000 characters; clear `edit_focus` when undo drops an inline edit; setext/indented-code Markdown escaping; mesh/strip fill in the scene painter (tapered connectors are not convex); connector style and direction in `SceneOptions`
- [ ] Step 6 (final review): lower the 256 MP PNG cap or check memory first (allocation failure aborts the app); swappable i18n bundle plus a document-language label provider for the scene (labels are also made on the export worker); one home for source-type names (en.ftl and core words.rs); translate the page-cap detail and the new close/export prompts into Dutch; Windows: a print file name still being deleted gives PermissionDenied instead of trying the next name

## Carry-over from step 5 (for step 6)
- [ ] Dutch `nl.ftl` with every UI string, including all strings added since step 4 (find bar, spelling panel, Search commands, layout direction and connector menus, minimap, numbers, autosave settings, recovery prompt, Argdown import report, Ignore/Show ignored hints, essay Plain text export, backstage Export/Print/Import); map labels and source names by document language
- [ ] macOS keys: Replace is Ctrl+H, but Cmd+H hides the app (ruling F3); review Alt+arrow reorder (Alt+Up/Down in left-to-right maps) and Alt+Q (Option types characters on macOS) with the rest of the macOS key review
- [ ] Accessibility of the new widgets: find bar, spelling panel, Search commands list, minimap (keyboard and screen reader), recovery prompt, Check tab Ignore buttons, Argdown import report, export dialog options
- [ ] Spelling suggestions off the UI thread (or cached), so a Dutch F7 pass steps faster than ~50 ms
- [ ] Spread the arrowheads where many children attach to one premise

## Carry-over from step 1 reviews (put into the step 2+ plans)
- [x] Step 2, first item: drag siblings to reorder them (spec §5), with a keyboard equivalent; add a "Connect to…" command for parking items
- [x] Step 2: rebind the whole-card objection shortcut — Ctrl+Shift+Tab is the app switcher on macOS (Cmd) and usually "previous tab"
- [x] Step 2: store roles/strength in `Layout` so evaluation visuals add no per-frame `owner_of` scans; design an image asset store kept out of undo snapshots; save must keep unknown zip entries
- [x] Step 5 (Tasks 4, 5): compact layout (contour packing) so 100+ box maps stay readable at Fit; lay out on main/cross axes for layout directions
- [x] Step 5 (Task 1): `validate()` should also check map key == stored id and kind-level attach rules (needed for Argdown import)
- [ ] Step 6: make the i18n bundle swappable for Dutch; Backspace deletes on macOS; verify Cmd+Q goes through the unsaved-changes prompt; close-button glyph contrast; long tab titles truncate

## Carry-over from step 2 reviews
- [x] Step 3, first item: mailto attach check — drop the '#' truncation and refuse control chars / '%' in parameter names
- [x] Step 3: keep theme/panel/marker toggles on the Connect-mode keep-list (optional polish)
- [x] Step 4, first task: backend-neutral painter (shape list / surface trait, images by asset key, PaintOptions export flag without selection/badges/link glyphs/grid) before any exporter
- [x] Step 4: inserting large photos off the UI thread (worker + busy state)
- [x] Step 5 (Task 6, ruling T6-1): link glyph vs rightmost arrowheads on busy boxes; badge/marker overlap between adjacent co-premises
- [ ] Step 6: store empty comment author and translate on display; stop parsing '●' out of ftl strings (dot count in code); Unicode Cf characters in links; duplicate accessible names (Nil, panel vs ribbon)

## Carry-over from step 3 reviews
- [x] Step 4, task 1: one `EditSession` enum (box/snippet/meta/essay) with a single snapshot; split actions.rs, doc_tab.rs, model.rs (roles.rs + card_roles == card_role test); plural selector for "not rated yet"; release-mode test build (cfg the debug-only helper test)
- [x] Step 4: exports and Essay use the document language for linking words ("because"/"omdat")
- [x] Step 5 (Task 14): per-issue "ignore this hint" stored in the document; CheckSet tolerant of unknown kinds; skip quoted text in Compound
- [ ] Step 6: Compound help by document language; panel min width; drag polish (snippet card follow, autoscroll)
