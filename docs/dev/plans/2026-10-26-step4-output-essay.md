# ReasonCraft Step 4 — Output and Essay — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Get maps out of ReasonCraft and into writing.
- Export images: PNG, SVG and PDF. Copy the map as an image to the clipboard. Print via PDF.
- Export the outline and an essay plan: Markdown, plain text and Word (.docx).
- A working Essay tab: a writing plan ordered strongest-first, with paragraph slots saved in the document.

Two clean-ups from earlier reviews come first:
1. One unified edit-session model.
2. A drawing "scene" that the screen painter and the exporters both use.

**Architecture:**
- **Scene.** The map is first turned into a backend-neutral `Scene`: a list of shapes, text runs laid out in lines, and images referenced by asset key. The screen painter draws a `Scene`.
- **Exports from the scene.**
  - SVG is written from the `Scene`.
  - PNG is the SVG rasterised with `resvg`.
  - PDF is the SVG converted with `svg2pdf`, with fonts embedded and text selectable.
  - Copy-as-image puts the PNG on the clipboard through `arboard`.
- **Text documents.**
  - Outline and essay exports are built in core as Markdown/plain text.
  - Word files are generated with `docx-rs`.
  - Linking words ("because", "omdat") follow the document's language.
- **Essay plan.** It is a new optional `Document.essay` field. Its text slots are edited through the unified edit session.

**Tech Stack:** step 3's stack plus `resvg` + `usvg` 0.49 and `svg2pdf` 0.13 (all MIT/Apache; pick versions that share one `usvg`), `docx-rs` 0.4 (MIT), and `arboard` 3 with `image-data` (MIT/Apache). The fonts embedded in exports are egui's bundled default fonts (OFL/Ubuntu Font Licence/Apache), so exports look like the screen.

**Spec:** `docs/dev/specs/2026-10-25-reasoncraft-design.md`:
- §6: the Essay tab and the Export ribbon.
- §9: exports — PNG with chooseable scale and optional transparent background; SVG; PDF with page setup; copy as image; whole map or current selection; Outline/Essay as Markdown, plain text and .docx; print via PDF.
- §5: exports default to the light theme, with an option for the current theme.

Also the step-2 and step-3 carry-overs in `docs/dev/todo-history.md`.

## Global Constraints

- **Platform.** Pure native egui, no webview. Windows, macOS and Linux. Fully offline.
- **Strings and colours.** All UI text goes through `i18n::t` / `t_args` (keys in `en.ftl`). All colours come from `theme::Tokens`. Exports use the token values of the chosen theme.
- **Mutation.** `actions::apply` is the only mutator. Document changes go through `DocTab::edit` (exactly one history entry; none when refused or a no-op).
- **File format.**
  - New fields are optional (`#[serde(default, skip_serializing_if…)]`, with a flattened `extra` on new structs).
  - `SCHEMA_VERSION` stays 1. The essay is a new optional field.
  - Step 1–3 files open unchanged.
- **Exports are faithful.** An export shows exactly what the map shows: the same layout, wrapped lines, colours, rating visuals, source icons and images. Screen-only overlays never appear in exports: selection ring, "+" button, comment badges, link glyphs, dot grid, and the parking frame unless the parking area is exported.
- **No synthetic OS input.** Never inject OS-level keyboard or mouse input during verification. Use kittests, files, and screenshots of the app window only.
- **Quality gate.** `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check` and `cargo test --workspace` pass at every commit. Files stay at roughly 400 lines or fewer.

## Review Focus

1. **Huge exports.** A 500-box map at 4× PNG scale must not freeze the UI or crash.
   - Cap pixel dimensions at 16 384 per side and 256 MP total; above that, show a message suggesting SVG/PDF.
   - Exports run on a worker thread with a busy state.
   - Tests: Task 5 `png_export_capped`, Task 7 `export_runs_off_ui_thread`.
