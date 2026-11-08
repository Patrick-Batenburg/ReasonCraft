# ReasonCraft Step 2 — Sources, Evaluation, Notes, Comments, Building Panel — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the step-1 editor into a full argument-mapping tool:
- typed sources with their own icons;
- evaluation (accept/unsure/reject, strong/weak/nil, solid/shaky/nil) shown on the map;
- comments with feedback presets;
- links and images in boxes;
- a working building panel with drag-and-drop;
- sibling reordering and "Connect to…".

**Architecture:** Follows step 1.
- **Core (`reasoncraft_core`):** gains the new model fields and their edit ops, ratings in `Layout`, and a package format that carries image assets and preserves unknown zip entries.
- **App (`reasoncraft`):**
  - new `Action`s, all mutating only through `actions::apply` → `DocTab::edit`;
  - an Evaluate ribbon tab, painter visuals for ratings;
  - a `shell/building.rs` panel and a `canvas/comments.rs` popover;
  - an asset store per tab that lives outside undo snapshots.

**Tech Stack:** Rust 1.99.1 (pinned), egui/eframe/egui_kittest 0.37, emath 0.37, serde/serde_json, zip 9, uuid 1, indexmap 2, rfd 0.17, fluent-bundle 0.16, thiserror 2. New dependencies:
- `image` 0.25 (`default-features = false`, features `png`, `jpeg`, `webp`): decode and downscale images.
- `sha2` 0.10: content hashes for assets.
- `open` 5: opening links in the system browser.

**Spec:** `docs/dev/specs/2026-10-25-reasoncraft-design.md`. This step covers:
- §4: sources, evaluation, comments, notes, image + hyperlink, parking "Connect to…";
- §5: rating visuals;
- §6: the Evaluate ribbon tab, the building panel, the Home Box group;
- §8: keyboard for every evaluate action;
- §10: settings (comment author name).

It also covers the step-1 carry-over items in `docs/dev/todo-history.md`.

## Global Constraints

