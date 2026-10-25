# ReasonCraft — Status

**Design:** [docs/superpowers/specs/2026-10-25-reasoncraft-design.md](superpowers/specs/2026-10-25-reasoncraft-design.md) (approved 2026-10-25)
**Active plan:** [docs/plans/2026-10-25-core-editor.md](plans/2026-10-25-core-editor.md) — step 1, core editor

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
- [ ] Step 2 — Sources, evaluation, notes, comments, building panel
- [ ] Step 3 — Text panel: Outline, Snippets, Check
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