2. **Text in exports matches the screen.** The same line breaks, and no clipped or overflowing text, including Dutch diacritics, long words and emoji fallback. Test: Task 3 `svg_text_rows_match_layout`.
3. **Fonts missing on the target machine.** PDF and SVG must not depend on installed fonts. PDF embeds them. SVG converts text to outlines by default, with an option to keep live text. Test: Task 4 `svg_outlines_have_no_text_elements`.
4. **Essay slots for items that were deleted or moved.** Text written for a branch must not be lost silently when that branch is deleted. It moves to an "Unplaced notes" list in the Essay tab, never dropped. Undo brings both back. Test: Task 9 `deleting_branch_moves_its_paragraph_to_unplaced`.
5. **Word export opens in Word and LibreOffice.** The file must be a valid OOXML zip: `[Content_Types].xml` and `word/document.xml` present and well-formed, headings using built-in styles. Test: Task 10 `docx_is_valid_ooxml`.

---

## File Structure

```
crates/reasoncraft_core/src/
  roles.rs           NEW (moved) owner_index, card_roles; model.rs shrinks
  words.rs           NEW linking words per document language (en/nl)
  outline_md.rs      NEW outline → Markdown
  essay.rs           NEW EssayPlan model + build_essay_order() + essay → Markdown/plain
app/reasoncraft/src/
  doc_tab/session.rs NEW  EditSession enum (Box/Snippet/Meta/Essay) + one snapshot
  actions/…          split so no file > ~400 lines
  scene/mod.rs       NEW  Scene, Shape, TextRun, build_scene(doc, layout, opts, fonts)
  scene/egui_paint.rs NEW Scene → egui Painter (canvas uses this)
  export/svg.rs      NEW  Scene → SVG string (live text or outlines)
  export/raster.rs   NEW  SVG → PNG (resvg); clipboard image (arboard)
  export/pdf.rs      NEW  SVG → PDF (svg2pdf) + page setup (fit / tile)
  export/docx.rs     NEW  outline/essay → .docx (docx-rs)
  export/job.rs      NEW  worker thread + progress/busy state
  shell/ribbon/export.rs NEW Export tab
  shell/export_dialog.rs NEW export options dialog
  shell/text_panel/essay.rs NEW Essay tab
  canvas/images.rs   image insert moves to a worker thread
```

---

### Task 1: Unified edit session and file splits

**Files:**
- Create: `app/reasoncraft/src/doc_tab/session.rs`, `crates/reasoncraft_core/src/roles.rs`.
- Modify: `doc_tab.rs` (and its submodules), `actions.rs` and `actions/*` (splits), `model.rs`, `lib.rs`, `en.ftl`.
- Test: existing session tests must keep passing unchanged, plus the new tests below.

**Interfaces:**
- **Produces:**
  - `pub enum EditSession { Box { node: NodeId }, Snippet { id: Uuid }, Meta, Essay { slot: EssaySlot } }`. `EssaySlot` is defined in Task 8; until then it is a placeholder type in core `essay.rs` with `#[allow(dead_code)]`.
  - `DocTab.session: Option<(EditSession, Document /*snapshot*/)>` replaces the three separate fields. It comes with:
    - `fn begin_session(&mut self, s: EditSession)` — commits any other open session first.
    - `fn commit_session(&mut self) -> bool` — records exactly one history entry if the document changed; returns whether it recorded.
    - `fn session(&self) -> Option<&EditSession>`.
  - The live-edit helpers (`set_edit_text`, `edit_snippet_live`, `edit_meta_live`) use these. `commit_edits()` becomes `commit_session()`. All behaviour stays identical: the same history counts and dirty rules.
  - `owner_index`, `card_roles` and a new `pub fn role_of(doc, CardId) -> Role` move to `roles.rs`. Re-exports stay the same. Add the test `card_roles_equals_card_role` on `sample_iot_like(117)`.
  - `actions.rs`, `doc_tab.rs` and `model.rs` end at ≤ ~400 lines each.
  - The "not rated yet" explanation gets a Fluent plural selector (`[one]` / `*[other]`).
  - Release-mode test build: put `#[cfg(debug_assertions)]` on the core unit test that calls the debug-only helper, so `cargo test --workspace --release` builds.
