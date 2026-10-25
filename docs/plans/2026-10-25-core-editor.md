# ReasonCraft Step 1 — Core Editor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A usable cross-platform desktop editor that creates, edits, lays out, draws (style D, curved connectors), saves and opens argument maps and grouping maps, with a parking area, undo/redo, keyboard control, dark/light theme, document tabs and a ribbon shell (Home + View).

**Architecture:** A Cargo workspace with two crates:
- `reasoncraft_core`: pure Rust, no UI. Holds the document model, structure rules, snapshot-based undo history, the `.rcraft` file format, automatic layout and connector geometry. Fully unit-tested.
- `reasoncraft` (binary): an eframe/egui app. It renders a `Layout` from core, turns input into `Action`s, and applies them to the document through core.

The app never mutates the document directly; every change goes through core so undo, rules and layout stay consistent.

**Tech Stack:** Rust 1.94 stable (edition 2024); egui/eframe 0.37 (glow backend, accesskit, persistence); emath 0.37 (geometry types in core); serde + serde_json; zip 9; uuid 1 (v4, serde); indexmap 2 (serde); rfd 0.17 (file dialogs); fluent-bundle 0.16 (UI strings); egui_kittest 0.37 (UI tests); thiserror 2.

**Spec:** `docs/superpowers/specs/2026-10-25-reasoncraft-design.md` (§4 model, §5 map look, §6 app layout, §8 editing, §9 file format, §10 theme). This plan implements spec §13 step 1 only; steps 2–6 get their own plans.

## Global Constraints

- Pure native egui UI: no webview, no Tauri. Must build and run on Windows, macOS and Linux.
- Licence: dual MIT / Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`, `license = "MIT OR Apache-2.0"` in every `Cargo.toml`).
- App name `ReasonCraft`; file extension `.rcraft`; app id `nl.reasoncraft.app`.
- Works fully offline: no network calls.
- All user-visible UI text goes through `i18n::t("key")`, with English strings in `app/reasoncraft/i18n/en.ftl`.
- Box text is never clipped: heights come from measuring the wrapped text.
- Connector rules (spec §5):
  - Fixed-size arrowheads (9 px at 100%).
  - Every connector ends in a straight 16 px stub.
  - Minimum visible stroke of 1.25 screen px.
  - Connectors sharing an attach point spread 16 px apart, ordered by child x.
  - A whole-card target attaches at the card frame's bottom centre.
  - The rating label sits inside the card frame at the top-right.
- Colours come only from `theme::Tokens`; role colours: reason `#4CC073`, objection `#E05A5A`, rebuttal `#E0A05A` (dark theme), darker variants for light theme.
- `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace` pass at every commit.

## Review Focus

1. **Long unbroken text** (URLs, 60+ char words, Dutch compounds) must wrap inside the box width and grow the box height. It must never overflow or clip. → Task 6 test `long_unbroken_word_grows_height` (core) and Task 9 test `egui_measure_breaks_long_words` (app).
2. **Deleting things others depend on:** deleting a premise that is the target of cards, the last premise of a card, or a card that has whole-card attackers must leave no dangling references. → Task 3 tests `remove_last_premise_removes_card`, `remove_branch_leaves_no_orphans`, `remove_keep_children_moves_to_parking`.
3. **Opening bad files:** opening a non-zip, a zip without `document.json`, malformed JSON or a newer schema shows a clear error and loses no open work. A failed save never corrupts the existing file (atomic write). → Task 5 tests `open_rejects_non_zip`, `open_rejects_newer_schema`, `save_is_atomic_on_failure`; Task 11 test `failed_open_keeps_tabs`.
4. **Undo/redo across structural edits:** undo restores the exact structure, including the parking area; a new edit clears redo; undo past a save marks the tab dirty again. → Task 4 tests `undo_restores_parking`, `new_edit_clears_redo`; Task 11 test `undo_after_save_marks_dirty`.
5. **Big and deep maps:** a 500-box map lays out in under 50 ms in release builds, and a chain 200 levels deep neither overflows the stack nor overlaps. → Task 6 tests `layout_500_boxes_fast` (`#[ignore]`, run with `--release --ignored`) and `deep_chain_no_overlap`.

---

## File Structure

```
reasoncraft/
  Cargo.toml                      workspace, shared deps, lints
  LICENSE-MIT  LICENSE-APACHE  README.md  .gitignore
  crates/reasoncraft_core/
    Cargo.toml
    src/lib.rs                    re-exports
    src/ids.rs                    NodeId, CardId, Item, Parent
    src/model.rs                  BoxKind, BoxNode, CardKind, Card, Document, DocMeta, MapKind, Role
    src/rules.rs                  allowed-children table, can_attach
    src/edit.rs                   add/move/remove/set_text operations, EditError
    src/history.rs                snapshot undo/redo
    src/file.rs                   .rcraft read/write, FileError
    src/layout/mod.rs             Measure trait, LayoutParams, Layout, layout()
    src/layout/tree.rs            tidy-tree placement
    src/layout/ports.rs           attach points + port spreading
    src/connector.rs              curved path geometry, stroke widths
    src/fixtures.rs               test-document builders (feature `test-fixtures`)
  app/reasoncraft/
    Cargo.toml
    i18n/en.ftl
    assets/icon.png               original ReasonCraft icon
    src/main.rs                   viewport, custom title bar setup
    src/app.rs                    ReasonCraftApp: tabs, settings, frame loop
    src/doc_tab.rs                DocTab: document, history, path, dirty, selection, editing, camera
    src/actions.rs                Action enum + apply()
    src/i18n.rs                   t() lookup over embedded en.ftl
    src/theme.rs                  Tokens (dark/light), ThemeChoice, apply to egui
    src/file_ops.rs               New/Open/Save/Save As/close-confirm via rfd
    src/shell/{mod,titlebar,tabs,ribbon,statusbar}.rs
    src/canvas/{mod,camera,measure,paint,hit,input,plus_menu}.rs
    tests/ui.rs                   egui_kittest tests
```

