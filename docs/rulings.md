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