- [ ] **Step 1:** Write failing tests:
  - `only_one_session_at_a_time` — box → snippet → meta transitions, each committing the previous one with exactly one history entry.
  - `card_roles_equals_card_role`.
  - `incomplete_plural_one`.
  - Then run `cargo test --workspace --release` and confirm it fails to build.
- [ ] **Step 2:** Run → FAIL. **Step 3:** Implement as a behaviour-preserving refactor. **Step 4:** `cargo test --workspace` and `cargo test --workspace --release` → PASS.
- [ ] **Step 5:** Commit `refactor: one edit session model; split oversized files; roles module`.

---

### Task 2: Scene builder and screen painter on the scene

**Files:**
- Create: `app/reasoncraft/src/scene/{mod.rs, egui_paint.rs}`.
- Modify: `canvas/paint.rs`, `canvas/boxes.rs`, `canvas/rating.rs`, `canvas/images.rs` (the canvas now builds a `Scene` and paints it).
- Test: unit tests in `scene/mod.rs`, plus a kittest showing the canvas still renders.

**Interfaces:**
- **Consumes:** `Layout`, `Document`, `Tokens`, `PaintOptions` (`show_markers`, `eval_shading`), the measure helpers (`font_for`, `wrapped_rows`), `paint_source_icon`, rating helpers and the connector geometry.
- **Produces:**
  - `pub struct SceneOptions { pub tokens: Tokens, pub show_markers: bool, pub eval_shading: bool, pub for_export: bool, pub include_parking: bool, pub only: Option<Item> }`. `only` is the subtree for a selection export.
  - `pub enum Shape`:
    - `Rect { rect: Rect, radius: f32, fill: Color32, stroke: Option<(f32, Color32)> }`
    - `Path { points: Vec<PathEl>, stroke: Option<(f32, Color32)>, fill: Option<Color32>, dashed: bool }`, where `PathEl { MoveTo(Pos2), LineTo(Pos2), CubicTo(Pos2, Pos2, Pos2), Close }`
    - `Circle { center: Pos2, radius: f32, fill: Option<Color32>, stroke: Option<(f32, Color32)> }`
    - `Text { origin: Pos2, rows: Vec<String>, size: f32, bold: bool, color: Color32, align: Align, line_height: f32 }`
    - `Image { rect: Rect, asset: String }`
  - Everything is in world coordinates.
  - `pub struct Scene { pub shapes: Vec<Shape>, pub bounds: Rect }`.
  - `pub fn build_scene(doc: &Document, layout: &Layout, opts: &SceneOptions, fonts: &dyn TextRows) -> Scene`, where `trait TextRows { fn rows(&self, text: &str, width: f32, kind: &BoxKind) -> Vec<String>; }` is implemented by `EguiMeasure` (the same wrap the layout uses).
  - Source icons and the rating dot circles become `Path`/`Circle` shapes. The icon painting is refactored to emit shapes, and the egui path draws the shapes.
  - Screen-only overlays (selection ring, plus button, badges, link glyphs, dot grid) are added only when `for_export == false`. The parking frame is added only when `include_parking` (always true on screen).
  - `pub fn paint_scene(painter: &Painter, scene: &Scene, cam: &Camera, images: &ImageCache)`. The canvas calls `build_scene` + `paint_scene` instead of the old direct painting. Culling: skip shapes outside the view.
  - The pixel output must stay visually identical. Verify with before/after screenshots of the window for the same sample file.
- [ ] **Step 1:** Write failing tests:
  - `scene_has_no_overlays_for_export` — a selection, a comment and a link are present, `for_export = true`, and the result contains no badge, glyph or ring shapes.
  - `scene_selection_subtree_only` — `only = Some(card)` gives only that card, its descendants and their connectors.
  - `scene_text_rows_from_measure`.
- [ ] **Step 2:** Run → FAIL. **Step 3:** Implement. **Step 4:** Run → PASS. Take before/after window screenshots of the same file (prepared on disk) and compare them by eye.
- [ ] **Step 5:** Commit `refactor(app): backend-neutral scene; canvas paints the scene`.

