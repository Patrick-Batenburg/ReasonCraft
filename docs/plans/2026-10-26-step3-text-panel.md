# ReasonCraft Step 3 — Text Panel: Outline, Check, Snippets — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fill the right-hand Text panel with three working tabs:
- **Outline:** the map as an indented, numbered text outline.
- **Check:** the course-rule checks from spec §7, with clickable issues.
- **Snippets:** collected source text you can drag onto the map, which is the bottom-up workflow.

The Essay tab stays a placeholder until step 4.

**Architecture:**
- **Core (`reasoncraft_core`)** gets three pure modules:
  - `outline.rs`: builds outline lines from a `Document`;
  - `checks.rs`: runs the rule checks, with English and Dutch word lists;
  - `snippets` support in the model and edit ops.
- **App (`reasoncraft`)** gets `shell/text_panel/{mod,outline,check,snippets}.rs`, plus a document revision counter on `DocTab` so derived data (outline, issues) is cached per revision.
- All mutations still go through `actions::apply` → `DocTab::edit`.

**Tech Stack:** unchanged from step 2 (Rust 1.99.1 pinned, egui/eframe/egui_kittest 0.37, serde, indexmap, uuid, image, zip, sha2, open, fluent-bundle). No new dependencies.

**Spec:** `docs/superpowers/specs/2026-10-25-reasoncraft-design.md`. This step covers:
- §6, the Text panel: the Outline, Check and Snippets tabs;
- §7, Checks 1–8;
- §8, the bottom-up workflow;
- §4, document language in DocMeta.

It also covers the step-2 carry-over items in `docs/todo.md`.

## Global Constraints

- **Platform:** pure native egui; Windows/macOS/Linux; offline.
- **Text and colours:** all UI text via `i18n::t` / `t_args` (keys in `app/reasoncraft/i18n/en.ftl`); colours only from `theme::Tokens`; box text never clipped.
- **Mutations:** `actions::apply` is the only state mutator; document changes via `DocTab::edit` (one history entry on success, none on refusal or no-op).
- **File format:** new document fields are optional (`#[serde(default, skip_serializing_if = …)]`). `SCHEMA_VERSION` stays 1, because snippets are a new optional field that older builds keep through `extra`. Step-1 and step-2 files open unchanged.
- **Checks are hints:** they never block an action.
- **Check performance:** checks and the outline must not run every frame for unchanged documents. They are cached per `DocTab.revision`.
- **Quality gate:** `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check` and `cargo test --workspace` pass at every commit.
- **File size:** files stay ≤ ~400 lines; split by responsibility when needed.

## Review Focus

1. **Stale derived data after undo/redo or tab switch.** The outline and issues must reflect the current document immediately. → Task 1 test `revision_bumps_on_every_document_change`; Task 5 test `issues_refresh_after_undo`.
2. **Dutch and punctuation in the key-term check.** Diacritics ("één", "coördinatie"), apostrophes ("auto's"), hyphens ("e-mail"), plurals ("banen"/"baan" is a known limit), and the same word in different case must not produce false "missing term" warnings. → Task 4 tests `key_term_normalises_case_punctuation_diacritics` and `key_term_dutch_example_from_course`.
3. **Huge pasted snippets.** 100 000+ characters must not freeze the UI; cap at `MAX_SNIPPET_CHARS = 50_000` with a notice. → Task 7 test `oversized_paste_is_capped`.
4. **Large maps in the Outline.** 500 boxes render without frame drops (rows virtualised with `ScrollArea::show_rows`; build cached per revision). → Task 2 test `outline_500_boxes_fast` (ignored, release) and Task 3 kittest `outline_scrolls_large_map`.
5. **Empty or whitespace-only snippet selections dragged onto the map.** These are refused: no history, red target. A long selection becomes a box with its full text, wrapped. → Task 8 tests `blank_selection_drop_refused` and `long_selection_drop_keeps_text`.

---

## File Structure