- **UI framework:** pure native egui, no webview; Windows/macOS/Linux; works offline (opening a user's link in their browser is the only outbound action, and only on explicit click).
- **Licensing:** dual MIT / Apache-2.0; new deps must be permissively licensed.
- **Text and colour:**
  - all UI text via `i18n::t` / `t_args`, with keys in `app/reasoncraft/i18n/en.ftl` (dotted keys, dashed in the .ftl);
  - all colours from `theme::Tokens`;
  - box text never clipped (measure = paint).
- **Mutation rule:** `actions::apply` is the only place app state/document changes; every document change goes through `DocTab::edit` (one history entry on success, none on failure).
- **File format:**
  - `.rcraft` stays readable by step-1 builds: new fields are optional (`#[serde(default, skip_serializing_if = …)]`);
  - `SCHEMA_VERSION` stays `1`;
  - files written by step 1 open unchanged in step 2.
- **Undo snapshots:** never contain image bytes.
- **Connector rules from step 1** still hold:
  - fixed 9 px arrowheads and a 16 px stub;
  - min stroke 1.25 screen px;
  - ports spread 16 px;
  - label inside the frame, top-right.
- **Quality gate:** `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check` and `cargo test --workspace` pass at every commit.

## Review Focus

1. **Older and unknown source types.** Files from step 1 (`source_type: "publication"`) and unknown future type strings must still open. Unknown types show as the generic Source. → Task 1 test `source_type_compat_and_unknown_fallback`.
2. **Ratings staying consistent through edits.** Ratings must survive kind changes, moves, undo, and save/open. A card switched from reason to objection keeps its strength. A box that changes kind loses ratings that no longer apply, instead of keeping hidden stale ones. → Task 1 test `ratings_follow_kind_rules`; Task 3 test `package_roundtrip_keeps_ratings_comments_links`.
3. **Broken or huge images.**
   - **Huge:** a 9000×9000 or 40 MB image is refused or downscaled with a message, never a crash or a frozen UI.
   - **Corrupt:** a corrupt image file gives an error modal.
   - **Missing asset:** a `.rcraft` whose image entry is missing still opens and shows a placeholder.

   → Task 3 test `open_with_missing_asset_keeps_image_ref`; Task 11 tests `oversized_image_is_downscaled`, `corrupt_image_shows_error`.
4. **Refused drops from the building panel.** Dropping a source on a category, a card on a note, or a comment preset on empty canvas must change nothing and record no history, and the target must highlight red. → Task 8 test `refused_panel_drop_changes_nothing`.
5. **Unknown zip entries.** A `.rcraft` written by a newer build with extra zip entries (e.g. `thumbnail.png`) must keep them after open → save. → Task 3 test `unknown_zip_entries_survive_save`.

---

## File Structure

```
crates/reasoncraft_core/src/
  model.rs           + SourceType, Acceptance, CardStrength, SourceRating, Verdict, Comment, ImageRef; new BoxNode/Card fields
  evaluate.rs        NEW  edit ops for ratings, kinds, links, comments, images; ClearScope
  edit.rs            validate() extended for new fields
  package.rs         NEW  Package { doc, assets, other_entries }; Assets; asset_key()
  file.rs            open/save now go through Package (old signatures kept as thin wrappers)
  layout/mod.rs      Layout gains card_roles, card_strength; Connector gains strength
  layout/tree.rs     owner index computed once; box_size includes image height
  layout/ports.rs    (unchanged unless reorder needs)
app/reasoncraft/src/
  settings.rs        NEW  Settings { theme, author, show_markers, eval_shading } persisted
  actions.rs         new variants; split: actions/evaluate.rs NEW, actions/insert.rs NEW
  shell/ribbon.rs    Evaluate tab + Home "Box" group; split ribbon/evaluate.rs NEW if >400 lines
  shell/backstage.rs + Settings page (author name)
  shell/building.rs  NEW  building panel (Boxes / Sources / Comments)
  shell/source_icons.rs NEW  17 vector source glyphs (shared by panel, ribbon, canvas)
  shell/link_modal.rs NEW  edit-link modal
  canvas/paint.rs, canvas/boxes.rs  rating visuals, markers, link glyph, comment badge, image
  canvas/comments.rs NEW  comment popover
  canvas/connect.rs  NEW  "Connect to…" pick mode
  canvas/drag.rs     reorder index + drops from the building panel
  canvas/images.rs   NEW  texture cache keyed by asset key
  doc_tab.rs         + assets: Assets, connect_mode: Option<Item>
  file_ops.rs        save/open Package; image insert
  i18n/en.ftl        new keys
```

---

### Task 1: Core model — source types, ratings, comments, links, images

**Files:**
- Modify: `crates/reasoncraft_core/src/model.rs`, `crates/reasoncraft_core/src/edit.rs` (validate), `crates/reasoncraft_core/src/lib.rs`
- Create: `crates/reasoncraft_core/src/evaluate.rs`
- Test: unit tests in `model.rs` and `evaluate.rs`

**Interfaces:**
- Consumes: step-1 `Document`, `BoxNode`, `Card`, `DocTab`-agnostic edit helpers (`touch`, `check_invariants`).
- Produces:
  - **`SourceType`.** `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)] #[serde(rename_all = "snake_case")] pub enum SourceType`, with variants:
    - `Source` (generic), `MyBasis`, `Assertion`, `ByDefinition`, `CaseStudy`, `CommonBelief`, `Data`, `Event`, `Example`, `ExpertOpinion`, `Law`, `Media`, `PersonalExperience`, `Publication`, `Quote`, `Statistic`, `Web`;
    - plus `#[serde(other)] Unknown` that maps to display as `Source`.

    Also `SourceType::ALL: [SourceType; 17]` (excluding Unknown, in the listed order) and `fn key(self) -> &'static str` (the snake_case name; `Unknown` → `"source"`).
  - **`BoxKind::Source`.** It becomes `Source { source_type: SourceType }`. JSON stays the same string, so old files still deserialize.
  - **Rating enums.** `pub enum Acceptance { Accept, Unsure, Reject }`, `pub enum CardStrength { Strong, Weak, Nil }`, `pub enum SourceRating { Solid, Shaky, Nil }`, `pub enum Verdict { Positive, NeedsWork, Problem }`. All `Copy, Eq, Serialize` (snake_case).
  - **`Comment`.** `pub struct Comment { pub id: Uuid, pub author: String, pub text: String, pub verdict: Option<Verdict>, pub created: String }`.
  - **`ImageRef`.** `pub struct ImageRef { pub asset: String, pub aspect: f32 }`, where `aspect` = height / width.
  - **New `BoxNode` fields.** All `#[serde(default, skip_serializing_if = …)]`:
    - `acceptance: Option<Acceptance>`
    - `source_rating: Option<SourceRating>`
    - `link: Option<String>`
    - `comments: Vec<Comment>`
    - `image: Option<ImageRef>`
  - **New `Card` field.** `strength: Option<CardStrength>` (same serde attrs).
  - **`impl Document` in `evaluate.rs`.** All return `Result<_, EditError>`, `touch()` on success, and run `check_invariants` in debug.
    - `fn set_acceptance(&mut self, id: NodeId, v: Option<Acceptance>)`: Claim/Premise only, else `NotAllowed`.
    - `fn set_source_rating(&mut self, id: NodeId, v: Option<SourceRating>)`: Source only.
    - `fn set_card_strength(&mut self, id: CardId, v: Option<CardStrength>)`.
    - `fn set_card_kind(&mut self, id: CardId, kind: CardKind)`: keeps strength.
    - `fn set_source_type(&mut self, id: NodeId, t: SourceType)`: Source only.
    - `fn set_link(&mut self, id: NodeId, link: Option<String>)`: any box. The value is trimmed and `Some("")` → `None`.
    - `fn add_comment(&mut self, id: NodeId, author: &str, text: &str, verdict: Option<Verdict>) -> Result<Uuid, EditError>`.
    - `fn remove_comment(&mut self, id: NodeId, comment: Uuid)`: `NotFound` if absent.
    - `fn set_image(&mut self, id: NodeId, image: Option<ImageRef>)`: any box except Note.
    - `pub enum ClearScope { Item(Item), Map }`.
    - `fn clear_evaluation(&mut self, scope: ClearScope)`: clears acceptance, source_rating and strength on the item and, for `Map`, on everything, including the parking area.
  - **`validate()` additions.**
    - `acceptance` only on Claim/Premise; `source_rating` only on Source; `image` not on Note.
    - Comment ids unique per box.
    - The error message names the offending id.
  - **Re-exports** in `lib.rs`: `SourceType, Acceptance, CardStrength, SourceRating, Verdict, Comment, ImageRef, ClearScope`.

- [ ] **Step 1: Write failing tests**
  - `source_type_compat_and_unknown_fallback`: deserialize `{"type":"source","source_type":"publication"}` → `Publication`; `"hologram"` → `Unknown` with `key() == "source"`; `SourceType::ALL.len() == 17`.
  - `ratings_follow_kind_rules`:
    - `set_acceptance` on a Source → `Err(NotAllowed)`;
    - `set_source_rating` on a Premise → `Err(NotAllowed)`;
    - a card set Strong then `set_card_kind(Objection)` → strength still `Some(Strong)`;
    - `validate()` fails on a hand-built document with `acceptance` on a Source.
  - `clear_map_clears_parking_too`: rate a parked card and a main premise, then `clear_evaluation(ClearScope::Map)` → all `None`.
  - `comments_add_remove`: add two, remove the first by id, one left; removing again → `NotFound`.
  - `link_trimmed_and_empty_is_none`: `set_link(id, Some("  ".into()))` → `link == None`.
  - `old_document_json_still_deserializes`: a step-1 JSON literal (no new fields) round-trips through `Document` equality.
- [ ] **Step 2: Run** `cargo test -p reasoncraft_core` → FAIL (compile).
- [ ] **Step 3: Implement** the types and ops. Update every `BoxKind::Source { source_type: String }` use site in core and app (fixtures, `NewLoose::Source` → `SourceType::Source`, painter keys `source.<key>`). Keep `en.ftl` source keys in sync with `SourceType::key()` (replace the six step-1 keys with the 17).
- [ ] **Step 4: Run** `cargo test --workspace` → PASS. Clippy clean.
- [ ] **Step 5: Commit** `feat(core): source types, ratings, comments, links and image refs`.

---

### Task 2: Layout carries roles and strength; image space

**Files:**
- Modify: `crates/reasoncraft_core/src/layout/{mod.rs,tree.rs}`, `crates/reasoncraft_core/src/connector.rs`
- Test: `crates/reasoncraft_core/tests/layout.rs`

**Interfaces:**
- Consumes: Task 1 fields.
- Produces:
  - `Layout.card_roles: IndexMap<CardId, Role>` and `Layout.card_strength: IndexMap<CardId, Option<CardStrength>>`, filled for every card.
  - `Connector.strength: connector::Strength` and `Connector.nil: bool`. The card's strength maps as Strong → `Strong`, None → `Normal`, Weak → `Weak`, Nil → `Weak` with `nil = true`. Non-card connectors get `Normal`/false.
  - **Owner index.** Roles are computed from an owner index built once per `layout()` call (`HashMap<Item, Parent>`), not via `card_role`/`owner_of` per card. Add `pub fn owner_index(doc: &Document) -> HashMap<Item, Parent>` in `model.rs` and use it in layout. `card_role` stays for other callers.
  - **Image height.** A box with `image: Some(ImageRef { aspect, .. })` adds `(width - 2*BOX_TEXT_PAD) * aspect.clamp(0.05, 4.0) + BOX_TEXT_PAD` above its text (applies to Claim/Premise/Category/Source). Constant `pub const IMAGE_GAP: f32 = 6.0;` sits between the image and the text.

- [ ] **Step 1: Write failing tests**
  - `layout_reports_roles_and_strength`: a strong reason, a weak objection, a nil rebuttal → `card_roles` and `card_strength` correct; the nil rebuttal's connector has `strength == Weak && nil`.
  - `image_adds_height`: the same premise with and without `ImageRef { aspect: 0.5 }` → height difference ≈ `(156 - 12) * 0.5 + 6 + IMAGE_GAP` within 0.5.
  - `layout_500_boxes_fast` (existing, ignored): still < 50 ms in release.
- [ ] **Step 2: Run** `cargo test -p reasoncraft_core --test layout` → FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** → PASS. Also run `cargo test -p reasoncraft_core --release --test layout -- --ignored`. App still compiles: update its `Connector` construction sites if any. The painter may keep `Strength::Normal` until Task 5.
- [ ] **Step 5: Commit** `feat(core): layout carries card roles and strength; image space in boxes`.

---

### Task 3: Package format — assets and unknown zip entries

**Files:**
- Create: `crates/reasoncraft_core/src/package.rs`
- Modify: `crates/reasoncraft_core/src/file.rs`, `crates/reasoncraft_core/src/lib.rs`, `Cargo.toml` (add `sha2`)
- Test: unit tests in `package.rs` and `file.rs`

**Interfaces:**
- Consumes: `Document`, `ImageRef`, step-1 `file` internals (temp + rename, read cap, `validate`).
- Produces:
  - **`Assets`.** `#[derive(Debug, Clone, Default, PartialEq)] pub struct Assets { entries: BTreeMap<String, Arc<[u8]>> }` with:
    - `fn insert(&mut self, ext: &str, bytes: Vec<u8>) -> String`: returns the key `"<sha256-hex>.<ext>"`; inserting the same bytes twice returns the same key.
    - `fn get(&self, key: &str) -> Option<Arc<[u8]>>`
    - `fn retain_referenced(&mut self, doc: &Document)`
    - `fn len(&self) -> usize`

    `Arc` keeps clones cheap; Assets are never part of `Document`.
  - **`Package`.** `pub struct Package { pub doc: Document, pub assets: Assets, pub other_entries: Vec<(String, Vec<u8>)> }`. `other_entries` = every zip entry that is neither `document.json` nor under `images/`, in original order.
  - **`file` API.**
    - `pub fn save_package(pkg: &Package, path: &Path) -> Result<(), FileError>` writes, in order:
      1. `document.json`;
      2. `images/<key>` for each asset referenced by the document (unreferenced assets are skipped);
      3. `other_entries` unchanged.
    - `pub fn open_package(path: &Path) -> Result<Package, FileError>`. Image entries are each capped at `MAX_ASSET_BYTES = 32 << 20`; over → `Malformed`. A referenced asset that is missing is NOT an error.
    - Step-1 `save(doc, path)` and `open(path)` stay as thin wrappers (empty assets / drop assets).
  - **Asset loading cap.** Total bytes read from the zip are capped at 512 MiB.

- [ ] **Step 1: Write failing tests**
  - `asset_key_is_content_hash`: the same bytes → the same key, ending in `.png`.
  - `package_roundtrip_keeps_ratings_comments_links`: a doc with ratings, comments, a link and an image + asset → `open_package(save_package(p)) == p`, comparing doc, assets and other_entries.
  - `unknown_zip_entries_survive_save`: build a zip by hand with `document.json` + `thumbnail.png` + `future/data.bin`; open, save, then reopen with the `zip` crate directly and assert both entries are present with identical bytes.
  - `open_with_missing_asset_keeps_image_ref`: the doc references `abc.png` with no entry → opens; `assets.get("abc.png") == None`; `image` still set.
  - `unreferenced_assets_not_written`.
  - `oversized_asset_rejected`: use a private `open_package_with_limits` helper with a small asset limit.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** **Step 4: Run** → PASS.
- [ ] **Step 5: Commit** `feat(core): .rcraft package with image assets and preserved unknown entries`.

---

### Task 4: Settings, evaluation actions, Evaluate ribbon tab and keyboard

**Files:**
- Create: `app/reasoncraft/src/settings.rs`, `app/reasoncraft/src/actions/evaluate.rs`
- Modify: `app/reasoncraft/src/{app.rs, actions.rs, shell/ribbon.rs (or create shell/ribbon/evaluate.rs), shell/icons.rs, canvas/input.rs}`, `app/reasoncraft/i18n/en.ftl`, `README.md`
- Test: `app/reasoncraft/src/actions/tests.rs`, `app/reasoncraft/tests/ui.rs`, input.rs unit tests

**Interfaces:**
- Consumes: Task 1 ops; step-1 `Action`, `apply`, `is_enabled`, `DocTab::edit`, ribbon helpers `large`/`small`/`group`.
- Produces:
  - **`Settings`.** `#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)] pub struct Settings { pub theme: ThemeChoice, pub author: String, pub show_markers: bool, pub eval_shading: bool }`, with defaults Dark / "" / true / true. It is persisted under the existing key `"settings"`; old saved settings (theme only) load with the defaults filled in via `#[serde(default)]`. `ReasonCraftApp.theme` is replaced by `ReasonCraftApp.settings: Settings`.
  - **`Rate` enum.** `pub enum Rate { High, Mid, Low, Clear }`. Mapping per selection: Claim/Premise → Accept/Unsure/Reject; Card → Strong/Weak/Nil; Source → Solid/Shaky/Nil; Clear → None.
  - **New `Action` variants:**
    - `Rate(Rate)`: applies to the selection.
    - `SetAcceptance(Option<Acceptance>)`, `SetStrength(Option<CardStrength>)`, `SetSourceRating(Option<SourceRating>)`: explicit, used by the ribbon.
    - `ClearEvaluation(ClearScope)`
    - `ToggleMarkers`, `ToggleShading`
  - **`is_enabled`.** Each rating action is enabled only when the selection's kind accepts it, so the ribbon shows the other groups greyed out.
  - **Keyboard** (not while editing): Alt+1 → `Rate(High)`, Alt+2 → `Rate(Mid)`, Alt+3 → `Rate(Low)`, Alt+0 → `Rate(Clear)`. `key_to_action` stops returning `None` for every Alt combo; only these four, plus Alt+Left/Right from Task 9, are mapped.
  - **Evaluate ribbon tab** groups:
    - `group.eval_claim` (Accept / Unsure / Reject);
    - `group.eval_card` (Strong / Weak / Nil);
    - `group.eval_source` (Solid / Shaky / Nil);
    - `group.eval_clear` (Selection / Map);
    - `group.eval_view` (Show markers toggle, Evaluation shading toggle).

    Buttons show `selected` when the current value matches. The View ribbon tab gets the same two toggles in a `group.evaluation` group (spec §6 lists shading under View). Icons: add `Icon::{Accept, Unsure, Reject, Strong, Weak, Nil, Clear, Markers, Shading}` as simple vector glyphs.
  - **README:** add Alt+1/2/3/0 to the selected-box shortcut table.

- [ ] **Step 1: Write failing tests**
  - Unit test `rate_maps_per_selection_kind`: for a premise, card and source selection, `Rate(High)` sets Accept / Strong / Solid; `Rate(Clear)` clears.
  - Unit test `rating_refused_records_no_history`: with the root claim selected, `SetStrength(Some(Strong))` → no history entry and `is_enabled == false`.
  - Unit test `alt_digits_map_to_rate`, and `alt_other_keys_unmapped`.
  - Kittest `evaluate_tab_rates_selected_card`: select a card, click Evaluate → Strong → `card.strength == Some(Strong)`; the Strong button shows selected.
  - Kittest `settings_from_old_storage_fill_defaults`: deserialize `{"theme":"Light"}` → author "" and toggles true.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** **Step 4: Run** → PASS.
- [ ] **Step 5: Commit** `feat(app): evaluation actions, Evaluate ribbon tab, settings and Alt shortcuts`.

---

### Task 5: Evaluation visuals on the map

**Files:**
- Modify: `app/reasoncraft/src/canvas/{paint.rs, boxes.rs}`, `app/reasoncraft/src/theme.rs` (tokens + contrast test), `app/reasoncraft/i18n/en.ftl`
- Test: unit tests in `paint.rs` and `theme.rs`, kittest smoke test

**Interfaces:**
- Consumes: `Layout.card_roles`, `Layout.card_strength`, `Connector.strength/nil` (Task 2), `Settings.show_markers/eval_shading` (Task 4), Task 1 fields.
- Produces (painting rules, spec §5):
  - **Card frame.** Fill alpha: Strong 0.22, None 0.12, Weak 0.08, Nil 0.06. With `eval_shading == false`, always 0.12. Border width: Strong 1.8, otherwise 1.0. Nil uses the new token `Tokens.nil` (a grey) for frame, border and connector instead of the role colour.
  - **Rating label.** Built by `pub fn rating_label(role: Role, strength: Option<CardStrength>) -> String`:
    - Strong → `t("rating.strong.<role>")`, e.g. "●● strongly supports";
    - Weak → `t("rating.weak.<role>")` ("● weakly supports");
    - None → `t("role.<role>")`;
    - Nil → `t("rating.nil.<role>")` ("does not support" / "does not oppose" / "does not rebut").
  - **Connectors.** `stroke_px(kind, connector.strength, zoom)`, coloured `Tokens.nil` when `connector.nil`.
  - **Markers** (when `show_markers`):
    - Acceptance: ✓ / ? / × in a 16 px circle at the box's top-left corner, inset 4 px. Accept uses the `reason` colour, Unsure `rebuttal`, Reject `objection`, drawn as vector shapes (not font glyphs).
    - Source rating: the same corner marker (Solid ✓, Shaky ?, Nil ×).
  - **Contrast.** Rating label text on each tinted fill (Strong 0.22 over `canvas`) meets ≥ 4.5:1 in both themes; adjust token values if needed. This closes the step-1 deferred minor.

- [ ] **Step 1: Write failing tests**
  - `rating_label_variants`: all 12 (role × strength) combinations return the expected English strings.
  - `frame_alpha_by_strength_and_shading`: a pure `fn frame_alpha(strength: Option<CardStrength>, shading: bool) -> f32` returns the values above.
  - `label_contrast_on_tinted_frames`: in theme.rs, both themes, all three roles, strong tint.
  - Kittest `rated_map_renders`: load a sample map, rate several items, and run 3 frames without panic.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.**
- [ ] **Step 4: Run** → PASS. Then run the app, rate items, and screenshot dark and light themes into the SDD workspace. Look for: labels inside frames, nil cards greyed, markers not overlapping text.
- [ ] **Step 5: Commit** `feat(app): rating visuals — tint, borders, labels, markers, connector strength`.

---

### Task 6: Sources — adding, icons, changing type

**Files:**
- Create: `app/reasoncraft/src/shell/source_icons.rs`
- Modify: `app/reasoncraft/src/{actions.rs, canvas/plus_menu.rs, canvas/boxes.rs, canvas/input.rs, shell/ribbon.rs, shell/icons.rs}`, `en.ftl`, `README.md`
- Test: actions tests, plus_menu tests, kittest

**Interfaces:**
- Consumes: Task 1 `SourceType`, `set_source_type`, step-1 `add_box`, `add_loose`.
- Produces:
  - **Actions.** `Action::AddSource(SourceType)` targets the selection: a premise or claim → `add_box(Parent::Box(id), BoxKind::Source{..}, "")`; nothing selected → loose (`NewLoose::Source` gains a `SourceType` payload). The new source is selected and in edit mode. `Action::SetSourceType(SourceType)` applies to a selected source.
  - **Icons.** `pub fn paint_source_icon(painter: &Painter, rect: Rect, t: SourceType, colour: Color32)` draws 17 distinct, original vector glyphs: simple strokes/fills, recognisable at 14 px and 32 px. `Unknown` and `Source` share the generic page glyph. These replace the step-1 `doc_glyph` on the canvas.
  - **"+" menu.** A "Source ▸" submenu listing all 17 types with icons (premise/claim selected).
  - **Home ribbon.** `group.add` gains "Source ▾", a dropdown with all 17 types.
  - **Keyboard.** Ctrl+Shift+E adds a generic Source (`AddSource(SourceType::Source)`) when not editing and while editing (finish + add). Add it to the README.
  - **en.ftl.** `source.<key>` names for all 17 types, plus `source.prompt.<key>` hint text shown as a placeholder in the inline editor for an empty source (e.g. "Title, author, outlet, date").

- [ ] **Step 1: Write failing tests**
  - `add_source_targets_selection`: a premise selected → the source is a child of the premise and selected; a category selected → `is_enabled(AddSource) == false`.
  - `set_source_type_changes_type`.
  - `ctrl_shift_e_adds_source` (key_to_action).
  - Kittest `plus_menu_source_submenu_adds_expert_opinion`.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** **Step 4: Run** → PASS. Screenshot of the 17 icons (a debug-only preview is fine), and look at it.
- [ ] **Step 5: Commit** `feat(app): typed sources with icons, plus menu, ribbon and shortcut`.

---

### Task 7: Comments with presets; author name setting

**Files:**
- Create: `app/reasoncraft/src/canvas/comments.rs`
- Modify: `app/reasoncraft/src/{actions.rs, shell/backstage.rs, canvas/boxes.rs, canvas/mod.rs, shell/ribbon.rs}`, `en.ftl`
- Test: actions tests, kittest

**Interfaces:**
- Consumes: Task 1 `add_comment`/`remove_comment`, Task 4 `Settings.author`.
- Produces:
  - **Presets.** `pub const PRESETS: [(&str /*i18n key*/, Verdict); 12]`. Keys and verdicts:

    | Key | Verdict |
    |---|---|
    | `comment.preset.great_work` | Positive |
    | `comment.preset.good_structure` | Positive |
    | `comment.preset.strong_evidence` | Positive |
    | `comment.preset.excellent_evaluation` | Positive |
    | `comment.preset.getting_there` | NeedsWork |
    | `comment.preset.refinement_needed` | NeedsWork |
    | `comment.preset.more_evidence_needed` | NeedsWork |
    | `comment.preset.too_accepting` | NeedsWork |
    | `comment.preset.unclear` | Problem |
    | `comment.preset.rethink_structure` | Problem |
    | `comment.preset.source_not_appropriate` | Problem |
    | `comment.preset.inconsistent_evaluation` | Problem |

    English texts are the builder's call (short, neutral, none copied from elsewhere).
  - **Actions.**
    - `Action::AddComment { node: NodeId, text: String, verdict: Option<Verdict> }` uses `settings.author`; an empty author is stored as `t("comment.anonymous")`.
    - `Action::RemoveComment { node: NodeId, comment: Uuid }`.
    - `Action::OpenComments(NodeId)` and `Action::CloseComments` are UI state on DocTab: `comments_open: Option<NodeId>`.
  - **Badge.** A speech-bubble badge with the comment count at the box's top-right corner (outside the text area, inset like the markers). Its colour follows the worst verdict present: Problem → `objection`, NeedsWork → `rebuttal`, Positive → `reason`, none → `text_muted`. Clicking it pushes `OpenComments`.
  - **Popover** (`comments.rs`), anchored next to the box with a fixed width of 280 px. It contains:
    - a list showing author, verdict dot, wrapped text and a delete ×;
    - a multiline input;
    - a verdict picker (none / positive / needs work / problem);
    - an "Add" button;
    - a "Presets ▾" menu that adds the preset immediately.

    Esc or a click outside closes it.
  - **Home ribbon.** `group.add` gains "Comment", which opens the popover for the selected box.
  - **Backstage.** A "Settings" page with an author-name text field (applied via `Action::SetAuthor(String)`, persisted) and the theme choice.
  - **Keyboard.** Ctrl+Shift+M opens comments for the selected box (README).

- [ ] **Step 1: Write failing tests**
  - `add_comment_uses_author_or_anonymous`.
  - `remove_comment_records_history`: one history entry; undo restores the comment.
  - `badge_colour_follows_worst_verdict`: a pure fn `badge_colour(&[Comment], &Tokens) -> Color32`.
  - Kittest `preset_adds_comment_and_badge_shows_count`.
  - Kittest `long_comment_wraps_in_popover`: a 600-char comment; the popover width stays 280.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** **Step 4: Run** → PASS.
- [ ] **Step 5: Commit** `feat(app): comments with presets, badge, popover and author setting`.

---

### Task 8: Building panel with drag-and-drop

**Files:**
- Create: `app/reasoncraft/src/shell/building.rs`, `app/reasoncraft/src/actions/insert.rs`
- Modify: `app/reasoncraft/src/{shell/mod.rs, canvas/drag.rs, canvas/mod.rs, actions.rs}`, `en.ftl`
- Test: unit tests in `insert.rs` and `drag.rs`, kittest

**Interfaces:**
- Consumes: Task 6 `AddSource`/icons, Task 7 presets, step-1 `drop_target`, `DropTarget`, `Hit`, `can_attach`.
- Produces:
  - **`Insertable`.** `pub enum Insertable { Reason, Objection, CoPremise, Category, Note, Source(SourceType), Comment(usize /*PRESETS index*/) }`.
  - **Actions.**
    - `Action::Insert { what: Insertable, to: Option<Parent>, on: Option<NodeId> }`:
      - `to = Some(parent)` for cards, categories, notes and sources dropped on the canvas;
      - `on = Some(node)` for co-premise drops (the node's card gets the premise) and comment drops (the comment goes on that node);
      - both `None` = "insert at the current selection", i.e. the same as clicking the ribbon.
    - The result is selected (and edited, for text items).
  - **Drop targets.** `pub fn panel_drop_target(doc: &Document, layout: &Layout, what: &Insertable, world: Pos2, zoom: f32) -> Option<(DropTarget, bool /*allowed*/)>`. It reuses `hit_test`:
    - a box or premise → `Parent::Box` (or `on` for CoPremise/Comment);
    - a card's bottom band or frame → `Parent::Card` (cards only);
    - empty canvas → `Parent::Parking`. Comments are refused there; everything else is allowed.

    Allowed-ness comes from `can_attach`.
  - **Panel.** Collapsible sections:
    - "Boxes": Reason, Objection, Co-premise, Category, Note;
    - "Sources": 17 types with icons, in a 3-column grid;
    - "Comments": the 12 presets, coloured by verdict.

    Click = insert at the selection (disabled items greyed via the same `is_enabled` logic). Drag = an egui drag payload `Insertable`, showing a ghost chip under the cursor and, over the canvas, a green/red target highlight using `Tokens.reason` / `Tokens.objection`. Drop → `Action::Insert`; a refused drop does nothing. The panel replaces the step-1 placeholder and keeps `default_size(220.0)`.

- [ ] **Step 1: Write failing tests**
  - `panel_drop_target_rules`: a source over a category → not allowed; a reason over a card bottom → `Parent::Card` allowed; a comment over empty canvas → not allowed; a note over empty canvas → Parking allowed.
  - `insert_at_selection_matches_ribbon`: `Insert{Reason, None, None}` gives the same document as `AddReason`.
  - Kittest `refused_panel_drop_changes_nothing`: drag a Source onto a category in the harness → document unchanged, `history.undo.len()` unchanged.
  - Kittest `panel_click_adds_note_to_selection`.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** **Step 4: Run** → PASS. Then try the panel drags by hand in the app.
- [ ] **Step 5: Commit** `feat(app): building panel with click-to-insert and drag-and-drop`.

---

### Task 9: Sibling reorder, "Connect to…", shortcut rebinding

**Files:**
- Create: `app/reasoncraft/src/canvas/connect.rs`
- Modify: `app/reasoncraft/src/{actions.rs, canvas/drag.rs, canvas/input.rs, canvas/plus_menu.rs, shell/ribbon.rs, shell/statusbar.rs, doc_tab.rs}`, `README.md`, `en.ftl`
- Test: drag.rs unit tests, input.rs unit tests, kittest

**Interfaces:**
- Consumes: step-1 `move_item(item, to, index)`, `drop_target`, `navigate`.
- Produces:
  - **`Action::Move` gains an index:** `Action::Move { item: Item, to: Parent, index: Option<usize> }`. Update all step-1 call sites.
  - **Drop index.** `drop_target` computes `index` from the pointer x relative to the target parent's existing children (cards and boxes in display order). Dropping onto the item's current parent at a new position reorders. A 3 px vertical insertion bar is drawn between siblings in `Tokens.accent`. Co-premise reordering: dragging a premise within its own card reorders `card.premises`, via `move_item(Item::Box(p), Parent::Card(same), Some(i))`.
  - **Keyboard reordering.** `Action::Reorder(Dir)` with Left/Right only. It moves the selected item (or premise within its card) one place, is no-op at the edges, and keeps the selection. Mapped to Alt+Left / Alt+Right when not editing.
  - **"Connect to…" mode.**
    - `Action::StartConnect` is enabled when the selection is a parking item; it sets `DocTab.connect_mode = Some(item)`.
    - The status bar shows `t("connect.hint")` ("Click a box or card to connect … — Esc to cancel").
    - The canvas highlights valid targets on hover (green/red).
    - A click on an allowed target → `Action::Move { item, to, index: None }` and the mode ends. Esc → `Action::CancelConnect`.
    - Keyboard path: arrows navigate as usual while in the mode, and Enter connects to the selected target.
    - Entry points: Home ribbon `group.editing` "Connect to…", a "+" menu entry for parked items, and Ctrl+Shift+K.
  - **Shortcut rebinding.**
    - The whole-card objection shortcut changes from Ctrl+Shift+Tab to Ctrl+Shift+O, in both modes. Ctrl+Shift+Tab is no longer mapped.
    - Ctrl+Shift+R stays for whole-card reason.
    - README tables and the `key_to_action` doc comment are updated.

- [ ] **Step 1: Write failing tests**
  - `drop_index_from_pointer_x`: three sibling cards; pointer between the 1st and 2nd → index 1.
  - `reorder_with_alt_arrows`: Alt+Right moves the selected card from 0 to 1; at the last position, no change and no history.
  - `co_premise_reorder_within_card`.
  - `connect_mode_click_connects_parking_card` (kittest).
  - `connect_mode_esc_cancels`.
  - `ctrl_shift_o_targets_whole_card_objection_and_ctrl_shift_tab_unmapped`.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** **Step 4: Run** → PASS.
- [ ] **Step 5: Commit** `feat(app): drag and keyboard reorder, Connect to…, whole-card objection on Ctrl+Shift+O`.

---

### Task 10: Change card type and links (Home "Box" group)

**Files:**
- Create: `app/reasoncraft/src/shell/link_modal.rs`
- Modify: `app/reasoncraft/src/{actions.rs, shell/ribbon.rs, shell/modal.rs (if shared), canvas/boxes.rs, canvas/mod.rs}`, `Cargo.toml` (add `open = "5"`), `en.ftl`
- Test: actions tests, unit tests, kittest

**Interfaces:**
- Consumes: Task 1 `set_card_kind`, `set_link`, Task 6 `SetSourceType`.
- Produces:
  - **New Home ribbon group `group.box`.**
    - "Type ▾": a selected card offers Reason / Objection (`Action::SetCardKind(CardKind)`); a selected source offers the 17 source types (`SetSourceType`); otherwise disabled.
    - "Link": opens the link modal.
    - "Image": Task 11.
  - **Link modal.** URL text field, "Remove link", "Save", "Cancel". Validation is a pure fn `pub fn normalize_link(s: &str) -> Result<Option<String>, LinkError>`:
    - accepts `http://`, `https://` and `mailto:`;
    - bare domains like `example.org/x` get `https://` prepended;
    - empty → `Ok(None)`;
    - anything else → `Err(Invalid)` with the message `t("link.invalid")`.

    Saving dispatches `Action::SetLink(Option<String>)`.
  - **Link glyph.** A 12 px chain glyph at the box's bottom-right. Click (or Ctrl+click while editing) → `Action::OpenLink(NodeId)`, which calls `open::that(url)`. Failures show an error modal via `t("link.open_failed")`. Tooltip = the URL.

- [ ] **Step 1: Write failing tests**
  - `normalize_link_cases`: `"https://a.b"` → same; `"a.b/c"` → `"https://a.b/c"`; `"mailto:x@y.z"` → same; `""` → `None`; `"javascript:alert(1)"` → `Err`; `"ftp://x"` → `Err`.
  - `set_card_kind_via_ribbon_keeps_strength` (kittest).
  - `open_link_uses_stored_url`: inject an `open` function through a trait `LinkOpener` held on the app (real impl = `open::that`, test impl records calls).
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** **Step 4: Run** → PASS.
- [ ] **Step 5: Commit** `feat(app): change card/source type and box links`.

---

### Task 11: Images in boxes

**Files:**
- Create: `app/reasoncraft/src/canvas/images.rs`
- Modify: `app/reasoncraft/src/{doc_tab.rs, file_ops.rs, actions.rs, shell/ribbon.rs, canvas/boxes.rs, canvas/mod.rs}`, `Cargo.toml` (`image` with png/jpeg/webp), `en.ftl`
- Test: unit tests in `images.rs` and `file_ops/tests.rs`, kittest, e2e extension

**Interfaces:**
- Consumes: Task 3 `Package`/`Assets`/`save_package`/`open_package`; Task 1 `set_image`; Task 2 image layout height.
- Produces:
  - **DocTab.** `DocTab.assets: Assets`, kept outside `History`. Assets are add-only during a session. Undoing an image insert just leaves the bytes unused; `retain_referenced` runs on save.
  - **File ops.** `file_ops` saves and opens via `save_package`/`open_package`, filling `tab.assets`. The `FileDialogs` trait gains `fn pick_image(&self) -> Option<PathBuf>` (filter `t("file.images")`: png, jpg, jpeg, webp).
  - **`prepare_image`.** `pub fn prepare_image(bytes: &[u8]) -> Result<(Vec<u8> /*png*/, f32 /*aspect*/), ImageError>`:
    - rejects input over 40 MiB (`TooLarge`);
    - decodes with limits: max 12000×12000, else `TooLarge`;
    - downscales so the longest side ≤ 1600 px (Lanczos3);
    - re-encodes as PNG;
    - returns the aspect.

    `ImageError { TooLarge, Unsupported, Corrupt }` maps to modal texts `t("image.error.<variant>")`.
  - **Actions.**
    - `Action::InsertImage` (Home "Image" / Box group) picks a file, runs `prepare_image` (synchronously; the 1600 px cap keeps it fast enough), then `assets.insert("png", bytes)` and `edit(set_image(id, Some(ImageRef{asset, aspect})))`.
    - `Action::RemoveImage`.
    - Both are enabled for Claim/Premise/Category/Source selections.
  - **Texture cache.** `ImageCache` (in `images.rs`) maps asset key → `egui::TextureHandle`, loaded lazily from `tab.assets` and evicted when the tab closes. The painter draws the image above the text at layout-reserved size. A missing asset or decode failure → a placeholder: a dashed rect in `text_muted` with `t("image.missing")`.

- [ ] **Step 1: Write failing tests**
  - `oversized_image_is_downscaled`: a 4000×2000 generated PNG → output longest side 1600, aspect 0.5.
  - `corrupt_image_shows_error`: kittest with FakeDialogs returning a path to garbage bytes → error modal, document unchanged.
  - `image_roundtrip_through_save`: insert, save, reopen → same asset key present, `ImageRef` intact.
  - `undo_image_insert_keeps_assets_out_of_history`: after insert + undo, `history.undo[..]` documents contain no bytes (structural: `Document` has no byte fields) and the next save writes no `images/` entry.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** **Step 4: Run** → PASS. Manual: insert a photo into a premise, save, reopen, screenshot.
- [ ] **Step 5: Commit** `feat(app): images in boxes with asset store outside undo history`.

---

### Task 12: End-to-end verification and docs

**Files:**
- Modify: `app/reasoncraft/tests/e2e.rs`, `README.md`, `docs/dev/todo-history.md`
- Test: e2e

- [ ] **Step 1: Extend `e2e_build_save_reopen`.** Through actions only, add:
  - ratings on a claim, a card and a source;
  - a source of type ExpertOpinion with a link;
  - two comments (one preset);
  - an image (FakeDialogs + generated PNG);
  - a reordered sibling;
  - a parking item connected via `StartConnect` + `Move`.

  Save, reopen, and assert full `Document` equality, equal `Assets`, and an equal layout within 0.5 px.
- [ ] **Step 2: Run** `cargo test --workspace` → PASS; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo build --release -p reasoncraft` succeeds.
- [ ] **Step 3: Manual check** in the release build: evaluate a small map by keyboard (Alt+digits), add sources from the panel by drag, add comments via presets, reorder with Alt+arrows, connect a parking item, insert an image, and toggle markers/shading. Take screenshots in dark and light. Record the results in the `docs/dev/todo-history.md` Review section for step 2, and tick the step-2 carry-over items that were done.
- [ ] **Step 4: README.** Add a feature overview for step 2 and keep the shortcut tables exact.
- [ ] **Step 5: Commit** `docs: step 2 verification and usage notes`.

---

## Decisions made in this plan (not in the spec)

- **Source fields.** Sources keep their citation as box text, plus an optional link. Structured fields (author/title/outlet/date) are deferred until the Outline/Essay export (step 4) needs them.
- **Templates.** The building panel's Templates section is deferred to step 6, alongside the bundled templates.
- **Colour and font size** in the Home "Box" group are deferred to step 5 (polish).
- **Comments** attach to boxes, not to card frames.
- **Rating shortcuts.** Alt+1/2/3/0 cover all three rating scales, interpreted by the selection's kind.
- **New shortcuts.**
  - Ctrl+Shift+E: add source.
  - Ctrl+Shift+M: comments.
  - Ctrl+Shift+K: Connect to….
  - Ctrl+Shift+O: whole-card objection.
- **Images** are normalised to PNG, at most 1600 px on the longest side, and stored content-addressed in the `.rcraft` zip.
- **No images in Notes.** Sticky notes cannot hold an image. This deviates from spec §4 ("any box can hold an image"): a note is a short aside next to the map, and an image would turn it into a second kind of content box.