---

### Task 3: SVG writer

**Files:** create `app/reasoncraft/src/export/{mod.rs, svg.rs}`. Test: unit tests.

**Interfaces:**
- **Consumes:** `Scene`, `Assets`.
- **Produces:**
  - `pub struct SvgOptions { pub text_as_paths: bool, pub transparent: bool, pub background: Color32, pub margin: f32 /*world px, default 24*/ }`.
  - `pub fn scene_to_svg(scene: &Scene, assets: &Assets, opts: &SvgOptions, font_family: &str) -> String`. It produces valid SVG 1.1:
    - `viewBox` = scene bounds plus the margin.
    - Text is one `<text>` per row with `xml:space="preserve"` and XML-escaped content.
    - Images are embedded as `data:image/png;base64,…` from the full-resolution assets. A missing asset becomes a dashed placeholder rect.
    - Colours are written as `#rrggbb` plus `fill-opacity`/`stroke-opacity`.
  - `text_as_paths` is handled after generation: parse with `usvg` using the bundled fonts loaded into a `fontdb`, convert text to paths, and serialise with `usvg`'s writer. Expose `pub fn bundled_fontdb() -> fontdb::Database` (egui default font bytes).
- [ ] **Step 1:** Write failing tests:
  - `svg_parses_with_usvg` — a sample map produces SVG that `usvg::Tree::from_str` parses.
  - `svg_text_rows_match_layout` — for every box, the number of `<text>` rows equals the `TextRows` rows for that box.
  - `svg_escapes_text` — the text `<b>&"x"` comes out escaped.
  - `svg_embeds_images`.
  - `svg_outlines_have_no_text_elements`.
- [ ] **Step 2:** Run → FAIL. **Step 3:** Implement. **Step 4:** Run → PASS.
- [ ] **Step 5:** Commit `feat(app): SVG export from the scene`.

---

### Task 4: PNG export and copy as image

**Files:** create `app/reasoncraft/src/export/raster.rs`. Modify `Cargo.toml` (`resvg`, `arboard` with `image-data`). Test: unit tests.

**Interfaces:**
- **Produces:**
  - `pub struct PngOptions { pub scale: f32 /*1,2,3,4*/, pub transparent: bool }`.
  - `pub fn svg_to_png(svg: &str, opts: &PngOptions, fonts: &fontdb::Database) -> Result<Vec<u8>, ExportError>`. It enforces the caps in `MAX_SIDE_PX = 16_384` and `MAX_PIXELS = 256_000_000`, failing with `ExportError::TooLarge { w, h }` before allocating.
  - `pub fn copy_png_to_clipboard(png_rgba: (u32, u32, Vec<u8>)) -> Result<(), ExportError>`, using `arboard::ImageData`.
  - `ExportError { TooLarge{w,h}, Io(std::io::Error), Render(String), Clipboard(String) }` maps to `t("export.error.<variant>")`.
- [ ] **Step 1:** Write failing tests:
  - `png_has_expected_size_for_scale` — bounds plus margin, times scale.
  - `png_transparent_background_has_alpha`.
  - `png_export_capped` — a scene with 20 000-px bounds at scale 4 gives `TooLarge` without a large allocation.
- [ ] **Step 2:** Run → FAIL. **Step 3:** Implement. **Step 4:** Run → PASS.
- [ ] **Step 5:** Commit `feat(app): PNG export and copy map as image`.

---

### Task 5: PDF export, page setup and print

**Files:** create `app/reasoncraft/src/export/pdf.rs`. Modify `Cargo.toml` (`svg2pdf`). Test: unit tests.