---

### Task 1: Workspace, licences and core ID/model types

**Files:**
- Create: `Cargo.toml`, `LICENSE-MIT`, `LICENSE-APACHE`, `README.md`, `crates/reasoncraft_core/{Cargo.toml,src/lib.rs,src/ids.rs,src/model.rs}`, `app/reasoncraft/{Cargo.toml,src/main.rs}` (main is a stub that opens an empty eframe window titled "ReasonCraft")
- Modify: `.gitignore` (already has `.superpowers/`, `target/`)
- Test: `crates/reasoncraft_core/src/model.rs` (unit tests module)

**Interfaces:**
- Produces:
  - `NodeId(Uuid)` and `CardId(Uuid)`, both `Copy, Eq, Hash, Ord, Serialize, Deserialize`, with `::new()` (v4).
  - `enum Item { Box(NodeId), Card(CardId) }`
  - `enum Parent { Box(NodeId), Card(CardId), Parking }`
  - `enum BoxKind { Claim, Premise, Category, Source { source_type: String }, Note }`. `source_type` is a stable key such as `"publication"`; the full source-type list arrives in step 2.
  - `struct BoxNode { id: NodeId, kind: BoxKind, text: String, width: Option<f32>, children: Vec<Item>, #[serde(flatten)] extra: serde_json::Map<String, Value> }`. `children` holds cards targeting this box plus sources/notes/categories under it, in display order.
  - `enum CardKind { Reason, Objection }`
  - `struct Card { id: CardId, kind: CardKind, premises: Vec<NodeId>, children: Vec<CardId>, extra }`. `children` holds cards targeting the whole card.
  - `struct DocMeta { title: String, author: String, created: String, modified: String, language: String }` (RFC 3339 strings; language `"en"` or `"nl"`)
  - `struct Document { schema_version: u32, meta: DocMeta, root: NodeId, boxes: IndexMap<NodeId, BoxNode>, cards: IndexMap<CardId, Card>, parking: Vec<Item>, extra }`
  - `Document::new(claim_text: &str) -> Document`, with `schema_version = 1`.
  - `pub const SCHEMA_VERSION: u32 = 1`
  - `enum MapKind { Empty, Reasoning, Grouping }` and `Document::map_kind(&self) -> MapKind`. It looks at the root's children: any card → Reasoning, any Category → Grouping, otherwise Empty.
  - `enum Role { Supports, Opposes, Rebuts }` and `Document::card_role(&self, id: CardId) -> Role`:
    - Reason → Supports.
    - Objection whose owner (the box or card it targets) sits inside an Objection card → Rebuts.
    - Otherwise → Opposes.
    - Loose objections → Opposes.
  - `Document::owner_of(&self, item: Item) -> Option<Parent>`. A linear scan is fine. It returns `Some(Parent::Parking)` for parking items and `None` for the root.

Workspace `Cargo.toml` uses `[workspace] resolver = "3"`, `members = ["crates/reasoncraft_core", "app/reasoncraft"]`, and `[workspace.package] edition = "2024", license = "MIT OR Apache-2.0", rust-version = "1.94"`. Pin shared deps in `[workspace.dependencies]` with the versions from the Tech Stack line. Set `[workspace.lints.clippy] all = "warn"`.

- [ ] **Step 1: Write failing tests** in `model.rs`:

```rust
#[test]
fn new_document_has_root_claim() {
    let d = Document::new("Robots create more jobs");
    let root = &d.boxes[&d.root];
    assert_eq!(root.kind, BoxKind::Claim);
    assert_eq!(root.text, "Robots create more jobs");
    assert_eq!(d.schema_version, SCHEMA_VERSION);
    assert!(d.parking.is_empty());
    assert_eq!(d.map_kind(), MapKind::Empty);
}

#[test]
fn rebuttal_is_derived_from_structure() {
    // root <- objection(card A, premise p) <- objection(card B) => B rebuts
    let mut d = Document::new("C");
    let a = CardId::new(); let p = NodeId::new(); let b = CardId::new(); let q = NodeId::new();
    d.boxes.insert(p, BoxNode::new(p, BoxKind::Premise, "P"));
    d.boxes.insert(q, BoxNode::new(q, BoxKind::Premise, "Q"));
    d.cards.insert(a, Card::new(a, CardKind::Objection, vec![p]));
    d.cards.insert(b, Card::new(b, CardKind::Objection, vec![q]));
    d.boxes.get_mut(&d.root).unwrap().children.push(Item::Card(a));
    d.boxes.get_mut(&p).unwrap().children.push(Item::Card(b));
    assert_eq!(d.card_role(a), Role::Opposes);
    assert_eq!(d.card_role(b), Role::Rebuts);
    assert_eq!(d.map_kind(), MapKind::Reasoning);
}
```

(`BoxNode::new(id, kind, text)` and `Card::new(id, kind, premises)` are plain constructors with empty children/extra.)

- [ ] **Step 2: Run** `cargo test -p reasoncraft_core`. Expected: compile errors / FAIL.
- [ ] **Step 3: Implement** the types and functions listed under Produces. Also add the licence files (standard MIT and Apache-2.0 texts, copyright "Patrick van Batenburg") and a README with one paragraph describing ReasonCraft.
- [ ] **Step 4: Run** `cargo test -p reasoncraft_core` → PASS. Run `cargo run -p reasoncraft` → an empty window titled ReasonCraft opens. Run `cargo clippy --workspace --all-targets -- -D warnings` → clean.
- [ ] **Step 5: Commit** `git commit -am "feat: workspace, licences and core model types"` (add new files first).

---