```
crates/reasoncraft_core/src/
  outline.rs        NEW  OutlineOptions, OutlineLine, LinkLabel, build_outline(), outline_plain_text()
  checks.rs         NEW  CheckKind, CheckSet, Issue, run_checks(); tokenise(); stopwords
  checks/stopwords.rs NEW en + nl stopword lists (const &[&str])
  snippets.rs       NEW  Snippet, SnippetSource; impl Document { add_snippet, update_snippet_text, set_snippet_source, remove_snippet }
  model.rs          Document.snippets: Vec<Snippet> (serde default/skip empty)
  edit.rs           validate(): unique snippet ids
app/reasoncraft/src/
  doc_tab.rs        + revision: u64 (bumped on every doc change incl. undo/redo/open)
  settings.rs       + text_tab: TextTab, checks: CheckSet, outline: OutlineOptions
  shell/text_panel/mod.rs      NEW tab strip Outline · Essay · Check · Snippets
  shell/text_panel/outline.rs  NEW
  shell/text_panel/check.rs    NEW
  shell/text_panel/snippets.rs NEW (list + editor + source tagging)
  shell/text_panel/drag_chip.rs NEW "drag selection" chip
  actions/snippets.rs          NEW
  shell/statusbar.rs           + issue count chip → opens Check tab
  shell/backstage.rs           + Document page: language (en/nl)
  canvas/drag/panel.rs         + Insertable::Text drops
```

---

### Task 1: Carry-over fixes, revision counter, text panel tab shell

**Files:**
- Modify: `app/reasoncraft/src/shell/link_modal.rs`, `actions/connect.rs`, `doc_tab.rs`, `settings.rs`, `shell/mod.rs`
- Create: `app/reasoncraft/src/shell/text_panel/mod.rs`
- Modify: `en.ftl`
- Test: unit tests and a kittest

**Interfaces:**
- Produces:
  - **mailto fix.** `safe_link` no longer truncates at `#` when scanning mailto parameters. It refuses any mailto whose decoded parameter names contain a control character or a `%`, and still refuses `attach`/`attachment` (case-insensitive).
  - **Connect-mode keep-list.** It now also keeps `SetTheme`, `TogglePanel`, `ToggleMarkers` and `ToggleShading`.
  - **`DocTab.revision: u64`.** Bumped by `DocTab::edit` on success, by undo/redo when they change something, by `commit_edit` when it records, by file open, and by any snippet edit session commit. Expose `fn revision(&self) -> u64`.
  - **`TextTab`.** `#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)] pub enum TextTab { #[default] Outline, Essay, Check, Snippets }`, stored as `Settings.text_tab` (persisted, `#[serde(default)]`).
  - **Tab strip.** The right panel shows `t("text.tab.outline|essay|check|snippets")` with the active tab underlined (same style as the ribbon tabs). Each body is a placeholder in this task, except Essay, which shows `t("text.essay.coming")` permanently in this step.
  - **Action.** `Action::SetTextTab(TextTab)`.
- [ ] **Step 1: Write failing tests.**
  - `mailto_fragment_and_encoded_names_refused`: `mailto:x@y.z?subject=a#&attach=/etc/passwd` and `mailto:x@y.z?attach%00=/x` → refused; `mailto:x@y.z?subject=hi` → accepted.
  - `connect_mode_survives_theme_and_panel_toggles`.
  - `revision_bumps_on_every_document_change`: edit, refused edit (no bump), undo, redo, open.
  - Kittest `text_panel_tabs_switch_and_persist`.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** **Step 4: Run** → PASS.
- [ ] **Step 5: Commit** `feat(app): text panel tabs, document revision counter, link and connect-mode fixes`.

---

### Task 2: Core outline builder

**Files:**
- Create: `crates/reasoncraft_core/src/outline.rs`
- Modify: `crates/reasoncraft_core/src/lib.rs`
- Test: unit tests and `crates/reasoncraft_core/tests/outline.rs`