**Interfaces:**
- **Produces:**
  - `pub enum Paper { A4, Letter }` and `pub enum Orientation { Portrait, Landscape }`.
  - `pub enum Fit { OnePage, Tile { scale: f32 } }`.
  - `pub struct PageSetup { pub paper: Paper, pub orientation: Orientation, pub fit: Fit, pub margin_mm: f32 /*10*/ }`, persisted in `Settings.page_setup`.
  - `pub fn svg_to_pdf(svg: &str, setup: &PageSetup, fonts: &fontdb::Database) -> Result<Vec<u8>, ExportError>`:
    - `OnePage` scales the drawing to fit one page.
    - `Tile` splits it into pages at the given scale, 1.0 meaning 1 world px = 0.25 mm. Each page carries a small page label "r/c" in the corner.
    - Fonts are embedded and text stays selectable (text is NOT converted to paths for PDF).
  - **Print:** `pub fn print_via_pdf(pdf: &[u8]) -> Result<(), ExportError>` writes the PDF to a temp file named `ReasonCraft-print-<timestamp>.pdf` and opens it with `open::that_detached` (the OS viewer prints it). Old print files are removed on the next start.
- [ ] **Step 1:** Write failing tests:
  - `pdf_one_page_has_one_page` — parse the page count by scanning for `/Type /Page`.
  - `pdf_tile_page_count`.
  - `pdf_contains_font_and_text` — `/FontFile` present, and the claim text is extractable as a substring from a content stream, or checked through a minimal parser.
  - `print_writes_temp_pdf` — uses a fake opener.
- [ ] **Step 2:** Run → FAIL. **Step 3:** Implement. **Step 4:** Run → PASS.
- [ ] **Step 5:** Commit `feat(app): PDF export with page setup and print via PDF`.

---

### Task 6: Core outline Markdown and linking words by document language

**Files:** create `crates/reasoncraft_core/src/{words.rs, outline_md.rs}`. Test: unit tests.

**Interfaces:**
- **Produces:**
  - `pub struct LinkingWords { pub because: &'static str, pub but: &'static str, pub however: &'static str }` and `pub fn linking_words(lang: &str) -> LinkingWords`:
    - `en` gives because / but / however;
    - `nl` gives omdat / maar / echter;
    - other values fall back to `en`, using the same `is_dutch` rule.
  - `pub fn source_word(t: &SourceType, lang: &str) -> &'static str` gives the source type name in the document language. Use small en/nl tables written here; these are document content, not UI.
  - `pub fn outline_markdown(lines: &[OutlineLine], opts: &OutlineOptions, lang: &str) -> String`:
    - The claim becomes `# heading`.
    - Each other line becomes a nested `- ` list item (two spaces of indent per depth), formatted `**{number}** *{word}* {text}`. Parts are left out according to `opts`.
    - Sources become `- _{SourceName}:_ {text}`.
    - Parked items go under `## {not connected}`, with the heading in the document language (en "Not connected", nl "Niet verbonden").
- [ ] **Step 1:** Write failing tests:
  - `words_nl_and_fallback`.
  - `markdown_matches_expected` — an exact string for a small map in en, and the same map in nl.
  - `markdown_escapes_special_chars` — `*`, `_`, `#` and backticks in box text are escaped.
- [ ] **Step 2:** Run → FAIL. **Step 3:** Implement. **Step 4:** Run → PASS.
- [ ] **Step 5:** Commit `feat(core): outline Markdown and linking words per document language`.

---

### Task 7: Export ribbon tab, export dialog and background jobs

**Files:** create `app/reasoncraft/src/shell/ribbon/export.rs`, `app/reasoncraft/src/shell/export_dialog.rs`, `app/reasoncraft/src/export/job.rs`. Modify `file_ops/dialogs.rs` (add `pick_export`), `actions/…`, `en.ftl`. Test: kittests.