### Task 2: Structure rules and add operations

**Files:**
- Create: `crates/reasoncraft_core/src/rules.rs`, `crates/reasoncraft_core/src/edit.rs`
- Test: unit tests in both files

**Interfaces:**
- Consumes: Task 1 types.
- Produces:
  - `enum NewItem { Card(CardKind), Category, Source, Note }`
  - `fn can_attach(doc: &Document, parent: Parent, new: NewItem) -> bool`. It implements the spec §4 table:
    - Root claim: cards or Category, never both. The first child decides; Source and Note are always allowed.
    - Premise: Card / Source / Note.
    - `Parent::Card`: Card only.
    - Category: Category / Note.
    - Source: Note.
    - Note: nothing.
    - Parking: anything.
  - `enum EditError { NotAllowed, NotFound, LastPremiseOfRoot, Cycle }` (thiserror)
  - `impl Document`:
    - `fn add_card(&mut self, parent: Parent, kind: CardKind, premise_text: &str) -> Result<(CardId, NodeId), EditError>`
    - `fn add_premise(&mut self, card: CardId, index: Option<usize>, text: &str) -> Result<NodeId, EditError>`. A co-premise; `None` appends.
    - `fn add_box(&mut self, parent: Parent, kind: BoxKind, text: &str) -> Result<NodeId, EditError>`, for Category/Source/Note. Claim and Premise kinds return `NotAllowed`.
    - `fn set_text(&mut self, id: NodeId, text: &str) -> Result<(), EditError>`

- [ ] **Step 1: Write failing tests:**

```rust
#[test]
fn root_cannot_mix_cards_and_categories() {
    let mut d = Document::new("C");
    d.add_card(Parent::Box(d.root), CardKind::Reason, "R").unwrap();
    assert_eq!(d.add_box(Parent::Box(d.root), BoxKind::Category, "X"), Err(EditError::NotAllowed));
    assert!(d.add_box(Parent::Box(d.root), BoxKind::Note, "n").is_ok());
}

#[test]
fn category_rejects_sources_and_cards() {
    let mut d = Document::new("Topic");
    let c = d.add_box(Parent::Box(d.root), BoxKind::Category, "Industry").unwrap();
    assert!(!can_attach(&d, Parent::Box(c), NewItem::Source));
    assert!(!can_attach(&d, Parent::Box(c), NewItem::Card(CardKind::Reason)));
    assert!(can_attach(&d, Parent::Box(c), NewItem::Category));
    assert_eq!(d.map_kind(), MapKind::Grouping);
}

#[test]
fn card_targets_premise_or_whole_card() {
    let mut d = Document::new("C");
    let (card, p) = d.add_card(Parent::Box(d.root), CardKind::Reason, "P1").unwrap();
    d.add_premise(card, None, "P2").unwrap();
    let (on_premise, _) = d.add_card(Parent::Box(p), CardKind::Objection, "against P1").unwrap();
    let (on_card, _) = d.add_card(Parent::Card(card), CardKind::Objection, "link fails").unwrap();
    assert_eq!(d.owner_of(Item::Card(on_premise)), Some(Parent::Box(p)));
    assert_eq!(d.owner_of(Item::Card(on_card)), Some(Parent::Card(card)));
    assert_eq!(d.cards[&card].premises.len(), 2);
    assert!(d.add_box(Parent::Card(card), BoxKind::Source, "s").is_err());
}
```

- [ ] **Step 2: Run** `cargo test -p reasoncraft_core` → FAIL.
- [ ] **Step 3: Implement** `can_attach` and the four operations. New items get fresh ids, and the operations update `meta.modified`.
- [ ] **Step 4: Run** → PASS.
- [ ] **Step 5: Commit** `feat(core): structure rules and add operations`.

---

### Task 3: Move, connect, parking area and removal

**Files:**
- Modify: `crates/reasoncraft_core/src/edit.rs`
- Test: unit tests in `edit.rs`