**Interfaces:**
- Consumes: `Document`, `owner_index`, roles (same rule as `card_role`), ratings, notes, comments.
- Produces:
  - **`LinkLabel`.** `pub enum LinkLabel { Because, But, However, None }`: Reason → Because, Objection → But, Rebuttal → However; categories, sources and notes → None.
  - **`OutlineLine`.** `pub struct OutlineLine { pub item: Item, pub depth: usize, pub number: String, pub label: LinkLabel, pub text: String, pub kind: LineKind, pub whole_card: bool, pub parked: bool }`.
  - **`LineKind`.** `pub enum LineKind { Claim, Premise { card: CardId, role: Role, strength: Option<CardStrength> }, Category, Source(SourceType), Note }`.
  - **`OutlineOptions`.** `pub struct OutlineOptions { pub numbering: bool, pub labels: bool, pub notes: bool }`. Defaults true/true/false. Serde, so settings can persist it.
  - **`build_outline`.** `pub fn build_outline(doc: &Document, opts: &OutlineOptions) -> Vec<OutlineLine>`. Pre-order over the main tree, then parked items (`parked = true`, numbered `P1`, `P2`, …).
    - **Numbering.** The claim has an empty number. First-level children are numbered `1`, `2`, …; nested ones `1.1`, `1.2`, …. Co-premises of one card share the card's number with letter suffixes: `1a`, `1b`. Their children continue as `1a.1`, and so on.
    - **Whole-card children** follow the card's premises and are numbered `1.w1`, `1.w2` with `whole_card = true`.
    - **Ordering.** Sources come directly under their premise, in document order. Notes are included only if `opts.notes`.
    - **Iterative.** No recursion; a 200-deep chain must work.
  - **`outline_plain_text`.** `pub fn outline_plain_text(lines: &[OutlineLine], opts: &OutlineOptions, label_word: &dyn Fn(LinkLabel) -> String, source_word: &dyn Fn(&SourceType) -> String) -> String`. Two spaces of indentation per depth. Each line reads `"{number} {label_word} {text}"`, leaving out empty parts. Source lines read `"[{source_word}] {text}"`. The words are passed in so core stays free of i18n.
- [ ] **Step 1: Write failing tests.**
  - `numbering_and_labels`: a claim with 2 reasons (the first with 2 co-premises), an objection with a rebuttal and a whole-card attack → the exact `number` and `label` sequence: `""`, `"1a"` Because, `"1b"` Because, `"1.w1"` But, `"2"` Because, `"3"` But, `"3.1"` However.
  - `sources_under_premise_and_parked_section`.
  - `notes_only_when_enabled`.
  - `plain_text_matches_expected`: an exact multi-line string.
  - `deep_chain_200_no_overflow`.
  - `outline_500_boxes_fast` (`#[ignore]`): < 5 ms in release.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** **Step 4: Run** → PASS (plus the ignored test in release).
- [ ] **Step 5: Commit** `feat(core): outline builder with numbering, labels and plain text`.

---

### Task 3: Outline tab

**Files:**
- Create: `app/reasoncraft/src/shell/text_panel/outline.rs`
- Modify: `shell/text_panel/mod.rs`, `settings.rs`, `actions.rs` (or `actions/view.rs`), `en.ftl`
- Test: kittests

**Interfaces:**
- Consumes: `build_outline`, `outline_plain_text`, `DocTab.revision`.
- Produces:
  - **Toolbar** with toggles for Numbering, Labels, Indent, Colour and Notes. Numbering, Labels and Notes map to `OutlineOptions`. `Settings.outline_indent: bool` (default true) and `Settings.outline_colour: bool` (default true) are view-only. Toggles are applied through `Action::SetOutlineOption(OutlineToggle, bool)`, where `OutlineToggle { Numbering, Labels, Indent, Colour, Notes }`. All of this is persisted.
  - **Rows.** One per line, inside `ScrollArea::vertical().show_rows(...)` (virtualised).
    - Indent is 14 px per depth when enabled.
    - The number is muted and the label word comes from `t("outline.label.because|but|however")`.
    - With Colour on, a 3 px left bar shows the role colour (Tokens), greyed for Nil cards.
    - Sources show their icon (`paint_source_icon`, 12 px) and the type name.
    - Ratings appear as a trailing small marker (same glyphs as the canvas, 10 px).
  - **Selection.** Clicking a row → `Action::Select(Some(line.item))`, followed by a canvas reveal. The selected item's row is highlighted with `selected_fill`.
  - **Copy all.** "Copy all" puts `outline_plain_text(...)` on the clipboard via `ctx.copy_text`, then shows a toast/status message `t("outline.copied")` for 2 s.
  - **Cache.** The outline is cached in `DocTab` as `outline_cache: Option<(u64 /*revision*/, OutlineOptions, Vec<OutlineLine>)>`.
