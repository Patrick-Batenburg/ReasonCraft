# ReasonCraft — Design

Date: 2026-10-25 · Owner: Patrick van Batenburg · Status: draft for approval

Items marked **(builder's call)** were left to whoever builds this. Items marked **(designer's call)** were chosen during design without the owner deciding them explicitly; they can be revisited.

## 1. Intent

ReasonCraft is a free, open-source, cross-platform desktop app for building **argument maps**: diagrams showing a claim, the reasons for and against it, how those reasons combine, and the evidence behind them. It also builds **grouping maps**, trees that sort a topic into categories.

It aims to be modern, free, maintained and self-contained.

What makes it worth building:
- Proper argument structure: co-premises (premises that only support a claim together), objections, rebuttals, and arguments aimed at a whole card rather than one premise.
- Evidence attached to each premise as a typed source (publication, statistic, expert opinion…).
- Evaluation: rating how strong each reason or objection is and whether each premise or source holds up, shown visually on the map.
- Turning a map into writing: an outline and an essay plan, strongest argument first.

**Audience:** everyone: students and teachers first (secondary school to university), but also professionals. It must not feel like school-only software.

**Reference material** (read-only, the owner's own files, not part of the project):
- Course slides on argumentation and on turning a map into an essay.
- An essay written from a reasoning map.
- Example maps as images: a small reasoning map, a large reasoning map (~117 boxes) and a grouping map.

These images show the target structure and size, and the slides contain the rules the checks implement. They are for understanding only; no content or visuals are copied into the app.

**Success:** a student can build, evaluate and hand in a full reasoning map and grouping map like a typical course assignment (claim, ≥5 lines of reasoning with co-premises and sources, evaluated, exported to PDF/PNG) without any other tool, on Windows, macOS or Linux. Large maps (100+ boxes) stay readable.

## 2. Goals, non-goals, anti-goals

**Goals**
- Reasoning maps and grouping maps in one tool, one document model.
- Modern Office-style ribbon UI (modelled on WordCraft/Word), dark mode, multiple interface languages.
- Two equally good ways to build: top-down (type the claim, add reasons) and bottom-up (collect source text, then arrange it).
- Course-rule checks that help people improve their maps.
- Exports for handing in and sharing: PNG, SVG, PDF, copy as image, outline, essay plan, Argdown.
- Open text interchange via Argdown import/export.
- Full keyboard operation and screen-reader support.

**Non-goals (for now)**
- Real-time collaboration, accounts or cloud sync. Sharing happens through files and exports. Keep stable IDs in the format so collaboration stays possible later.
- An online map-sharing URL.
- Update prompts, website links, an embedded web browser, emailing work from the app.
- Mobile or web versions.

**Anti-goals: things that would make it a failure even if it technically works**
- Big maps that turn into unreadable tangles: lines crossing through boxes, piled-up arrowheads, hairline connectors.
- Needing an internet connection for anything.
- Feeling like dated software, or like a toy for kids.

## 3. Constraints and decided technology

- **Language/UI:** Rust, pure native UI with **egui/eframe** following the pattern of ArtCraft's `artcraft-native` crate (github.com/storytold/artcraft, MIT/Apache-2.0; egui/eframe 0.36 at the time of writing). No webview, no Tauri. (designer's call, based on the owner's "same fashion as ArtCraft"; WordCraft is built the same way.) There is **no reusable ribbon code**: WordCraft's source isn't public, and the ArtCraft repo has no ribbon. ReasonCraft builds its own ribbon, backstage, tabs, command registry and theme. Use `artcraft-native` as a reference for egui setup and conventions only, and the installed WordCraft app and a screenshot of it as the visual reference for the ribbon.
- **Platforms:** Windows, macOS, Linux from one codebase.
- **Licence:** dual MIT / Apache-2.0. (designer's call; owner did not object)
- **Name:** ReasonCraft. **File extension:** `.rcraft`.
- **Repository:** local git, no remote yet.
- Works fully offline.

Library choices beyond egui are the builder's call. Reasonable candidates: `rfd` (file dialogs), `arboard` (image clipboard), `resvg`/`tiny-skia` (raster/SVG rendering), a PDF crate such as `krilla` or `printpdf`, `zip` + `serde_json` (file format), `fluent` (translations), `docx-rs` (Word export), a Hunspell-compatible spellchecker such as `spellbook`, `directories` (settings paths).

## 4. Concepts and document model

One document = one map: **one main tree with one root box, plus a parking area** (owner's decision). The parking area holds loose boxes that aren't connected yet. Typical sources are snippets dropped on empty canvas, half-built fragments, and extra top-level claims from an Argdown import. Loose items can be small connected fragments (e.g. a card with its premises and sources), not just single boxes. Users connect them by dragging onto a box or card in the main tree, or by a "Connect to…" command. The parking area is shown as a clearly separate, collapsible region next to the main tree (exact placement is the builder's call). Loose items are saved in the file and included in the Outline under a "Not connected" heading. Exports leave them out by default, with an option to include them. Checks flag unconnected items as a hint.

People make separate documents for a reasoning map and a grouping map. A map's kind follows from its root's children:
- **Reasoning map:** the root is a Claim with cards under it.
- **Grouping map:** the root is a Claim (the topic/thesis) with category items under it. (designer's call)

**What may sit under what** (designer's call):

| Parent | Allowed children |
|---|---|
| Claim (root) | Reason/Objection cards **or** category items. Not both: the first child sets the kind; the user must convert or remove to switch. Also sources, notes, comments. |
| Premise (in a card) | Reason/Objection cards (targeting that premise), sources, notes, comments |
| Card (as a whole) | Reason/Objection cards targeting the whole card |
| Category item | Category items, notes, comments. No sources, cards or evaluations. |
| Source | Notes, comments |
| Note / comment | Nothing |

Outline and Argdown export handle both kinds. The Essay tab and the evaluation checks apply only to reasoning maps. The empty-category check applies only to grouping maps.

**Boxes**
- **Claim**: the root statement (the contention). One per map.
- **Premise**: a statement inside a card (see below).
- **Category item**: a neutral box in a grouping tree.
- **Source**: evidence attached under a premise (or claim). Has a type, a citation/description and an optional URL, and may carry structured fields (author, title, outlet, date) **(builder's call on fields)**. Types (each with its own icon): Source (generic), My own basis, Assertion, By definition, Case study, Common belief, Data, Event, Example, Expert opinion, Law, Media, Personal experience, Publication, Quote, Statistic, Web.
- **Note**: a sticky note attached to a box, or loose in the parking area.
- Any box can hold an image and a hyperlink. Text is plain with automatic word wrap; box height grows to fit (measured, never clipped). Width has a sensible default and is resizable.

**Cards (argument groups)**
- A **card** holds one or more premises side by side and points at a target. Several premises in one card are **co-premises**.
- Card kinds: **Reason** (supports, green) and **Objection** (opposes, red). An objection whose target lies inside an objection card is shown and named as a **Rebuttal** (rebuts, orange). Rebuttal is derived from the structure, not set by hand. (designer's call; this keeps labels consistent)
- **Targets:** a card can target (a) the claim, (b) **one specific premise** in another card, or (c) **a whole card**, attacking or backing the link itself ("even if these premises are true, they don't support that"). The owner explicitly asked for both (b) and (c).
- **Grouping relation:** a category item's children are joined by neutral lines with no support/oppose meaning. Grouping and reasoning live in the same model. There are **no separate modes** for grouping and reasoning: the map adapts to what you add. Converting between them isn't needed.

**Evaluation** (all optional, shown only when set)
- Claim & premises: Accept / Unsure / Reject.
- Reason & objection cards: Strong / Weak / Nil.
- Sources: Solid / Shaky / Nil.
- Clear evaluation for a box or the whole map.

**Comments** carry feedback, from teachers or anyone else: any box can carry comments (author name + text + optional verdict: positive / needs work / problem). A preset library offers common feedback (e.g. "Great work", "Refinement needed", "Unclear", "Too accepting", "Inconsistent", "Source not appropriate"), so teachers can give feedback fast, but comments also work for peer review and professional use.

**Document-level:** title, author, created/modified dates, document language (used by spellcheck and checks), snippets pane contents, layout direction.

## 5. Look of the map (decided with mockups)

The owner reviewed several mockups and chose these:

- **Box style "D, Subtle tint":** boxes are dark (in dark theme) with a thin border in the role colour. Each card is a rounded, lightly tinted frame in its role colour around its premise(s). A stronger rating gives more tint and a slightly thicker border. The claim is a neutral box with a light border. Sources have no border: icon, bold type name, then the citation in small centred text, hanging under their premise by a thin grey line.
- **Rating label inside the card frame,** top-right (e.g. "●● strongly supports", "● weakly rebuts", "opposes"). Strong = two dots, weak = one dot, unrated = no dots, nil = greyed-out card and a negated label ("does not support"). Accept/Unsure/Reject show as a subtle ✓ / ? / × marker on the box, toggleable in View.
- **Connectors**, user-selectable in View, saved as a personal preference (also used for exports):
  - **Curved** (default): smooth S-curves from the child card's top centre to the parent.
  - **Elbow**: vertical/horizontal segments with rounded corners (org-chart style).
  - **Tapered**: filled connectors that widen toward the parent.
- **Connector rules (all styles):**
  - Supports end in an arrowhead at the parent; opposes and rebuts end in a short flat bar (⊥).
  - Arrowheads have a fixed size (they don't scale with line thickness), and every connector ends in a short straight stub so the arrowhead lines up with its line.
  - Strong = thicker, weak = thinner, but there is a **minimum visible thickness** at every zoom level, so nothing goes hairline.
  - When several connectors attach to the same point, they **spread out along that edge** in left-to-right order of their children instead of stacking.
  - A card targeting **one premise** connects to that premise's bottom edge. A card targeting a **whole card** connects to the target card frame's bottom edge (between the co-premises, or below the single premise).
  - Connectors never run through or behind boxes or over rating labels.
- **Layout:** fully automatic tidy-tree layout with readable spacing. Sources sit compactly directly under their premise. People reorder siblings by dragging; free positioning of boxes isn't offered, which keeps maps tidy. (designer's call) Layout direction: top-down (default), bottom-up or left-to-right, per document.
- Light theme uses the same structure with light boxes. Role colours are tuned per theme for contrast. Exports default to the light theme (print-friendly), with an option to use the current theme.

## 6. Application layout (decided with mockups: "Classic")

```
┌ title bar: quick access (Save, Undo, Redo) · document name · Search commands ┐
├ ribbon tabs: File  Home  Insert  Evaluate  View  Export  Help                ┤
├ ribbon groups for the active tab                                             ┤
├──────────────┬──────────────────────────────────────────┬────────────────────┤
│ Building     │ Map canvas (pan/zoom, overview minimap)  │ Text panel         │
│ panel        │                                          │ Outline · Essay ·  │
│ boxes,       │                                          │ Check · Snippets   │
│ sources,     │                                          │                    │
│ templates,   │                                          │                    │
│ comments     │                                          │                    │
├──────────────┴──────────────────────────────────────────┴────────────────────┤
│ status: box count · check issues · theme toggle · zoom slider                │
└──────────────────────────────────────────────────────────────────────────────┘
```

- Visual language modelled on WordCraft/Word: dark ribbon, labelled groups, large and small buttons, galleries, a **Search commands** box that can run any command.
- Multiple open documents appear as **tabs**, one map per tab.
- Both side panels can be collapsed and resized.

**Ribbon contents.** Grouping and exact placement are the builder's call; the commands below must exist.
- **File (backstage):** New (blank or from template), Open, Recent, Save, Save As, Import Argdown, Export…, Print, Settings, About.
- **Home:** Clipboard (Paste, Cut, Copy, Copy as image). Add (Reason, Objection, Co-premise, Source ▾, Category item, Note, Comment). Box (Change type ▾, Colour ▾, Image, Link, Font size). Editing (Find, Spelling, Delete box, Delete branch).
- **Insert:** Templates gallery (single reason, multiple reasons, pro/con, co-premise argument, grouping tree, plus essay-structure starters), Source types gallery, Image, Note, Comment presets.
- **Evaluate:** Claim/Premise (Accept/Unsure/Reject), Reasons (Strong/Weak/Nil), Objections (Strong/Weak/Nil), Sources (Solid/Shaky/Nil), Clear (box/map), Show evaluation markers.
- **View:** Zoom (fit map, fit selection, 100%), Full screen, Layout direction, Connector style, Overview minimap (on/off/auto), Labels on links, Evaluation shading, Numbering (1, 1.1, 1.2…), panel toggles, Theme (system/light/dark), Language.
- **Export:** Image (PNG, SVG), PDF, Copy as image, Outline (Markdown, plain text, Word .docx), Essay plan (Markdown, .docx), Argdown, Page setup, Print.
- **Help:** Help contents (local, offline), Keyboard shortcuts, Example maps, About. No website links in v1.

**Building panel:** collapsible sections: Boxes (claim, reason, objection, co-premise, category item, note), Sources (all types), Templates, Comments (presets). Drag onto the canvas:
- Drop on a premise box: attach to that premise.
- Drop on a card frame's edge: target the whole card.
- Drop on empty space: create a loose item in the parking area.

**Text panel tabs**
- **Outline:** the map as an indented, optionally numbered text outline, with toggles for numbering, labels ("because", "but", "however"), indentation, colour and notes, plus Copy all. Clicking a line selects its box.
- **Essay:** a writing plan. Each first-level line of reasoning becomes a section, ordered by evaluated strength (strongest first). Each element (claim → reason → evidence → counterargument → rebuttal) gets a paragraph slot next to it where the user can write. A suggested structure: claim in the introduction, strongest argument first, one section per remaining branch, conclusion repeating the claim and first-level arguments. Exports to Markdown/.docx.
- **Check:** list of issues found by course-rule checks (§7). Clicking an issue selects the box.
- **Snippets:** a scratchpad. Paste or type source text and keep it with the document. Select text and drag it onto the map to create a box. If the snippet is tagged with a source (type + citation), that source is attached to the new box automatically. This is the bottom-up workflow.

**Status bar:** box count, number of check issues, theme toggle, zoom slider with percentage.

## 7. Checks (Check panel)

Each check can be switched off in Settings and respects the document language (Dutch and English at launch).
1. **Key-term check:** key terms in a card's target statement that appear in none of its premises (stop words ignored). This finds missing co-premises.
2. **Compound statement:** a box containing "and"/"or" ("en"/"of") may hold two statements.
3. **Not a statement:** a claim or premise phrased as a question.
4. **Premise without support:** a leaf premise with no source and no supporting card.
5. **Empty category:** a grouping category with no children.
6. **Incomplete evaluation:** some items rated, others not (only once evaluation has started).
7. **Inconsistent evaluation:** e.g. a card rated strong while all its premises are rejected, or a "solid" rating on a source whose premise is rejected.
8. **Not connected:** items still in the parking area.

These are hints, never blocking. The builder may add checks.

## 8. Editing and workflow

- **Top-down:** type the claim, then add reasons, objections, co-premises and sources from the ribbon, the building panel, a small "+" menu on the selected box, or the keyboard. Every add/edit/navigate/evaluate action must work from the keyboard. Exact shortcuts are the builder's call; Tab adds a reason to the selection and arrow keys move between boxes.
- **Bottom-up:** collect text in Snippets and drag it out (see §6).
- Typing on a selected box edits its text in place.
- Unlimited undo/redo for all document changes.
- Find (and replace) across box text, sources and notes.
- Spellcheck in the document language (Dutch and English dictionaries bundled).
- **Autosave and crash recovery:** autosave to a recovery location every 30–60 s (builder's call on the interval). After a crash, offer to restore on next start.

## 9. Files, import and export

**`.rcraft` format** (designer's call, details are the builder's call):
- A zip container holding `document.json` (versioned schema with stable UUIDs for every box and card), an `images/` folder and an optional `thumbnail.png`.
- JSON stays human-readable and diff-friendly.
- Unknown fields from newer versions are preserved on save where practical.
- Schema version is checked on open, with a clear message if the file is newer than the app.

**Export**
- **PNG:** chooseable scale/DPI, optional transparent background.
- **SVG:** vector, fonts embedded or text converted to outlines (builder's call).
- **PDF:** vector, page setup: paper size, orientation, fit to one page or tile across pages.
- **Copy as image** to the clipboard.
- Whole map or current selection.
- **Outline / Essay plan:** Markdown, plain text, Word (.docx).
- **Print:** produces the PDF and hands it to the operating system's print/preview. Native cross-platform printing isn't required. (designer's call)

**Argdown import and export** (argdown.org)
- Export the map as Argdown text.
- Import Argdown files, laying them out automatically.
- Map statements, support/attack relations, co-premise arguments (premise–conclusion reconstructions) and attacks on inferences/whole cards. Where the two models differ, import as faithfully as possible and report anything dropped.
- Exact mapping is the builder's call; document it.

## 10. Interface language, theme, accessibility, settings

- **Interface languages:** English and Dutch at launch. All UI text goes in translation files (e.g. Fluent) so others can add languages. Default follows the OS language, switchable in View/Settings.
- **Themes:** system / light / dark. Colours come from theme tokens and meet contrast guidelines (WCAG AA for text).
- **Accessibility:** full keyboard navigation and screen-reader support through egui's AccessKit integration. UI scale / font size setting.
- **Settings** (per user, stored in the OS config directory): theme, language, connector style, checks on/off, autosave interval, default export options, comment author name, "reset to defaults".

## 11. Bundled content

- Example maps written for ReasonCraft (not copied from anywhere), e.g. a reasoning map and a grouping map on a neutral topic, used by Help → Example maps and the templates.
- Offline help pages covering concepts (claim, premise, co-premise, objection, rebuttal, source types, evaluation, the key-term check) and how-tos.
- An original icon set for box and source types and ribbon commands. A permissively licensed set such as Lucide (ISC) may be used as a base (builder's call).

## 12. Distribution

Installable builds for Windows (installer), macOS (.dmg) and Linux (AppImage and/or .deb). The format is the builder's call. Code signing and notarisation are out of scope for now. Files with `.rcraft` open in the app.

## 13. Suggested order of work

The planner may change this, but each step should leave a usable app.
1. **Core editor:** document model, canvas with style-D rendering and curved connectors, automatic layout (top-down), adding/editing/deleting boxes and cards including both target kinds, keyboard navigation, undo/redo, save/open `.rcraft`, dark/light theme, ribbon shell with Home/View, tabs.
2. **Sources and evaluation:** source types, evaluation ratings and their visuals, notes, comments with presets, building panel with drag-and-drop.
3. **Text panel:** Outline, Snippets, Check.
4. **Output:** PNG/SVG/PDF export, copy as image, print, outline/essay exports, Essay tab.
5. **Interchange and polish:** Argdown import/export, Elbow/Tapered connectors, other layout directions, minimap, numbering, find/replace, spellcheck, autosave/recovery, Search commands.
6. **Release readiness:** Dutch translation, accessibility pass, help and example maps, templates, installers for all three platforms.

## 14. Deferred / out of scope ideas noted during design

- Real-time collaboration (keep the format ready: stable IDs).
- Decision maps (options weighed by their pros and cons). Dropped for v1.
- A shuffle exercise (scramble a map for someone to put back together). Dropped for v1, possible later for teachers.
- Opening documents in a separate window. Tabs only for v1.

## 15. Left to the builder

Library choices (within §3), crate structure, exact colours and spacing, icons, keyboard shortcuts, the layout algorithm, the JSON schema, the Argdown mapping details, export rendering approach, installer tooling, check heuristics beyond §7, ribbon group arrangement.