**Interfaces:**
- Consumes: Tasks 1–2.
- Produces (on `Document`):
  - `fn move_item(&mut self, item: Item, to: Parent, index: Option<usize>) -> Result<(), EditError>`. It covers connecting a parking item, detaching to parking (`to = Parent::Parking`) and reordering among siblings. It checks `can_attach` for the moved item's kind. Moving a premise box means moving it between cards (co-premise to another card) via `Parent::Card`; any other premise move is `NotAllowed`. Moving an item into its own descendant → `Cycle`. The root can't move → `NotAllowed`.
  - `fn add_loose(&mut self, kind: NewLoose, text: &str) -> Item`, where `enum NewLoose { Card(CardKind), Category, Source, Note }`. A loose card gets one premise.
  - `enum RemoveMode { Branch, KeepChildren }`
  - `fn remove_item(&mut self, item: Item, mode: RemoveMode) -> Result<(), EditError>`:
    - Branch deletes the item and everything under it.
    - KeepChildren moves its direct children (cards, boxes and, for a card, its premises' children) to the parking area.
    - Removing the last premise removes its card (same mode).
    - Removing the root → `NotAllowed`.
  - `fn descendants(&self, item: Item) -> Vec<Item>` (iterative, not recursive)

- [ ] **Step 1: Write failing tests** `remove_last_premise_removes_card`, `remove_branch_leaves_no_orphans`, `remove_keep_children_moves_to_parking`, `connect_parking_card_to_premise`, `move_into_descendant_is_cycle`, `detach_to_parking_and_reorder`. Key assertions:

```rust
#[test]
fn remove_branch_leaves_no_orphans() {
    let mut d = Document::new("C");
    let (card, p) = d.add_card(Parent::Box(d.root), CardKind::Reason, "P").unwrap();
    d.add_box(Parent::Box(p), BoxKind::Source, "s").unwrap();
    d.add_card(Parent::Card(card), CardKind::Objection, "x").unwrap();
    d.remove_item(Item::Card(card), RemoveMode::Branch).unwrap();
    assert_eq!(d.boxes.len(), 1);          // only root
    assert!(d.cards.is_empty());
    assert!(d.boxes[&d.root].children.is_empty());
}

#[test]
fn remove_keep_children_moves_to_parking() {
    let mut d = Document::new("C");
    let (card, p) = d.add_card(Parent::Box(d.root), CardKind::Reason, "P").unwrap();
    let (sub, _) = d.add_card(Parent::Box(p), CardKind::Reason, "Q").unwrap();
    d.remove_item(Item::Card(card), RemoveMode::KeepChildren).unwrap();
    assert_eq!(d.parking, vec![Item::Card(sub)]);
    assert!(!d.cards.contains_key(&card));
}
```

- [ ] **Step 2: Run** → FAIL.
- [ ] **Step 3: Implement.** Removal collects `descendants`, deletes those ids from `boxes`/`cards`, and removes the item from its owner's list. A debug-only `fn check_invariants(&self)` asserts that every id referenced is present and owned exactly once; call it at the end of every edit op in `#[cfg(debug_assertions)]`.
- [ ] **Step 4: Run** → PASS.
- [ ] **Step 5: Commit** `feat(core): parking area, move/connect and removal`.

---

### Task 4: Undo/redo history

**Files:**
- Create: `crates/reasoncraft_core/src/history.rs`
- Test: unit tests in `history.rs`

**Interfaces:**
- Consumes: `Document` (`Clone`).
- Produces: `struct History { undo: Vec<Document>, redo: Vec<Document>, limit: usize, saved_marker: Option<usize> }`, with:
  - `History::new(limit: usize)` (the app uses 500)
  - `fn record(&mut self, before: &Document)`: call before applying an edit; clears redo.
  - `fn undo(&mut self, current: &mut Document) -> bool` and `fn redo(&mut self, current: &mut Document) -> bool`
  - `fn mark_saved(&mut self)` and `fn is_dirty(&self) -> bool`. Dirty means the current position differs from the saved position; a fresh history with no edits is clean.

Snapshot undo was chosen over command inversion because documents are small (hundreds of boxes) and snapshots can't get out of sync with the rules.

- [ ] **Step 1: Write failing tests** `undo_restores_parking`, `new_edit_clears_redo`, `limit_drops_oldest`, `dirty_tracks_saved_position`. For example:

```rust
#[test]
fn dirty_tracks_saved_position() {
    let mut d = Document::new("C"); let mut h = History::new(500);
    assert!(!h.is_dirty());
    h.record(&d); d.set_text(d.root, "C2").unwrap();
    assert!(h.is_dirty());
    h.mark_saved(); assert!(!h.is_dirty());
    h.undo(&mut d); assert!(h.is_dirty());
    h.redo(&mut d); assert!(!h.is_dirty());
}
```

- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** **Step 4: Run** → PASS.
- [ ] **Step 5: Commit** `feat(core): snapshot undo/redo history`.

---

### Task 5: `.rcraft` file format

**Files:**
- Create: `crates/reasoncraft_core/src/file.rs`
- Test: unit tests in `file.rs` (use `tempfile` as a dev-dependency)

**Interfaces:**
- Consumes: `Document`, `SCHEMA_VERSION`.
- Produces:
  - `enum FileError { Io(std::io::Error), NotAReasonCraftFile, MissingDocument, Malformed(String), NewerVersion { found: u32, supported: u32 } }` (thiserror, `Display` in plain English; the app maps these to i18n keys later)
  - `fn save(doc: &Document, path: &Path) -> Result<(), FileError>`. It writes a zip with `document.json` (pretty JSON) and an empty `images/` dir entry, first to `path.with_extension("rcraft.tmp")` in the same directory, then atomically renames it over `path`. On error the temp file is removed and the original is untouched.
  - `fn open(path: &Path) -> Result<Document, FileError>`. It rejects non-zip files (`NotAReasonCraftFile`), a missing `document.json`, bad JSON (`Malformed`) and `schema_version > SCHEMA_VERSION` (`NewerVersion`). Unknown fields survive a round trip through the `extra` maps.

- [ ] **Step 1: Write failing tests:**
  - `roundtrip_preserves_structure_and_unicode`: a map with co-premises, a whole-card attack, parking items, text `"Één — 日本 😀"`; assert `open(save(d)) == d`. `Document` derives `PartialEq`.
  - `unknown_fields_survive`: write JSON with `"future_field": 1` at document and box level by hand, open, save, re-read the raw JSON and assert the field is still present.
  - `open_rejects_non_zip`: a plain text file → `NotAReasonCraftFile`.
  - `open_rejects_newer_schema`: `schema_version: 2` → `NewerVersion { found: 2, supported: 1 }`.
  - `save_is_atomic_on_failure`: save a valid file, then save into a path whose temp target is blocked by a directory with the same name as the temp file → `Err`, and the original still opens with its original content.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement** with the `zip` crate (deflate). **Step 4: Run** → PASS.
- [ ] **Step 5: Commit** `feat(core): .rcraft file format with atomic save`.

---

### Task 6: Automatic layout

**Files:**
- Create: `crates/reasoncraft_core/src/layout/{mod.rs,tree.rs,ports.rs}`, `crates/reasoncraft_core/src/fixtures.rs` (compiled only with feature `test-fixtures`; core's `[dev-dependencies]` lists itself, `reasoncraft_core = { path = ".", features = ["test-fixtures"] }`, so its integration tests can use it, and the app enables the feature in its dev-dependencies)
- Test: `crates/reasoncraft_core/tests/layout.rs`

**Interfaces:**
- Consumes: `Document`, `Item`, `owner_of`, `card_role`.
- Produces:
  - `trait Measure { fn text_height(&self, text: &str, width: f32, kind: &BoxKind) -> f32; }`
  - `struct LayoutParams`, with `Default` values:
    - `box_width: 156.0`, `source_width: 128.0`, `min_box_height: 40.0`
    - `card_pad: 8.0`, `card_label_height: 24.0`, `premise_gap: 8.0`
    - `h_gap: 18.0`, `v_gap: 70.0`
    - `port_spacing: 16.0`, `port_margin: 15.0`, `parking_gap: 120.0`
  - `enum ConnectorKind { Supports, Opposes, Rebuts, Neutral, Attachment }`. Neutral is grouping lines; Attachment is source/note lines.
  - `enum TargetKind { Box, WholeCard }`
  - `struct Connector { child: Item, from: Pos2, to: Pos2, kind: ConnectorKind, target: TargetKind }`. `from` is the child's top centre; `to` is the parent port.
  - `struct Layout { boxes: IndexMap<NodeId, Rect>, cards: IndexMap<CardId, Rect>, card_labels: IndexMap<CardId, Pos2>, connectors: Vec<Connector>, main_bounds: Rect, parking_bounds: Option<Rect>, bounds: Rect }`. `cards` holds frame rects; `card_labels` holds the top-right anchor inside each frame.
  - `fn layout(doc: &Document, m: &dyn Measure, p: &LayoutParams) -> Layout` (top-down only in step 1)
  - Test helpers in `reasoncraft_core::fixtures`: `struct FixedMeasure`, giving 14 px per line at 7 px per char with word wrap, and breaking words longer than the width. Also `fn sample_iot_like(n_boxes: usize) -> Document`: a deterministic mix of reasons, objections, rebuttals, co-premises, whole-card attacks and sources.

**Algorithm** (the tests don't determine it):
- **Units.** Each layout unit is a card, a box outside cards, or the root.
  - Card frame width = `n·box_width + (n−1)·premise_gap + 2·card_pad`.
  - Card frame height = `card_label_height + max(premise heights) + card_pad + 2`.
  - Box height = `max(min_box_height, text_height + 12)`.
  - Source width = `source_width`, height = `text_height + 30` (icon and type line).
- **Subtree width.** A unit's subtree width is `max(own width, Σ child subtree widths + gaps)`. A card's children are, in order: for each premise, that premise's children, with the card's whole-card children inserted at the middle premise index (`ceil(n/2)`), so they sit centred under the card. Compute post-order with an explicit stack, not recursion, so 200-deep chains are safe.
- **Placement.** Place children centred under their parent, at `parent.bottom + v_gap`. Sources and notes under a premise are ordinary children; no separate rows are needed, because they are narrow and come first in `children` order. Sort each premise's children so attachments (sources, notes) come before cards.
- **Ports.** Group connectors by attach point:
  - `(box, premise index)` for a box or premise target;
  - `(card, whole)` for a whole-card target.

  A box attach point is the box's bottom centre; a whole-card attach point is the frame's bottom centre. Spread a group of n ports at `port_spacing`, clamped to the parent width minus `2·port_margin`, ordered by child centre x.
- **Parking area.** Lay out each parking item as its own mini-tree and stack them vertically with `v_gap` between, starting at `main_bounds.right + parking_gap`.

- [ ] **Step 1: Write failing tests** in `tests/layout.rs`:
  - `boxes_never_overlap`: for `sample_iot_like(117)`, no two rects in `boxes ∪ cards` intersect, except a premise inside its own card frame.
  - `long_unbroken_word_grows_height`: a premise with an 80-char word gets height > `min_box_height` and width == `box_width`.
  - `whole_card_target_attaches_at_frame_bottom`: a connector with `TargetKind::WholeCard` has `to.y == cards[&card].bottom()` and `to.x` within the frame.
  - `shared_ports_are_spread_and_ordered`: root with 5 cards → 5 connectors with distinct `to.x`, 16 px apart, increasing with child x.
  - `label_anchor_inside_frame`: every `card_labels[c]` lies inside `cards[c]` within the top `card_label_height`.
  - `parking_is_right_of_main`: `parking_bounds.left() >= main_bounds.right() + 120.0`.
  - `deep_chain_no_overlap`: a chain of 200 reason cards lays out without panic, and every child's top is below its parent's bottom.
  - `layout_500_boxes_fast` (`#[ignore]`): `sample_iot_like(500)` lays out in < 50 ms.
- [ ] **Step 2: Run** `cargo test -p reasoncraft_core --test layout` → FAIL.
- [ ] **Step 3: Implement** `layout()` per the algorithm.
- [ ] **Step 4: Run** → PASS. Then run `cargo test -p reasoncraft_core --release --test layout -- --ignored` → PASS.
- [ ] **Step 5: Commit** `feat(core): automatic tidy-tree layout with ports and parking area`.

---

### Task 7: Connector geometry

**Files:**
- Create: `crates/reasoncraft_core/src/connector.rs`
- Test: unit tests in `connector.rs`

**Interfaces:**
- Consumes: `Connector`, `ConnectorKind`.
- Produces:
  - `enum Segment { Cubic([Pos2; 4]), Line([Pos2; 2]) }`
  - `fn curved_path(from: Pos2, to: Pos2, stub: f32) -> Vec<Segment>`. `from` is the child top (lower on screen) and `to` the parent port (higher). It returns a cubic from `from` to `(to.x, to.y + stub)`, with control points `(from.x, mid)` and `(to.x, mid)` where `mid` is the y halfway between the cubic's endpoints, then a `Line` up to `to`. The app uses `stub = 16.0`.
  - `fn end_marker(kind: ConnectorKind) -> EndMarker`, where `enum EndMarker { Arrow, Bar, None }`: Supports → Arrow; Opposes/Rebuts → Bar; Neutral/Attachment → None.
  - `enum Strength { Strong, Normal, Weak }` (only Normal is used in step 1; evaluation arrives in step 2)
  - `fn stroke_px(kind: ConnectorKind, strength: Strength, zoom: f32) -> f32`, the on-screen width:
    - base widths at 100%: Strong 3.0, Normal 2.0, Weak 1.4; Neutral 1.5; Attachment 1.0;
    - multiply by `zoom`;
    - clamp to at least 1.25 (Attachment at least 1.0).
  - `pub const ARROW_PX: f32 = 9.0;`. The arrowhead size is in screen px, independent of stroke and zoom.

- [ ] **Step 1: Write failing tests:**
  - `curved_path_ends_with_vertical_stub`: the last segment is a `Line` from `(to.x, to.y+16)` to `to`.
  - `stroke_never_below_minimum`: `stroke_px(Supports, Weak, 0.1) >= 1.25`.
  - `strong_thicker_than_weak` at zoom 1.0.
  - `markers_by_kind`: the three cases listed under `end_marker`.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** **Step 4: Run** → PASS.
- [ ] **Step 5: Commit** `feat(core): curved connector geometry and stroke rules`.

---

### Task 8: App shell — theme, i18n, title bar, tabs, ribbon, status bar

**Files:**
- Create: `app/reasoncraft/src/{app.rs,doc_tab.rs,actions.rs,i18n.rs,theme.rs}`, `app/reasoncraft/src/shell/{mod,titlebar,tabs,ribbon,statusbar}.rs`, `app/reasoncraft/i18n/en.ftl`, `app/reasoncraft/assets/icon.png`, `app/reasoncraft/tests/ui.rs`
- Modify: `app/reasoncraft/src/main.rs`

**Interfaces:**
- Consumes: core `Document`, `History`.
- Produces:
  - `fn t(key: &str) -> String`, which looks up the embedded `en.ftl` via fluent-bundle. A missing key returns the key itself and logs a warning in debug builds.
  - `enum ThemeChoice { System, Light, Dark }` and `struct Tokens { bg, panel, ribbon, canvas, canvas_dot, box_fill, box_text, claim_border, text_muted, accent, reason, objection, rebuttal, attachment_line: Color32 }`, plus `Tokens::dark()`, `Tokens::light()`, `fn resolve(choice: ThemeChoice, os_dark: bool) -> Tokens` and `fn apply(ctx: &egui::Context, t: &Tokens)`.
    - Dark values: bg `#141416`, panel `#202024`, ribbon `#232327`, canvas `#161618`, box_fill `#26262B`, accent `#8AB4FF`, plus the role colours from Global Constraints.
    - Light values are the builder's call, as long as text meets WCAG AA contrast.
  - `struct DocTab { doc: Document, history: History, path: Option<PathBuf>, selection: Option<Item>, editing: Option<NodeId>, camera: Camera }`, with `fn title(&self) -> String`: the file stem, or `t("untitled")` followed by `•` when dirty.
  - `enum Action`:
    - `New`, `Open`, `Save`, `SaveAs`, `CloseTab(usize)`, `Undo`, `Redo`
    - `AddReason`, `AddObjection`, `AddCoPremise`, `AddCategory`, `AddNote`, `AddLoose(NewLoose)`
    - `TargetWholeCard(CardKind)`
    - `Delete`, `DeleteBranch`, `Edit`, `StopEditing`
    - `Select(Option<Item>)`, `Navigate(Dir)`
    - `ZoomIn`, `ZoomOut`, `ZoomFit`, `Zoom100`
    - `SetTheme(ThemeChoice)`, `TogglePanel(Panel)`

    Supporting enums: `enum Dir { Up, Down, Left, Right }` and `enum Panel { Left, Right }`.
  - `fn apply(app: &mut ReasonCraftApp, action: Action)`: the single place where actions mutate state. Every document edit calls `history.record` first.
  - `struct ReasonCraftApp { tabs: Vec<DocTab>, active: usize, theme: ThemeChoice, left_panel: bool, right_panel: bool, pending: Vec<Action> }`. The theme is persisted with eframe storage under key `"settings"`.
- **UI:**
  - **Title bar** (custom, as in `artcraft-native`): `with_decorations(false)`, and on macOS `fullsize_content_view` with the title bar hidden. It contains the quick access buttons (Save, Undo, Redo), the document title in the centre, a "Search commands" box (disabled placeholder in step 1) and window buttons (minimise, maximise, close) on Windows/Linux. Dragging the empty title bar moves the window.
  - **Tabs row:** document tabs plus a "+" button.
  - **Ribbon:** tab strip File · Home · Insert · Evaluate · View · Export · Help. In step 1:
    - File opens a simple backstage with New/Open/Save/Save As.
    - Home has the groups Clipboard (disabled placeholders), Add (Reason, Objection, Co-premise, Category, Note), Editing (Delete, Delete branch, Undo, Redo).
    - View has Zoom (Fit, 100%, In, Out), Theme (System/Light/Dark) and Panels (left, right).
    - The other tabs show the text "Coming soon" from `t("ribbon.coming_soon")`.
    - Buttons have an icon plus a label, styled after the WordCraft screenshot. Icons are drawn with simple vector shapes or bundled original SVGs.
  - **Status bar:** box count, a theme toggle, a zoom slider with percentage.
  - **Side panels:** left (Building panel) and right (Text panel) are collapsible empty placeholders with headings in step 1. Their content arrives in steps 2–3.

- [ ] **Step 1: Write failing kittest tests** in `tests/ui.rs`:
  - `ribbon_add_reason_adds_card`: build the app with a new document, click the "Reason" ribbon button (via `harness.get_by_label(t("action.add_reason"))`) with the root selected → the document has 1 card.
  - `theme_switch_applies_dark_visuals`: click View → Dark → `ctx.style().visuals.dark_mode == true`.
  - `new_tab_button_opens_second_document`.
  - `missing_i18n_key_returns_key`: `t("no.such.key") == "no.such.key"`.

  Expose `ReasonCraftApp::new_for_test()` (no storage) so tests can build the app.
- [ ] **Step 2: Run** `cargo test -p reasoncraft` → FAIL.
- [ ] **Step 3: Implement** the shell per the UI description above.
- [ ] **Step 4: Run** → PASS, and `cargo run -p reasoncraft` shows the shell in dark mode.
- [ ] **Step 5: Commit** `feat(app): shell with theme, i18n, title bar, tabs, ribbon and status bar`.

---

### Task 9: Canvas — camera, measurement, style-D painting, hit testing

**Files:**
- Create: `app/reasoncraft/src/canvas/{mod,camera,measure,paint,hit}.rs`
- Modify: `app/reasoncraft/src/app.rs` (central panel hosts the canvas)
- Test: unit tests in `camera.rs`, `measure.rs`, `hit.rs`; kittest smoke test in `tests/ui.rs`

**Interfaces:**
- Consumes: `layout()`, `Layout`, `curved_path`, `stroke_px`, `end_marker`, `ARROW_PX`, `Tokens`.
- Produces:
  - `struct Camera { offset: Vec2, zoom: f32 }`, with zoom clamped to 0.1..=4.0. Methods: `world_to_screen(Pos2) -> Pos2`, `screen_to_world(Pos2) -> Pos2`, `zoom_at(screen_pt: Pos2, factor: f32)` (keeps the world point under the cursor fixed), and `fit(bounds: Rect, viewport: Rect)` (10% margin).
  - `struct EguiMeasure<'a> { ctx: &'a egui::Context }`, implementing core `Measure`. It uses `ctx.fonts(|f| f.layout(text, FontId::proportional(13.0), color, width - 16.0))` with `break_anywhere` enabled for overlong words, and returns the galley height. Results are cached in a `HashMap<(String, u32 width), f32>` that is cleared when fonts change.
  - `fn hit_test(layout: &Layout, world: Pos2) -> Option<Hit>`, where `enum Hit { Box(NodeId), CardFrame(CardId), CardBottomEdge(CardId) }`. It prefers the box, then the bottom-edge band (8 px), then the frame.
  - `fn paint(painter: &Painter, layout: &Layout, doc: &Document, cam: &Camera, tokens: &Tokens, selection: Option<Item>)`. Style D per spec §5:
    - Card frame: rounded 10 px, fill = role colour at 12% alpha, stroke = role colour at 55%.
    - Premise: rounded 7 px, `box_fill` with a 1 px border in the role colour.
    - Claim: `claim_border` at 1 px.
    - Category: neutral border.
    - Source: no border; icon placeholder plus bold type name plus small centred text.
    - Note: yellow sticky.
    - Role label inside the frame at its anchor, right-aligned, 10.5 px: "supports" / "opposes" / "rebuts" in the role colour. Strength dots arrive in step 2.
    - Connectors drawn with `curved_path` as an egui cubic Bézier plus a line segment. Stroke from `stroke_px`. Arrowhead: a filled triangle `ARROW_PX` long, pointing along the stub. Bar: an 11 px horizontal line, 2.4 px thick.
    - The selection draws a 2 px accent outline, offset 2 px.
    - The parking area is drawn as a dashed rounded rect with the heading `t("parking.title")` ("Not connected").
    - The canvas background is a dot grid.

  Draw order: connectors, then frames, then boxes, then labels.
- [ ] **Step 1: Write failing tests:**
  - `zoom_at_keeps_point_fixed`
  - `fit_contains_bounds`
  - `egui_measure_breaks_long_words`: an 80-char word at width 156 gives a height greater than one line.
  - `hit_prefers_box_over_frame`
  - `hit_bottom_edge_band`
  - kittest `canvas_renders_117_box_fixture_without_panic`: load `sample_iot_like(117)` into a tab and run 3 frames.

  The app's `[dev-dependencies]` enables core's `test-fixtures` feature.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** Pan with middle/right-drag or space+drag; zoom with Ctrl+wheel and pinch; plain wheel scrolls vertically and Shift+wheel horizontally. **Step 4: Run** → PASS.
- [ ] **Step 5: Manual check:** `cargo run -p reasoncraft`, use View → Zoom fit on the fixture (temporary debug menu item, debug builds only). The map should look like the approved style-D mockup: no lines through boxes, labels inside frames, spread ports.
- [ ] **Step 6: Commit** `feat(app): canvas with style-D rendering, camera and hit testing`.

---

### Task 10: Editing interactions — selection, inline text, keyboard, "+" menu, drag to connect

**Files:**
- Create: `app/reasoncraft/src/canvas/{input.rs,plus_menu.rs}`
- Modify: `app/reasoncraft/src/actions.rs`, `app/reasoncraft/src/canvas/mod.rs`
- Test: `app/reasoncraft/tests/ui.rs`, unit tests in `input.rs`

**Interfaces:**
- Consumes: `Action`, `apply`, `hit_test`, core edit ops.
- Produces:
  - `fn key_to_action(key: egui::Key, mods: egui::Modifiers, editing: bool) -> Option<Action>`, a pure function. Bindings:
    - Tab → AddReason; Shift+Tab → AddObjection; Ctrl+Enter → AddCoPremise
    - Enter / F2 → Edit; Esc → StopEditing
    - Delete → Delete; Shift+Delete → DeleteBranch
    - Arrow keys → Navigate (Up = owner, Down = first child, Left/Right = previous/next sibling or co-premise)
    - Ctrl+Z → Undo; Ctrl+Y and Ctrl+Shift+Z → Redo
    - Ctrl+S → Save; Ctrl+O → Open; Ctrl+N → New
    - Ctrl+0 → Zoom100; Ctrl+1 → ZoomFit; Ctrl+= → ZoomIn; Ctrl+- → ZoomOut

    While editing, only Esc, Ctrl+Enter (finish and add a co-premise) and Tab (finish and add a reason) are intercepted. Cmd replaces Ctrl on macOS via `Modifiers::command`.
  - `fn navigate(doc: &Document, from: Item, dir: Dir) -> Option<Item>` (pure; in the app crate)
  - Add actions target the selection:
    - A selected premise or root box: a new card targets that box.
    - A selected card frame: a new card targets the whole card.
    - Nothing selected: the item goes loose into the parking area.

    After an add, the new premise is selected and in edit mode.
  - **Inline editing:** an `egui::TextEdit::multiline` overlaid on the box rect in screen space, with font size scaled by zoom. Text is committed to core through `set_text` on Esc or click-away, recorded as one history entry per edit session.
  - **Typing on a selected box:** a printable key starts editing with that character appended.
  - **"+" menu:** a small round button at the selected box's bottom-right. Its menu offers Reason, Objection, Co-premise (premises only), Category (grouping only), Note, and, for premises, "Target whole card → Reason/Objection". Items not allowed by `can_attach` are disabled.
  - **Drag:** dragging a box or card and dropping it on a box or card bottom edge calls `move_item` (connect or reattach); dropping on empty canvas detaches it to parking. A ghost outline follows the cursor, and the drop target highlights green if allowed, red if not.

- [ ] **Step 1: Write failing tests:**
  - Unit tests: `tab_adds_reason`, `typing_mode_only_intercepts_esc_tab_ctrl_enter`, `navigate_up_down_siblings`.
  - Kittest `keyboard_builds_map`: press Tab, type "Reason one", press Esc, press Shift+Tab, type "Objection", press Esc, press Ctrl+Z → one card left, with text "Reason one".
  - Kittest `whole_card_target_via_plus_menu`.
  - Kittest `drop_on_empty_canvas_detaches_to_parking`.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** **Step 4: Run** → PASS.
- [ ] **Step 5: Commit** `feat(app): selection, inline editing, keyboard control, plus menu and drag-to-connect`.

---

### Task 11: File operations in the app

**Files:**
- Create: `app/reasoncraft/src/file_ops.rs`
- Modify: `app/reasoncraft/src/actions.rs`, `app/reasoncraft/src/app.rs`, `app/reasoncraft/i18n/en.ftl`
- Test: `app/reasoncraft/tests/ui.rs`

**Interfaces:**
- Consumes: core `file::save` / `file::open` / `FileError`, `History::mark_saved` / `is_dirty`.
- Produces:
  - `trait FileDialogs { fn pick_open(&self) -> Option<PathBuf>; fn pick_save(&self, suggested: &str) -> Option<PathBuf>; }`, implemented with rfd (filter `ReasonCraft map (*.rcraft)`). A `FakeDialogs` implementation is used in tests.
  - `fn open_into_app(app: &mut ReasonCraftApp, path: &Path)`. On success it adds a tab, or reuses the active tab if that tab is untouched and untitled. On error it shows a modal message `t("error.open.<variant>")` and changes no tabs.
  - `fn save_tab(tab: &mut DocTab, dialogs: &dyn FileDialogs, save_as: bool) -> Result<(), FileError>`. It appends `.rcraft` if missing, then calls `mark_saved`.
  - Close tab / quit with unsaved changes → modal with Save / Don't save / Cancel. Hook `eframe::App::on_exit` and the viewport close request.
  - The title shows the file stem and `•` when dirty.
  - Window title: `"<doc> — ReasonCraft"`.
  - Open file paths passed as command-line arguments on start, which covers double-clicking `.rcraft` files.

- [ ] **Step 1: Write failing kittest tests** `save_then_open_roundtrip` (tempdir + FakeDialogs), `failed_open_keeps_tabs` (a garbage file → tab count unchanged, error modal visible), `undo_after_save_marks_dirty`, `close_dirty_tab_asks_to_save`.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** **Step 4: Run** → PASS.
- [ ] **Step 5: Commit** `feat(app): new, open, save, save as and unsaved-changes prompts`.

---

### Task 12: End-to-end verification and docs

**Files:**
- Modify: `README.md` (build and run instructions, keyboard shortcuts table), `docs/todo.md` (review section)
- Create: `app/reasoncraft/tests/e2e.rs`

- [ ] **Step 1: Write the test** `e2e_build_save_reopen`. Through actions only:
  - build a reasoning map with 2 reasons (one with 2 co-premises), an objection with a rebuttal, a whole-card attack, a note and a parking item;
  - save, open in a new tab;
  - assert structural equality of the two documents, and equal layout `bounds` within 0.5 px.
- [ ] **Step 2: Run** `cargo test --workspace` → PASS. Run `cargo clippy --workspace --all-targets -- -D warnings` → clean. Run `cargo build --release -p reasoncraft` → success.
- [ ] **Step 3: Manual check:** run the release build, build a small map by keyboard only, save it, reopen it, switch theme and zoom fit. Then repeat the stress fixture view. Record the results and any defects in the `docs/todo.md` review section.
- [ ] **Step 4: Commit** `docs: step 1 verification and usage notes`.

---

## Later plans (not in this plan)

Steps 2–6 of spec §13 each get their own plan when this one is complete:
- **Step 2:** sources and evaluation, notes and comments, building panel.
- **Step 3:** Outline, Snippets, Check.
- **Step 4:** exports, print, Essay.
- **Step 5:** Argdown, Elbow/Tapered connectors, layout directions, minimap, numbering, find/replace, spellcheck, autosave, Search commands.
- **Step 6:** Dutch translation, accessibility, help, examples, templates, installers.