- [ ] **Step 1: Write failing kittests.**
  - `outline_click_selects_box`.
  - `outline_toggles_persist`.
  - `copy_all_puts_plain_text_on_clipboard`: read `harness.ctx` output commands or a `Clipboard` fake.
  - `outline_scrolls_large_map`: `sample_iot_like(500)`; 3 frames; frame time recorded and < 50 ms in release (`#[ignore]`).
  - `outline_refreshes_after_edit` (revision cache).
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** **Step 4: Run** → PASS. Screenshot the panel with a rated sample map and look at it.
- [ ] **Step 5: Commit** `feat(app): outline tab with numbering, labels, colours and copy`.

---

### Task 4: Core checks

**Files:**
- Create: `crates/reasoncraft_core/src/checks.rs`, `crates/reasoncraft_core/src/checks/stopwords.rs`
- Modify: `lib.rs`
- Test: unit tests and `crates/reasoncraft_core/tests/checks.rs`

**Interfaces:**
- Consumes: `Document`, `owner_index`, `DocMeta.language`.
- Produces:
  - **`CheckKind`.** `pub enum CheckKind { KeyTerm, Compound, Question, Unsupported, EmptyCategory, IncompleteEvaluation, InconsistentEvaluation, NotConnected }`, with `CheckKind::ALL: [CheckKind; 8]`.
  - **`CheckSet`.** `pub struct CheckSet { pub disabled: Vec<CheckKind> }` with `fn is_on(&self, k: CheckKind) -> bool` (serde; default = all on).
  - **`Issue`.** `pub struct Issue { pub kind: CheckKind, pub item: Item, pub terms: Vec<String> }`. `terms` is filled for KeyTerm (the missing terms) and Compound (the conjunction found).
  - **`run_checks`.** `pub fn run_checks(doc: &Document, set: &CheckSet) -> Vec<Issue>`. Order: main tree pre-order, then parking; within an item, `CheckKind::ALL` order.
  - **`tokenise`.** `pub fn tokenise(text: &str, lang: &str) -> Vec<String>`:
    - lowercase;
    - strip diacritics for comparison (é→e, ë→e, ö→o; a small mapping table for Latin-1 + Latin Extended-A);
    - drop apostrophe suffixes ("auto's" → "auto");
    - split on non-alphanumerics, with hyphenated words split into parts;
    - drop stopwords for `lang` (`"nl"` or `"en"`; other values → `"en"`) and tokens shorter than 4 characters.
  - **Rules:**
    1. **KeyTerm.** For each card (reasoning maps only, and not for whole-card attacks), find terms of the target statement (the box the card targets) that appear in none of the card's premises. A term counts as present if a premise token equals it or the two share a prefix of length ≥ `max(4, shorter.len() - 2)`. One issue per card, on `Item::Card`, listing the missing terms; skipped if there are none.
    2. **Compound.** A Claim/Premise text containing a standalone conjunction: en `and`/`or`; nl `en`/`of`. Case-insensitive, whole word.
    3. **Question.** A Claim/Premise whose trimmed text ends with `?`.
    4. **Unsupported.** A Premise with no Source child and no Reason card targeting it (reasoning maps only).
    5. **EmptyCategory.** A Category with no Category children (grouping maps only).
    6. **IncompleteEvaluation.** Only applies if at least one rating exists anywhere. Every Claim/Premise without acceptance, every card without strength, and every source without a source rating gets one issue.
    7. **InconsistentEvaluation.**
       - A card rated Strong while all its premises are Reject.
       - A Source rated Solid under a premise rated Reject.
       - A card rated Nil whose premises are all Accept and that has no objection — a "nil but nothing undermines it" hint.
    8. **NotConnected.** Every top-level parking item.