**Interfaces:**
- **Produces:**
  - `pub enum ExportFormat { Png, Svg, Pdf, OutlineMarkdown, OutlineText, OutlineDocx, EssayMarkdown, EssayDocx }`.
  - `pub enum ExportScope { Map, Selection }`.
  - `Action::OpenExport(ExportFormat)` opens the dialog for that format. Options per format:
    - all image formats: scope, theme (light default / current);
    - PNG: scale 1–4 and transparent background;
    - SVG: text as outlines (default on);
    - PDF: page setup.

    Settings are remembered in `Settings.export`.
  - `Action::RunExport(ExportRequest)` asks for a path (`FileDialogs::pick_export(format, suggested_name) -> Option<PathBuf>`; the extension is forced) and starts a worker thread through `ExportJob`. The UI shows a busy chip in the status bar. On success it shows `t("export.done")` with an "Open folder" link (`open::that_detached` on the folder); on failure it shows an error modal.
  - `Action::CopyAsImage` copies the PNG of the whole map (scale 2, current theme) to the clipboard on the worker.
  - `Action::Print` builds the PDF with the saved page setup and calls `print_via_pdf`.
  - **Export ribbon tab** groups:
    - Image: PNG, SVG, PDF, Copy as image.
    - Text: Outline ▾ (Markdown, plain text, Word) and Essay ▾ (Markdown, Word). The Essay entries are disabled until Task 10.
    - Print: Page setup…, Print.
  - The Home tab's step-1 "Copy as image" placeholder is now wired to `CopyAsImage`.
  - `FakeDialogs` implements `pick_export`.
  - The job reads a snapshot: it clones the `Document`, `Assets` and `Layout` before spawning, so editing during an export is safe.
- [ ] **Step 1:** Write failing tests:
  - `export_png_writes_file` — FakeDialogs path, then wait for the job in the test harness.
  - `export_runs_off_ui_thread` — the job is spawned, the harness keeps stepping frames while the job sleeps (inject a test delay), and the busy chip is visible.
  - `export_selection_scope_only_subtree`.
  - `copy_as_image_puts_image_on_clipboard` — a fake clipboard trait.
  - `outline_markdown_export_uses_doc_language`.
- [ ] **Step 2:** Run → FAIL. **Step 3:** Implement. **Step 4:** Run → PASS. Export PNG, SVG and PDF of a prepared sample file and look at all three, using the OS image viewer for PNG/SVG and the PDF viewer for PDF. Screenshots are fine, but no synthetic input.
- [ ] **Step 5:** Commit `feat(app): export tab, export dialog, background export jobs, copy as image, print`.

---

### Task 8: Core essay plan

**Files:** create `crates/reasoncraft_core/src/essay.rs`. Modify `model.rs` (add `Document.essay`), `lib.rs`. Test: unit tests.

**Interfaces:**
- **Produces:**
  - `pub enum EssaySlot { Intro, Conclusion, Item(Item) }` (serde).
  - `pub struct EssayPlan { pub paragraphs: IndexMap<String /*slot key*/, String>, pub unplaced: Vec<(String /*former slot label*/, String /*text*/)>, #[serde(flatten)] extra }`.
    - Slot keys are `"intro"`, `"conclusion"` and `"item:<uuid>"`.
    - `Document.essay: EssayPlan` uses `serde(default, skip_if empty)`.
  - `pub fn essay_order(doc: &Document) -> Vec<EssaySection>`, with `EssaySection { item: Item /*first-level card*/, strength_rank: u8, elements: Vec<EssayElement> }` and `EssayElement { slot: EssaySlot, role: ElementRole /*Claim, Reason, Evidence, Counter, Rebuttal*/, text: String, depth }`.
    - First-level cards under the claim are sorted strongest-first: Strong > unrated > Weak > Nil. Ties keep map order. Reasons sort before objections only within equal strength.
    - Each section lists its card's premises, then their sources (Evidence), then their objections (Counter) and rebuttals, in display order.
  - **Edit ops** on `Document`:
    - `set_essay_text(slot, text)` — capped at 50 000 chars; an empty string removes the slot.
    - `remove_unplaced(index)`.
  - **Integrity hook.** When any edit op removes an item that has essay text (through `remove_item`, or when cards vanish), that text moves to `unplaced` with a label taken from the item's text (first 60 chars). Implement this inside `remove_item` and the other removal paths in `edit.rs`. The app's undo restores both, because it is all one document change.
  - `pub fn essay_markdown(doc: &Document, lang: &str) -> String` and `essay_plain_text`. These produce headings (Introduction / Inleiding, then Section n: claim text, then Conclusion / Conclusie in the document language), the written paragraphs, and as italic hints the map statements for slots without text.
