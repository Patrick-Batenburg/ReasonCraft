# Rulings and assumptions made without explicit owner input

Kept so the owner can review every choice made autonomously. Newest step last.

## Step 1 — core editor (summary)
- Branch in place, no worktree.
- Rust 1.99.1 pinned per repo (egui needs ≥1.95); system default toolchain untouched.
- chrono for timestamps; removing a card's last premise removes the card (its whole-card attackers go to parking).
- Unsaved = undo position differs from saved position; text never clipped (measure = paint).
- Add buttons target the selection; refused actions greyed out.
- Keyboard: Up from premise selects its card; Ctrl+Shift+R whole-card reason; Ctrl+Shift+P parking jump; Ctrl+S/O/N work while editing.
- Save As appends .rcraft and asks before replacing; borderless window resizable by edges.
- Zoom floor 3%; undo depth 500.
- Deferred then: sibling reorder & Connect to… (done in step 2); validate() key/kind checks (step 5).

## Step 2 — sources, evaluation, comments, building panel
- Ruling: branch in place, no worktree (same as step 1; nothing else checked out) — cost: none.
- Ruling A2: carry into Task 6 — make SourceType preserve unknown strings (e.g. custom (de)serialize to an Other(String)-style variant) so newer files don't lose data — spec §9 "unknown fields from newer versions preserved" — cost if wrong: small serde work.
- Ruling B2: markers are 16 world px overlays at the box's top-left, hidden below zoom 0.35; paint gets PaintOptions{show_markers, eval_shading}; Tokens.nil added; rating code in canvas/rating.rs — plan left sizes/split open — cost if wrong: visual tweak.
- Ruling C2: T7 also splits oversized files (actions/comments.rs, shell/ribbon/home.rs, icons split) and refuses blank comments in the UI; badge at top-right corner, hidden <0.35 zoom, hit-tested before boxes — carries T1/T6 minors — cost if wrong: refactor churn.
- Ruling D2: Insertable is Clone (SourceType not Copy); Insert delegates to existing add helpers; egui dnd payload API across panel→canvas; T8 also fixes badge click_and_drag + popover primary-only outside close (T7 minors) — cost if wrong: small.
- Ruling E2: StartConnect only for top-level parking items; drop index in document order of the parent's children; T9 also: Alt+numpad ignored for rating + AltGr test (T4 minor), panel drop primary-only (T8 minor) — cost if wrong: can't connect items nested inside a parked fragment directly (drag still works).
- Ruling F2: app keeps tab.other_entries so unknown zip entries survive open→save in the app (Review Focus #5); image decode limits via image::Limits (12000 px, 512 MiB alloc); only "png" passed to Assets::insert — cost if wrong: none.
- Ruling G2: I3 (backend-neutral painter + export options) becomes step 4's first task — not needed before exports — cost if wrong: refactor later is the same size.
- Ruling H2: fix wave also takes M2 (Stored for images), M3 (hash check on open), M6 (connect mode ends on app-level actions), placeholder flash, mailto attach hardening, Alt+Shift+0 clear-map key (M5), docs (notes-no-images decision, SCHEMA_VERSION bump rule for M4) — cheap, closes Review-Focus gaps — cost: none.
- Ruling I2b: M7 (store empty author, translate on display) and split_dots/● parsing deferred to step 6 (i18n) — cost: existing comments keep English "Anonymous" text.
- Ruling J2: park — mailto '#' fragment / attach%00 bypass of the attach check; carry to step 3 first task (drop # truncation, refuse control chars/% in param names) — cost: low (needs a client honouring attach plus user sending mail).
- Ruling K2: park — Linux/macOS launcher failures after spawn are silent with that_detached — acceptable vs UI freeze.

## Step 3 — text panel: Outline, Check, Snippets
- Ruling: branch in place, no worktree — cost: none.
- Ruling L2: revision not unique across tabs is harmless, because the outline and issue caches live inside each DocTab and are never keyed by tab index — cost if wrong: a stale view, which would surface in the outline and check tests.
- Ruling M2: plan rule "EmptyCategory = any category without Category children" would flag every leaf of a normal grouping tree (the leaves are the content). Revised: flag a category with no children only when it is a direct child of the root (a top-level heading with nothing under it — matches the course's "≥2 levels, no empty categories"); deeper leaves are content — cost if wrong: misses deeper empty headings.
- Ruling N2: accept enabling eframe's "clipboard" feature (arboard, MIT/Apache) — without it system copy/paste never worked (Ctrl+V in boxes, Copy all) — cost: a small dependency.
- Ruling O2: no OS-level synthetic keyboard or mouse input during development checks (an implementer's synthetic input once reached another app's window); verification uses kittests, scenario files and window captures only — cost: less hands-on manual checking.
- Ruling P2: check noise fix — one aggregated "not rated yet" hint; parked items excluded from rating/support checks; parked notes not flagged; Compound worded as a question; key-term rows "Missing: …"; key-term kept at every level (it is the course's co-premise signal) — cost if wrong: fewer per-item rating reminders.
- Ruling Q2: step 4 starts with (1) one EditSession enum + splitting actions.rs/doc_tab.rs/model.rs, (2) backend-neutral painter, before the Essay tab and exports; exported linking words follow the document's language — cost: none.
- Ruling R2: parked — "1 items" plural wording, weak Dutch sample test, status chip counts the aggregate as one — fixed in step 4 task 1 — cost: cosmetic.

## Step 4 — output: exports, print, essay
- Ruling: branch in place, no worktree — cost: none.
- Ruling S2: resvg/usvg pinned to 0.45 (not plan's 0.49) to share one usvg with svg2pdf 0.13 — cost: older renderer, upgrade when svg2pdf does.
- Ruling T2: OnePage fit never enlarges beyond 1 world px = 0.25 mm (same as Tile scale 1.0) so tiny maps print at normal text size; carry into T7 with a test; also clamp/skip the r/c label when margin < label height — cost: small maps print small and centred.
- Ruling U2: essay card slots are not user-writable (set_essay_text refuses Item::Card); salvage still handles card keys for old/hand-edited files — avoids text no export shows — cost: no per-section paragraph distinct from its premise slot.
- Ruling V2: final review fix wave takes I1 (per-tab widget ids), I2 (close waits for exports), I3 (print temp files create-new, 0600), M1 (Copy as image uses export theme), M5 (oversized images become placeholders in exports); M2–M4, M7–M11 carried to step 5, M6 and i18n label provider to step 6 — cost: those spec gaps (selection copy, essay plain text, backstage Export/Print) remain until step 5.
- Ruling W2: after an export error surfaces during window close the window stays open and the user closes again — cost: one extra click after a failure.
- Ruling X2: new close/export prompt strings are English-only until nl.ftl exists (step 6) — cost: none now.

## Step 5 — interchange and polish
Planning rulings (from the plan's final section, adopted as written by Ruling P5-6):
- Ruling Y2: spellcheck uses `spellbook` (MPL-2.0, used unmodified, isolated in `reasoncraft_spell`) — the only pure-Rust Hunspell-compatible checker that handles Dutch compounds — cost: one file-level-copyleft dependency, with a notice in `THIRD-PARTY.md`.
- Ruling Z2: Argdown export always includes the parking area (a lossless interchange format, unlike the image exports) — cost: a reader may see unfinished fragments under "Not connected".
- Ruling A3: Argdown mapping — every card is an argument with a premise-conclusion structure; objections conclude "Not: …" plus `->`; whole-card reasons use `+` under the inference line (a ReasonCraft convention); sources and categories are marked `{source: …}` and `{category: true}`; images are not exported — cost: other Argdown tools show whole-card reasons as plain support of the argument.
- Ruling B3: Find covers box text only (claims, premises, categories, source citations, notes), not comments, links or snippets — cost: comments are not searchable.
- Ruling C3: spelling underlines appear only in editing fields, never on the painted map or in exports — cost: misspellings are visible only while editing a field or during the F7 pass.
- Ruling D3: autosave defaults to 45 s (range 30–600, 0 = off); the recovery folder is the OS local data dir `ReasonCraft/recovery`, and sessions are locked with `File::try_lock` — cost: none.
- Ruling E3: in LeftRight, co-premises stack vertically inside the card frame; the label band stays on top — cost: tall cards in LeftRight.
- Ruling F3: Search commands uses Alt+Q and Replace uses Ctrl+H; on macOS Cmd+H hides the app, and the macOS key review is a step-6 carry-over — cost: Replace on macOS works only from the ribbon or Search commands until then.
- Ruling G3: numbering on the map uses the Outline's numbers (1, 1a/1b for co-premises, 1.w1 for whole-card attacks, P1 for parking), so the map and the Outline always agree — cost: the scheme is not the plain "1, 1.1" of the spec example.

Made while executing:
- Ruling: branch in place, no worktree — cost: none.
- Ruling P5-1: Task 2 does not add `ExportFormat::Argdown`; Task 12 adds it — avoids an unhandled variant in between — cost: none.
- Ruling P5-2: Task 2 does not add the "Import Argdown…" backstage entry; Task 13 adds it enabled — avoids a placeholder string (the Task 2 review's finding on that string was set aside for this reason) — cost: none.
- Ruling P5-3: Task 4 introduces the axis frame for top-down only; Task 5 adds `LayoutDirection` and the frame's direction — cost: none.
- Ruling P5-4: each tab gets a new random `recovery_key: Uuid` (the existing per-tab key is a u64 counter, not stable on disk) — cost: none.
- Ruling P5-5: `structurally_equal` lives in core `fixtures.rs` (feature `test-fixtures`) so the app's end-to-end test can use it — cost: none.
- Ruling T3-1: a command's group is its ribbon group label, not its tab — the search examples only work with group labels — cost: search cannot match by tab name.
- Ruling T3-2: the Type ▾ items, the paste/cut/copy placeholders and the context-dependent "clear selection" are not in the command registry — they are not standalone commands — cost: not searchable.
- Ruling T3-3: `Action::OpenCommandSearch` added so Alt+Q lives in the shortcut table and works while editing — cost: none.
- Ruling T3-4: README rows for keys that only work in context are marked "(context)"; a duplicate backstage print string was removed — cost: none.
- Ruling T4-1: the subtree contour also covers the connector band below each parent, so connectors never run behind boxes (spec §5) — cost: slightly less compaction.
- Ruling T4-2: frame methods take `self` by value (Copy; clippy) — cost: none.
- Ruling T4-3: a premise's sources (and their notes) go under that premise, not centred under the whole card; carried into Task 5, which rewrites the card internals per direction — cost: a layout change in Task 5.
- Ruling T5-1: Ruling T4-3 is implemented as a row of sources centred on its premise — a column would need connectors through boxes or a new attach mode — cost: wide rows for premises with many sources.
- Ruling T5-2: in a left-to-right map, Left from a premise goes premise → card → claim (two presses), mirroring Up in top-down — cost: none.
- Ruling T5-3: in "crowded" cards (source rows longer than the card) the layout test only checks premise order; rows may sit beside their premise — unavoidable with rows — cost: in crowded left-to-right cards sources are not strictly under their premise.
- Ruling T5-4: bottom-up connectors that target a premise end at the card frame's top edge, never crossing the rating label band (spec §5) — cost: on a single-premise bottom-up card a premise target and a whole-card target arrive on the same edge, told apart only by position.
- Ruling T6-1: boxes that show any indicator (marker, link glyph, badge, later the number) get an indicator row, so indicators never cover text — placing them inside the box alone overlapped the text — cost: boxes with indicators are taller.
- Ruling T6-2: elbow runs into the same parent are staggered so no two run on one line, ordered to avoid crossings where possible — cost: elbow runs at slightly different depths.
- Ruling T6-3: accepted deviations — no direction in `SceneOptions` (each connector carries it); tapered width is max(stroke, minimum visible width) / zoom; a 1.5 px floor for all styles (attachment lines go from 1.0 to 1.5 px at 100%) — cost: none.
- Ruling T6-4: a box with only a link or comments keeps its indicator row in exports (a blank band), because exports share the screen's layout — cost: a small blank band in such exported boxes.
- Ruling T7-1: map numbers follow the user's current Outline options, so the map and the Outline always agree — cost: none.
- Ruling T8-1: Enter in the replacement field runs Replace; the ribbon labels are "Find…" and "Replace…" — cost: none.
- Ruling T10-1: `SpellService::get` keeps its own load jobs (the plan's `Jobs` type does not exist); `CloseSpelling` and `SetSpellcheck` were added, with a Settings checkbox — cost: none.
- Ruling T11-1: no `directories` crate (it pulls in MPL-2.0 `option-ext`, and MPL is allowed only for spellbook); the recovery folder is worked out with std from `LOCALAPPDATA`, `~/Library/Application Support` or `XDG_DATA_HOME`/`~/.local/share` — cost: no Known-Folder API on Windows.
- Ruling T11-2: a file's time is recorded at open and save, not at autosave; an error opening a command-line file must not swallow the recovery prompt — cost: none.
- Ruling T11-3: one ordered recovery worker thread (writes and removes cannot overtake each other); the lock is `<session>/session.lock` — cost: none.
- Ruling T12-1: wide escaping (`\ [ ] < > { } # @ * _` and a comment-starting `/`) anywhere in text, so the import's clean-up cannot change texts; parked notes are statements with `{note: true}`; source types are written by full key; relations follow the stored child order — cost: exported Argdown has more backslashes than hand-written Argdown.
- Ruling T12-2: overriding plan text A3 where it gave invalid Argdown — whole-card conclusions use argument mentions `@<R1>`, parked cards get the placeholder conclusion `No target. {target: none}`, every argument header carries `{kind: reason|objection}`, `{ } =` are escaped in YAML strings and a leading `-` is stripped from titles (spec §9 requires valid Argdown) — cost: slightly noisier output.
- Ruling T13-1: the import's root is chosen among top-level statements and statements that relate to nothing (the literal "most relations" could pick an attacked premise or a cycle member) — cost: none for exported files.
- Ruling T13-2: an import always opens a new tab; long message bodies scroll — cost: none.
- Ruling T13-3: items that cannot attach where the file puts them go to the parking area and are reported (spec §9 "report anything dropped") — cost: none.
- Ruling T13-4: the import stays on the UI thread (every hostile case takes under 1 s in release) — cost: a brief pause on very large files.
- Ruling T15-1: there is no bold face, so bold is one text element plus a 0.35 px stroke under the fill (`paint-order="stroke"`) in SVG and PNG; in PDF the stroke is written as glyph-outline paths under plain text, because svg2pdf 0.13 writes fill-and-stroke text as two runs — keeps copied text single — cost: the PDF's bold outline is not selectable text (the plain text is).
- Ruling T15-2: the export bold stroke (0.35 px) stays lighter than the screen's faux bold (0.45 px), as the plan asked — cost: exported bold labels are a touch lighter.
- Ruling T16-1: the end-to-end test compares the SVGs of the source and the imported tab byte for byte: the SVG writer emits no ids, so "identical apart from ids" means identical — cost: none.
- Ruling T16-2: the end-to-end test sets the imported tab to left-to-right before exporting, because Argdown carries no layout direction (docs/argdown.md) — cost: none.
- Ruling S5-F1: final review — fixed before merge: recovery folder test on macOS/Linux, imported and restored tabs autosaved before the first edit (revision ≥ 1), import report grouped by full note kind, translated recovery error; everything else triaged to step 6 or "can wait" (see todo.md) — cost: dense left-to-right arrowheads and the other listed items stay until step 6.

## Step 6 — release readiness
Planning rulings (from the plan's final section, adopted as written by Ruling Q6-2):
- Ruling H3: the UI language defaults to the OS language (Dutch for any `nl*` locale, otherwise English). It is switchable in View and Settings and takes effect immediately — cost: none.
- Ruling I3: map labels, source names and the words in exports follow the document's language, not the UI language — cost: a mixed-language screen when a Dutch user opens an English map.
- Ruling J3: the Dutch uses informal "je". The fixed argument terms are bewering, reden, bezwaar, weerlegging, medepremisse, premisse and bron — cost: the owner may prefer other terms; they are all in `nl.ftl`.
- Ruling K3: on macOS, every Alt shortcut becomes Ctrl, Replace becomes Cmd+Alt+F and Search commands becomes Cmd+Shift+A (refined by S6-T6-1 and S6-T6-4) — cost: these differ from the Windows keys.
- Ruling L3: installers come from `cargo-packager`: NSIS per user (no admin) on Windows, DMG on macOS, and AppImage plus .deb on Linux. They are not code-signed (spec §12) — cost: SmartScreen and Gatekeeper warnings.
- Ruling M3: the Windows installer is verified only inside the owner's "Windows 11 Sandbox" VM, which is reverted afterwards, never on the host. Linux is verified in Docker. macOS is configured only and runs in CI once a remote exists — cost: the macOS build is unverified until then.
- Ruling N3: recent files and the personal dictionary survive "Reset to defaults" — cost: none.
- Ruling O3: examples and templates open as clean untitled tabs, so closing them never prompts — cost: an edited example still prompts, as usual.
- Ruling P3: the new app icon is an original design: two linked rounded boxes with an arrow — cost: the owner may want a different design later.

Made while implementing:
- Ruling Q6-1: Task 9 adds the Help → Example maps entry only once Task 10 exists — Task 10 adds it (Task 9 omits it) — cost: none.
- Ruling Q6-2: planning rulings H3–P3 adopted as written.
- Ruling S6-T2-1: one process-wide UI language behind an `RwLock`; language-switching tests are isolated in their own test binary (`tests/ui_language/`) with a restoring mutex; all other tests run in English — cost: tests that change the language must live in `tests/ui_language/`.
- Ruling S6-T3-1: core keeps the linking words, the essay headings and the Argdown "Not connected" heading (the importer needs both languages without the app) — cost: two homes for a few words.
- Ruling S6-T3-2: the Outline panel's content (including Copy all) is document text, so it uses the document language (matching its exports); tooltips stay in the UI language — cost: none.
- Ruling S6-T3-3: `ui_language` uses `deserialize_text_or_none` (RON-compatible) — cost: none.
- Ruling S6-T4-1: the Home ribbon was already wider than 1280 px in English; Category/Note/Comment, Delete/Delete branch and Link/Image/Remove image became small stacked buttons so Dutch fits (28 px spare) — cost: smaller targets for those commands.
- Ruling S6-T4-2: Dutch glossary beyond the fixed terms (card = blok, box = vak, Outline = Opzet, Essay = Betoog, linking words = signaalwoorden, hints = tips, Assertion = Mededeling, parking = Niet verbonden; the full list is in [glossary-nl.md](glossary-nl.md)) — cost: the owner may prefer other words; all are in `nl.ftl`.
- Ruling S6-T5-1: F6 / Shift+F6 cycle the panes (ribbon → building panel → map → text panel → status bar), the Office/Windows convention, so keyboard users can leave the map (Tab stays "add reason") — cost: one more shortcut.
- Ruling S6-T5-2: links keep invisible marks after the host (percent-encoded) and refuse them in the host and in mailto parameter names — cost: Persian or emoji paths are shown percent-encoded.
- Ruling S6-T6-1: on macOS, Alt→Ctrl applies to letters and digits only; Option+arrows stay (Ctrl+arrows are taken by Mission Control and Spaces) — cost: none.
- Ruling S6-T6-2: winit's default macOS app menu is turned off so Cmd+Q goes through the unsaved-changes prompt (its Quit exits at once) — cost: no Hide/Minimize/Quit app-menu items on macOS until a custom menu is written and tested on a Mac (unverified here).
- Ruling S6-T6-3: egui's fonts have no ⌘⌥⇧⌃ glyphs, so macOS shows text modifiers in Apple order ("Shift+Cmd+S") — cost: less Mac-like.
- Ruling S6-T6-4: macOS ratings use Cmd+Option+digit (and Cmd+Option+Shift+0 to clear the map) instead of Control+digit, which Mission Control may take for "Switch to Desktop N" — cost: a three-key chord on the Mac.
- Ruling S6-T7-1: in left-to-right maps a box grows along the cross axis so its ports keep `port_spacing` (fanning alone cannot separate nine arrowheads on a 40-unit edge); top-down and bottom-up squeeze and fan — cost: tall boxes in dense left-to-right maps. Extended by S6-T7-2.
- Ruling S6-T7-2 (extends S6-T7-1): in every direction a box grows along the cross axis so its ports keep `port_spacing`; fanning is only a clamped fallback (tan θ ≤ 1), as spec §5 forbids connectors and markers inside boxes — cost: wide boxes when many children attach to one premise.
- Ruling S6-T7-3: the whole-card port group is decided from the model: with an odd premise count it joins the middle premise's group; with an even count it spreads about the gap between the two middle premises, which grow to keep clear (sizing comes before positions) — cost: with premises of unequal size, whole-card connectors may sit off the card's centre.
- Ruling S6-T8-1: the 512 MiB memory guard is the real PNG limit (about 134 megapixels); the 256-megapixel cap stays as a backstop, and the message states the effective limit — cost: none.
- Ruling S6-T8-2: the print file's temp-name retry gives up after 16 PermissionDenied errors in a row, on all platforms — cost: none.
- Ruling S6-T9-1: the Check tab's "?" help link sits on each check heading, not on every issue row (unique accessible names); F1 maps the Outline tab to the outline-and-essay topic and Snippets to snippets; the help writes menu paths with ">" (the font lacks "→"); the backstage entry list scrolls — cost: none.
- Ruling S6-T10-1: the backstage's New opens a page (blank map, templates, examples) per spec §6; Ctrl+N and the tab "+" still create a blank map; only a blank untouched tab is reused (never an example or template) — cost: one more click for a blank map from the backstage.
- Ruling S6-T10-2: the example premises avoid "and"/"en", so the examples show no Compound hints; the rebuttal "some cities" stays rated Weak — cost: slightly less direct wording.
- Ruling S6-T11-1: the default renderer is wgpu (with the WARP software fallback on Windows); if it cannot start, glow is tried, then a native translated error dialog shows — the app must start in VMs, over RDP and without a GPU — cost: a larger binary and more dependencies.
- Ruling S6-T11-2: double-clicking a `.rcraft` file on macOS (an Apple Event "open") is not handled yet; this is a documented limitation — cost: Mac users open files with File → Open until it is fixed.
- Ruling S6-T11-3: cargo-packager's NSIS template is vendored (it removes the association on uninstall, quotes the exe path and drops an unlicensed plugin), with zlib compression (no CPL-licensed LZMA), a lower-case installer name, and VM auto sign-in turned on inside the checkpoint only — cost: we maintain a 690-line upstream template.
- Ruling S6-T11-4: the per-user install folder is `%LOCALAPPDATA%\Programs\ReasonCraft` (the old folder collided with the recovery data folder) — cost: none.
- Ruling S6-T13-1: the release end-to-end test (`release_flow`) switches the UI language, so it lives in the `tests/ui_language/` binary with its language lock (ruling S6-T2-1) instead of `tests/e2e/` — cost: none.
- Ruling S6-T13-2: when the document title does not fit centred in the title bar, it moves into the free room between the quick buttons and the command search, shortened with "…" (at 150 % UI scale it ran under the search box) — cost: the title is off-centre in narrow windows.
- Ruling S6-T13-3: the Linux release binary opens no libXrandr (checked with `strings`: it names libX11, libX11-xcb, libXcursor, libXi, libXrender, libxcb, libxkbcommon, Wayland, EGL, GL and Vulkan), so the .deb does not depend on libxrandr2 — cost: none.
- Ruling S6-T13-4: egui's bundled fonts (Ubuntu Light under the Ubuntu Font Licence 1.0, Hack, and the egui-icons subset of emoji-icon-font) are listed in `THIRD-PARTY.md`; Ubuntu Light has shipped since step 1 and is embedded in PDF exports. It is accepted as egui's default font, not a new dependency — cost: a font licence notice outside MIT/Apache must ship.
- Ruling S6-T14-1: ribbon overflow is handled by horizontal scrolling with arrow buttons in their own gutters; the Insert tab gets real commands; REASONCRAFT_PROFILE_DIR (absolute path) keeps settings and recovery in one folder; an app-wide keyboard focus outline — cost: a scrolling ribbon in small windows.
- Ruling S6-T14-2: the File page's "Export…" is renamed "Export formats"/"Exportformaten" (it switches to the Export tab, where each format has its own dialog) — cost: none.
- (User authorisation, 2026-10-27: for Task 14 only, real keyboard and mouse input was sent to the desktop for the UX pass, under foreground-window checks and an isolated profile; the "no synthetic OS input" rule applies again afterwards.)
- Ruling S6-F1: a new document's language follows the interface language until the user picks one ("Same as interface") — cost: none.
- Ruling S6-F2: cargo-packager pinned to 0.11.8 everywhere; bumping it means re-vendoring installer.nsi — cost: manual upgrades.
- Ruling S6-F3: licence texts ship in the app: About › Licences (fonts, dictionaries, spellbook, NSIS, 171 crate notices generated by `cargo run -p xtask -- notices`) — cost: a longer About page and a release-checklist step.
- Ruling S6-F4: copyright line "Patrick van Batenburg and ReasonCraft contributors"; installer publisher stays "ReasonCraft contributors" — cost: none.
- Ruling S6-F5: crate licence list split line by line and pinned to LF so Windows checkouts keep it; copyright lines in nsis.txt/spellbook.txt copied verbatim or reduced to plain attributions — cost: none.