- [ ] **Step 1: Write failing tests.**
  - `key_term_normalises_case_punctuation_diacritics`: target "Coördinatie van auto's", premise "coordinatie AUTO" → no issue.
  - `key_term_dutch_example_from_course`: target "Robots zorgen voor meer banen", premise "Robots verhogen de productiviteit" → missing `["zorgen", "banen"]` (stopwords drop "voor" and "meer").
  - `compound_language_aware`: "of" is flagged in nl but not in en.
  - `question_and_unsupported`.
  - `empty_category_grouping_only`.
  - `incomplete_only_after_first_rating`.
  - `inconsistent_cases`: all three.
  - `not_connected_lists_parking`.
  - `disabled_checks_skipped`.
  - `checks_500_boxes_fast` (`#[ignore]`): < 20 ms in release.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** Stopword lists: about 150 common English and about 150 common Dutch function words, original lists written by the implementer. **Step 4: Run** → PASS.
- [ ] **Step 5: Commit** `feat(core): course-rule checks with English and Dutch tokenising`.

---

### Task 5: Check tab, settings and status bar count

**Files:**
- Create: `app/reasoncraft/src/shell/text_panel/check.rs`
- Modify: `shell/statusbar.rs`, `settings.rs`, `shell/backstage.rs` (check toggles in Settings), `doc_tab.rs` (issue cache), `en.ftl`
- Test: kittests and unit tests

**Interfaces:**
- Consumes: `run_checks`, `CheckSet`, `DocTab.revision`.
- Produces:
  - **Cache.** `DocTab.issue_cache: Option<(u64, CheckSet, Vec<Issue>)>`, with `fn issues(&mut self, set: &CheckSet) -> &[Issue]` that recomputes only on a revision or set change.
  - **Check tab.** Issues are grouped by kind under headings (`t("check.kind.<kind>")`) with counts. Each issue row shows the item's text (truncated to one line with a tooltip of the full text) and a short explanation (`t_args("check.explain.<kind>", terms)`).
    - KeyTerm lists the missing terms and explains the key-word rule in one sentence.
    - Clicking a row selects the item (`Action::Select`) and reveals it on the canvas.
    - Empty state: `t("check.all_clear")`.
  - **Status bar.** An issue count chip (`t_args("status.issues", count)`, Fluent plural) that opens the Check tab and the right panel when clicked (`Action::SetTextTab(Check)` + `Action::ShowPanel(Panel::Right)`; add `ShowPanel` if no equivalent exists).
  - **Settings.** `Settings.checks: CheckSet`. The backstage Settings page gets a "Checks" section with one checkbox per kind (`Action::SetCheckEnabled(CheckKind, bool)`).
- [ ] **Step 1: Write failing tests.**
  - `issues_refresh_after_undo`: add a question premise → 1 Question issue; undo → 0.
  - `issue_click_selects_item`.
  - `status_chip_opens_check_tab`.
  - `disabled_check_hidden_in_tab_and_count`.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** **Step 4: Run** → PASS. Screenshot and look.
- [ ] **Step 5: Commit** `feat(app): check tab, issue count in status bar, per-check settings`.

---

### Task 6: Core snippets

**Files:**
- Create: `crates/reasoncraft_core/src/snippets.rs`
- Modify: `model.rs` (`Document.snippets`), `edit.rs` (`validate`), `lib.rs`
- Test: unit tests

**Interfaces:**
- Produces:
  - **`SnippetSource`.** `pub struct SnippetSource { pub source_type: SourceType, pub citation: String, pub link: Option<String> }`.
  - **`Snippet`.** `pub struct Snippet { pub id: Uuid, pub text: String, pub source: Option<SnippetSource> }`.
  - **`Document.snippets`.** `Vec<Snippet>` with `#[serde(default, skip_serializing_if = "Vec::is_empty")]`.
  - **Edit ops.** `impl Document`, returning `Result<_, EditError>`:
    - `add_snippet(text: &str) -> Uuid`: text is capped at `pub const MAX_SNIPPET_CHARS: usize = 50_000` chars, cut at a char boundary.
    - `update_snippet_text(id, text)`: same cap; `NotFound` if absent.
    - `set_snippet_source(id, Option<SnippetSource>)`: the link goes through the core's trim; app-side `safe_link` happens in Task 7.
    - `remove_snippet(id)`.
    - `move_snippet(id, index)`.
  - **`validate()`.** Snippet ids are unique.