- [ ] **Step 1:** Write failing tests:
  - `essay_order_strongest_first`.
  - `deleting_branch_moves_its_paragraph_to_unplaced`.
  - `undo_restores_paragraph_slot` — done as a core snapshot test: clone, remove, compare with the clone.
  - `essay_markdown_nl_headings`.
  - `essay_roundtrip_and_old_files`.
- [ ] **Step 2:** Run → FAIL. **Step 3:** Implement. **Step 4:** Run → PASS.
- [ ] **Step 5:** Commit `feat(core): essay plan with strongest-first ordering and safe slots`.

---

### Task 9: Essay tab

**Files:** create `app/reasoncraft/src/shell/text_panel/essay.rs` (split into `essay_slot.rs` if needed). Modify `actions/…`, `en.ftl`. Test: kittests.

**Interfaces:**
- **Consumes:** `essay_order`, `EssaySlot`, `set_essay_text`, `EditSession::Essay` (Task 1).
- **Produces:**
  - **Tab layout.** Intro slot, then one section per first-level branch (header showing strength dots and claim text), then the Conclusion slot.
  - **Elements.** Each element shows its map statement (role-coloured left bar, the same visuals as the outline) and, below it, a multiline paragraph field for that element.
  - **Editing.** Text is edited through `EditSession::Essay { slot }`: live `Action::EditEssayText { slot, text }` with no history, committed into one entry like snippets.
  - **Navigation.** Clicking a statement selects it on the map and reveals it.
  - **Help.** A "How to use" note (`t("essay.help")`) explains the lesson-5 structure: claim in the introduction, strongest argument first, one section per branch, conclusion repeats the claim.
  - **Unplaced notes.** Listed at the bottom with "Copy" and "Remove" buttons.
  - **Revision cache.** `essay_order` is cached per revision, like the outline.
- [ ] **Step 1:** Write failing tests:
  - `essay_tab_lists_sections_strongest_first`.
  - `essay_typing_is_one_history_entry`.
  - `essay_session_commits_on_save_and_close`.
  - `deleting_branch_shows_unplaced_note`.
- [ ] **Step 2:** Run → FAIL. **Step 3:** Implement. **Step 4:** Run → PASS. Take a window screenshot from a prepared file and look at it.
- [ ] **Step 5:** Commit `feat(app): essay tab with paragraph slots`.

---

### Task 10: Word (.docx) export for outline and essay

**Files:** create `app/reasoncraft/src/export/docx.rs`. Modify `Cargo.toml` (`docx-rs`), the export dialog and the ribbon (enable the Essay entries). Test: unit tests and kittests.

**Interfaces:**
- **Produces:**
  - `pub fn outline_docx(lines: &[OutlineLine], opts: &OutlineOptions, lang: &str) -> Result<Vec<u8>, ExportError>`. The claim uses the Title style. Lines are Normal paragraphs with a left indent of 0.6 cm per depth; the number is bold and the linking word italic.
  - `pub fn essay_docx(doc: &Document, lang: &str) -> Result<Vec<u8>, ExportError>`:
    - Heading 1 for Introduction / sections / Conclusion.
    - Written paragraphs as Normal.
    - Slots without text become italic grey hints.
    - The title comes from `DocMeta.title`, falling back to the claim, and the author from `DocMeta.author`.
  - Both are wired into `ExportFormat::{OutlineDocx, EssayDocx, EssayMarkdown}`.
- [ ] **Step 1:** Write failing tests:
  - `docx_is_valid_ooxml` — unzip; `[Content_Types].xml`, `word/document.xml` and `word/styles.xml` exist; `document.xml` parses as XML (use `quick-xml` as a dev-dependency, or `roxmltree`); the heading paragraphs reference the Heading1 style.
  - `essay_docx_contains_paragraph_text`.
  - `outline_docx_dutch_words`.
- [ ] **Step 2:** Run → FAIL. **Step 3:** Implement. **Step 4:** Run → PASS. Optionally open a generated `.docx` in any installed word processor to look at it; if none is installed, say so in the report.
- [ ] **Step 5:** Commit `feat(app): Word export for outline and essay`.

---

### Task 11: Image insert off the UI thread

**Files:** modify `canvas/images.rs` / `actions/images.rs`, and add a small `ImageJob`. Test: kittest.

**Interfaces:**
- `Action::InsertImage` reads the file and runs `prepare_image` on a worker thread. The box shows a "Preparing image…" placeholder; the status bar busy chip is shared with the export jobs (`export/job.rs` generalised into `jobs.rs`).
- On completion, the result is applied through an `Action::ImagePrepared { node, result }` pushed from the job poll. That action does the `assets.insert` + `edit(set_image)` as exactly one history entry, and only if the box still exists and has not changed kind. Otherwise it is discarded with a status note.
- [ ] **Step 1:** Write failing tests:
  - `image_insert_runs_off_ui_thread` — inject a test delay; frames keep stepping and the placeholder is visible.
  - `image_prepared_for_deleted_box_is_discarded`.
- [ ] **Step 2:** Run → FAIL. **Step 3:** Implement. **Step 4:** Run → PASS.
- [ ] **Step 5:** Commit `feat(app): prepare inserted images on a worker thread`.

---

### Task 12: End-to-end verification and docs

**Files:** modify `app/reasoncraft/tests/e2e/*`, `README.md`, `docs/dev/todo-history.md`, `docs/rulings.md`.

- [ ] **Step 1:** Extend e2e through actions:
  - write essay text in the intro and two sections;
  - delete a branch with text and check that its text is in unplaced;
  - undo;
  - export PNG, SVG, PDF, outline Markdown/.docx and essay Markdown/.docx through FakeDialogs, waiting for jobs;
  - assert the files exist and are valid (PNG signature and size; SVG parses; PDF page count; `.docx` OOXML check);
  - save and reopen, and check the essay round-trips.
- [ ] **Step 2:** `cargo test --workspace` (debug and `--release`), clippy and fmt → clean; `cargo build --release -p reasoncraft` succeeds.
- [ ] **Step 3:** Manual check without synthetic input:
  - prepare a sample `.rcraft` (Dutch, rated, with an image and essay text) and export it in all formats through a headless example binary or a kittest that writes into the SDD workspace;
  - look at the PNG and the SVG→PNG; look at the PDF by rendering a page with `pdfium` if available, otherwise list its page count and fonts;
  - take window screenshots of the Essay tab and the Export tab;
  - record the results in the `docs/dev/todo-history.md` "Review — step 4" section, and tick the carry-overs done.
- [ ] **Step 4:** README: an Export/Print section and an Essay section, with shortcut tables kept exact. Add the step-4 rulings from the ledger to `docs/rulings.md`.
- [ ] **Step 5:** Commit `docs: step 4 verification and usage notes`.

---

## Decisions made in this plan (not in the spec)

- **One export path.** Every image export goes through SVG: PNG is rendered from it with `resvg`, and PDF is converted from it with `svg2pdf`.
- **SVG text.** Text is converted to outlines by default; live text is an option. PDF keeps text selectable, with fonts embedded.
- **PDF page setup.** Paper is A4 or Letter, Portrait or Landscape. The drawing either fits on one page or is tiled across pages at a chosen scale. Pages are labelled "row/column".
- **Printing.** The app makes a PDF and opens it in the system viewer; there is no native print dialog.
- **Linking words and source names in exports.** They follow the document's language. Only English and Dutch are covered.
- **Essay order.** Branches are sorted by strength, strongest first: Strong, then unrated, then Weak, then Nil. Within the same strength, reasons come before objections.
- **Essay text of deleted branches.** It is moved to "Unplaced notes", never thrown away.
- **Export size caps.** Exports are capped at 16 384 px per side and 256 megapixels in total.