- [ ] **Step 1: Write failing tests.**
  - `snippet_crud_and_order`.
  - `snippet_text_capped_at_char_boundary`: a 60 000-char string with multibyte characters → 50 000 chars, valid UTF-8.
  - `step2_document_without_snippets_unchanged_on_roundtrip`.
  - `duplicate_snippet_ids_invalid`.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** **Step 4: Run** → PASS.
- [ ] **Step 5: Commit** `feat(core): snippets in the document`.

---

### Task 7: Snippets tab

**Files:**
- Create: `app/reasoncraft/src/shell/text_panel/snippets.rs`, `app/reasoncraft/src/actions/snippets.rs`
- Modify: `doc_tab.rs`, `en.ftl`
- Test: kittests and unit tests

**Interfaces:**
- Consumes: Task 6 ops; `safe_link`; `SourceType::ALL` and `paint_source_icon`.
- Produces:
  - **Toolbar.** "New snippet" (an empty snippet, focused) and "Paste as snippet" (reads text from the clipboard via the egui paste event while the panel has focus; it also works by pasting into an empty snippet). Both go through `Action::AddSnippet(String)`.
  - **Snippet card.**
    - A multiline `TextEdit` with its text.
    - Edits are a session, like inline box editing: a snapshot when focus starts and one history entry on focus loss if the text changed. Use `DocTab.snippet_edit: Option<(Uuid, Document /*snapshot*/)>` and `Action::EditSnippetText { id, text }` (live, no history), committed by any non-text action or by focus loss.
    - A source line: type picker (17 icons), citation field and link field. The link is checked with `normalize_link` (invalid → red hint, not stored). Changes go through `Action::SetSnippetSource { id, source }`.
    - Delete × (`Action::RemoveSnippet(id)`) and drag handle ⋮⋮ for reordering within the list (`Action::MoveSnippet { id, index }`).
  - **Paste cap.** Text pasted beyond `MAX_SNIPPET_CHARS` is cut, with a notice `t("snippet.capped")`.
- [ ] **Step 1: Write failing tests.**
  - `snippet_edit_is_one_history_entry`.
  - `oversized_paste_is_capped`: 120 000 chars → capped, notice shown, frame time OK.
  - `invalid_snippet_link_not_stored`.
  - `snippet_reorder`.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** **Step 4: Run** → PASS. Screenshot and look.
- [ ] **Step 5: Commit** `feat(app): snippets tab with editing sessions, sources and reordering`.

---

### Task 8: Drag a snippet selection onto the map

**Files:**
- Create: `app/reasoncraft/src/shell/text_panel/drag_chip.rs`
- Modify: `shell/building.rs` (or wherever `Insertable` lives), `actions/insert.rs`, `canvas/drag/panel.rs`, `en.ftl`
- Test: unit tests and kittests

**Interfaces:**
- Consumes: the step-2 panel drag-and-drop machinery (`Insertable`, `panel_drop_target`, `Action::Insert`).
- Produces:
  - **`Insertable` gains `Text { text: String, source: Option<SnippetSource> }`.**
  - **Drag chip.** When a snippet's `TextEdit` has a non-empty, non-whitespace selection, a small "Drag to map" chip appears under the snippet showing the first 40 characters. Dragging the chip carries `Insertable::Text` with the selected text, trimmed, and the snippet's source. The whole snippet can be dragged from its ⋮⋮ handle with Alt held (the handle alone is reordering).
  - **Drop semantics** (`panel_drop_target` rules extended):
    - on a Claim/Premise → a new Reason card targeting that box, with the text as its premise;
    - on a card bottom band → a whole-card Reason;
    - on a Category → a new Category child with the text;
    - on empty canvas → a loose Reason card in parking.

    If the text carries a source, a Source box (type, citation as text, link) is added under the new premise in the same history entry. This is one `DocTab::edit` closure performing both ops.
  - **Refusals.** A whitespace-only selection is refused (no chip). Notes and sources as targets are refused (red).
- [ ] **Step 1: Write failing tests.**
  - `text_drop_on_premise_adds_reason_with_source_in_one_step`: undo removes both.
  - `text_drop_on_category_adds_category`.
  - `blank_selection_drop_refused`.
  - `long_selection_drop_keeps_text`: 3 000 chars → box text equal, layout height grows.
  - Kittest `drag_chip_appears_for_selection`.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** **Step 4: Run** → PASS. Try it by hand.
- [ ] **Step 5: Commit** `feat(app): drag snippet selections onto the map with their source`.

---

### Task 9: Document language

**Files:**
- Modify: `shell/backstage.rs` (Document page), `actions/…`, `en.ftl`, `README.md`
- Create (core): `pub fn set_language(&mut self, lang: &str) -> Result<(), EditError>` in `evaluate.rs` or a small `meta.rs`
- Test: unit tests and a kittest

**Interfaces:**
- Produces:
  - **Core op.** `Document::set_language(lang)` accepts `"en"` or `"nl"`, else `NotAllowed`. It is a no-op when unchanged.
  - **Action.** `Action::SetDocLanguage(String)`.
  - **Backstage "Document" page.** Title, author and language (a picker with `t("lang.en")` = "English" and `t("lang.nl")` = "Dutch"). Title and author edits go through `Action::SetDocMeta { title, author }`, one history entry per commit.
  - **New documents** take their language from `Settings.default_doc_language` (default `"en"`), which is also set on the Settings page.
  - **Checks** use `doc.meta.language` (Task 4 already reads it); verify with a kittest that switching to Dutch changes the Compound result for "of".
- [ ] **Step 1: Write failing tests.**
  - `set_language_validates`.
  - `switching_language_changes_checks` (kittest).
  - `new_doc_uses_default_language`.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** **Step 4: Run** → PASS.
- [ ] **Step 5: Commit** `feat: document language and metadata page`.

---

### Task 10: End-to-end verification and docs

**Files:**
- Modify: `app/reasoncraft/tests/e2e.rs`, `README.md`, `docs/todo.md`, `docs/rulings.md`

- [ ] **Step 1: Extend e2e.** Through actions:
  - add two snippets, one with a source;
  - drag (via `Action::Insert { Text … }`) a selection onto a premise;
  - set the language to nl;
  - toggle outline options;
  - save and reopen.

  Then assert document equality (snippets included), `outline_plain_text` equality before and after, and the same issues list before and after.
- [ ] **Step 2: Run** `cargo test --workspace`, clippy and fmt → clean; `cargo build --release -p reasoncraft` → success.
- [ ] **Step 3: Manual check in release.**
  - Build a map bottom-up from snippets.
  - Check the outline numbering and copy.
  - Resolve check issues by clicking.
  - Switch language.

  Take dark and light screenshots and look at them. Record the step-3 review in `docs/todo.md` and tick the step-3 carry-over items done in Task 1.
- [ ] **Step 4: README.** Text panel overview, and any new shortcuts in the tables (exactly matching `key_to_action`).
- [ ] **Step 5: Commit** `docs: step 3 verification and usage notes`.

---

## Decisions made in this plan (not in the spec)

- **Outline numbering:** `1`, `1.1`, …. Co-premises get letter suffixes (`1a`, `1b`), whole-card attacks `.w1`, and parked items `P1`, `P2`.
- **Key-term matching:** lowercase, diacritics stripped, apostrophe suffixes dropped, stopwords and tokens shorter than 4 characters removed, and a shared-prefix match of at least `max(4, len−2)`. Dutch plural/stem changes beyond that (e.g. "baan"/"banen") are a known limitation, not handled.
- **Inconsistent evaluation:** includes a third rule beyond the spec's two examples ("nil but nothing undermines it").
- **Snippets:** capped at 50 000 characters each.
- **Snippet drops:** create a Reason by default; change it with Type ▾ afterwards. Dropping on a Category creates a Category.
- **Document language:** only "en" and "nl" in this step.
